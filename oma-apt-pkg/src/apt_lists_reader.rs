//! Lazy APT lists reader — builds a sparse byte-offset index over
//! `*_Packages` files and parses individual entries on demand.
//!
//! Unlike [`AptDb`](crate::AptDb) which parses all entries upfront and
//! caches the full struct in binary form, this reader only records where
//! each package's paragraph starts. When queried, it seeks to the offset
//! and parses a single deb822 paragraph. This is useful when only a few
//! packages need to be looked up, saving memory and I/O on the bulk parse.

use std::borrow::Cow;
use std::cell::OnceCell;
use std::collections::HashMap;
use std::io::{BufReader, Seek, SeekFrom};
use std::path::{Path, PathBuf};

use deb822_fast::{Deb822, FromDeb822Paragraph};

use crate::apt_lists::{
    AptListsError, IndexSource, PackageEntry, PackageIndex, PackageVersion,
};
use crate::apt_sources::SourceLookup;

/// A (source, byte offset) pair pointing to a single deb822 paragraph in a
/// `*_Packages` file.
#[derive(Debug, Clone)]
pub struct ListIndexEntry {
    /// APT list filename, e.g.
    /// `mirrors.example.com_debian_dists_bookworm_main_binary-amd64_Packages`
    /// — the key into the file map.
    pub source: String,
    /// The [`IndexSource`] this file was resolved to at build time, or
    /// [`IndexSource::none`] when no lookup was supplied.
    pub index_source: IndexSource,
    /// Byte offset in the file where the paragraph starts.
    pub offset: u64,
}

/// Lazy reader for APT `*_Packages` files.
///
/// Builds a lightweight byte-offset index and parses individual package
/// entries on demand.
///
/// # Example
///
/// ```ignore
/// let reader = AptListsReader::build("/var/lib/apt/lists")?;
/// if reader.has_package("bash") {
///     for entry in reader.get("bash")? {
///         println!("{} {}", entry.package, entry.version.unwrap_or_default());
///     }
/// }
/// ```
pub struct AptListsReader {
    /// Package name → list of (file, offset) entries.
    index: HashMap<String, Vec<ListIndexEntry>>,
    /// Source filename → absolute path.
    file_map: HashMap<String, PathBuf>,
}

impl AptListsReader {
    /// Build a new reader by scanning the given lists directory for
    /// `*_Packages` files and building the offset index, without source
    /// resolution (every entry reports [`IndexSource::none`]).
    pub fn build(lists_dir: impl AsRef<Path>) -> Result<Self, AptListsError> {
        let mut reader = Self {
            index: HashMap::new(),
            file_map: HashMap::new(),
        };
        reader.build_from_dir(lists_dir.as_ref())?;
        Ok(reader)
    }

    /// Build a new reader that scans exactly the lists files the source
    /// list generates (see [`SourceLookup::index_files`]): every component
    /// × architecture plus `binary-all` per source, skipping files that are
    /// not present. Files that no configured source produces are never
    /// scanned, exactly like [`AptDb`](crate::AptDb).
    pub fn build_with_sources(
        lists_dir: impl AsRef<Path>,
        lookup: &SourceLookup,
        archs: &[String],
    ) -> Result<Self, AptListsError> {
        let mut reader = Self {
            index: HashMap::new(),
            file_map: HashMap::new(),
        };
        for (filename, index_source) in lookup.index_files(archs) {
            let path = lists_dir.as_ref().join(&filename);
            if !path.is_file() {
                continue;
            }
            reader.file_map.insert(filename.clone(), path.clone());
            reader.scan_file(&path, &filename, index_source)?;
        }
        Ok(reader)
    }

    fn build_from_dir(&mut self, lists_dir: &Path) -> Result<(), AptListsError> {
        let dir = std::fs::read_dir(lists_dir).map_err(AptListsError::Io)?;

        for entry in dir {
            let entry = entry.map_err(AptListsError::Io)?;
            let name = entry.file_name().to_string_lossy().to_string();
            if !name.ends_with("_Packages") {
                continue;
            }

            let path = entry.path();
            self.file_map.insert(name.clone(), path.clone());
            self.scan_file(&path, &name, IndexSource::none())?;
        }

        Ok(())
    }

    /// Scan a single `*_Packages` file, recording byte offsets of each
    /// paragraph whose first line is `Package: <name>`.
    fn scan_file(
        &mut self,
        path: &Path,
        source: &str,
        index_source: IndexSource,
    ) -> Result<(), AptListsError> {
        let content = std::fs::read_to_string(path).map_err(AptListsError::Io)?;
        let bytes = content.as_bytes();
        let total_len = bytes.len() as u64;
        let mut byte_pos: u64 = 0;
        let mut pending_para = true;

        while byte_pos < total_len {
            let line_end = bytes[byte_pos as usize..]
                .iter()
                .position(|&b| b == b'\n')
                .map(|p| byte_pos + p as u64)
                .unwrap_or(total_len);

            let line = &content[byte_pos as usize..line_end as usize];

            if line.trim().is_empty() {
                pending_para = true;
            } else if pending_para {
                pending_para = false;

                if let Some(suffix) = line.strip_prefix("Package: ") {
                    let pkg_name = suffix.trim().to_string();
                    self.index
                        .entry(pkg_name)
                        .or_default()
                        .push(ListIndexEntry {
                            source: source.to_string(),
                            index_source: index_source.clone(),
                            offset: byte_pos,
                        });
                }
            }

            byte_pos = line_end + 1;
        }

        Ok(())
    }

    /// Return all entries for a package name.
    ///
    /// Opens the relevant `*_Packages` file(s), seeks to each recorded
    /// offset, and parses the single deb822 paragraph into a
    /// [`PackageEntry`].
    pub fn get(&self, name: &str) -> Result<Vec<PackageEntry>, AptListsError> {
        let Some(entries) = self.index.get(name) else {
            return Ok(Vec::new());
        };

        let mut results = Vec::with_capacity(entries.len());
        for entry in entries {
            let pkg = self.parse_at(entry)?;
            results.push(pkg);
        }

        Ok(results)
    }

    fn parse_at(&self, entry: &ListIndexEntry) -> Result<PackageEntry, AptListsError> {
        let path = self.file_map.get(&entry.source).ok_or_else(|| {
            AptListsError::Io(std::io::Error::new(
                std::io::ErrorKind::NotFound,
                format!("source file not cached: {}", entry.source),
            ))
        })?;

        let file = std::fs::File::open(path).map_err(AptListsError::Io)?;
        let mut reader = BufReader::new(file);
        reader
            .seek(SeekFrom::Start(entry.offset))
            .map_err(AptListsError::Io)?;

        let mut para_iter = Deb822::iter_paragraphs_from_reader(reader);
        match para_iter.next() {
            Some(Ok(paragraph)) => {
                PackageEntry::from_paragraph(&paragraph).map_err(|e| AptListsError::Parse {
                    path: entry.source.clone(),
                    detail: e,
                })
            }
            Some(Err(e)) => Err(AptListsError::Parse {
                path: entry.source.clone(),
                detail: e.to_string(),
            }),
            None => Err(AptListsError::Io(std::io::Error::new(
                std::io::ErrorKind::UnexpectedEof,
                format!(
                    "empty paragraph at offset {} in {}",
                    entry.offset, entry.source
                ),
            ))),
        }
    }

    /// Return the raw index entries for a package name.
    pub fn lookup(&self, name: &str) -> Option<&[ListIndexEntry]> {
        self.index.get(name).map(|v| v.as_slice())
    }

    /// Total number of unique package names in the index.
    pub fn len(&self) -> usize {
        self.index.len()
    }

    /// Whether the index is empty.
    pub fn is_empty(&self) -> bool {
        self.index.is_empty()
    }
}

impl PackageIndex for AptListsReader {
    fn has_package(&self, name: &str) -> bool {
        self.index.contains_key(name)
    }

    fn packages(&self) -> Box<dyn Iterator<Item = &str> + '_> {
        Box::new(self.index.keys().map(|s| s.as_str()))
    }

    fn get_all(&self, name: &str) -> Cow<'_, [PackageVersion]> {
        // Parsing is best-effort: malformed paragraphs are skipped; the same
        // version from several source files is merged into one version with
        // every source listed.
        let mut versions: Vec<PackageVersion> = Vec::new();
        for entry in self.index.get(name).into_iter().flatten() {
            let Ok(pkg) = self.parse_at(entry) else {
                continue;
            };
            if let Some(existing) = versions.iter_mut().find(|v| v.entry.version == pkg.version) {
                if !existing.sources.contains(&entry.index_source) {
                    existing.sources.push(entry.index_source.clone());
                }
            } else {
                versions.push(PackageVersion {
                    entry: pkg,
                    sources: vec![entry.index_source.clone()],
                    deps: OnceCell::new(),
                    parsed_version: OnceCell::new(),
                });
            }
        }
        Cow::Owned(versions)
    }

    fn get_candidate(&self, name: &str) -> Option<Cow<'_, PackageVersion>> {
        self.get_all(name)
            .iter()
            .max_by_key(|v| v.parsed_version())
            .cloned()
            .map(Cow::Owned)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::path::Path;

    fn write_packages(dir: &Path, filename: &str, content: &str) {
        fs::write(dir.join(filename), content).unwrap();
    }

    fn packages_dir() -> (tempfile::TempDir, PathBuf) {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().to_path_buf();
        (dir, path)
    }

    #[test]
    fn test_basic_lookup() {
        let (_d, dir) = packages_dir();
        write_packages(
            &dir,
            "test_main_binary-amd64_Packages",
            r#"Package: bash
Version: 5.2-3
Architecture: amd64
Depends: libc6
Description: GNU Bourne Again SHell
 shell

Package: zsh
Version: 5.9-1
Architecture: amd64
Description: Z shell
 shell

"#,
        );

        let reader = AptListsReader::build(&dir).unwrap();
        assert!(reader.has_package("bash"));
        let entries = reader.get("bash").unwrap();
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].package, "bash");
        assert_eq!(entries[0].version.as_deref(), Some("5.2-3"));
        assert_eq!(entries[0].depends.as_deref(), Some("libc6"));
    }

    #[test]
    fn test_not_found() {
        let (_d, dir) = packages_dir();
        write_packages(
            &dir,
            "test_main_binary-amd64_Packages",
            r#"Package: bash
Version: 5.2-3

"#,
        );

        let reader = AptListsReader::build(&dir).unwrap();
        assert!(!reader.has_package("nonexistent"));
        assert!(reader.get("nonexistent").unwrap().is_empty());
    }

    #[test]
    fn test_multiple_entries_same_package() {
        let (_d, dir) = packages_dir();
        write_packages(
            &dir,
            "test_main_binary-amd64_Packages",
            r#"Package: bash
Version: 5.2-3

Package: bash
Version: 5.1-1

"#,
        );

        let reader = AptListsReader::build(&dir).unwrap();
        let entries = reader.get("bash").unwrap();
        assert_eq!(entries.len(), 2);
        // Order matches file order
        assert_eq!(entries[0].version.as_deref(), Some("5.2-3"));
        assert_eq!(entries[1].version.as_deref(), Some("5.1-1"));
    }

    #[test]
    fn test_multiple_packages() {
        let (_d, dir) = packages_dir();
        write_packages(
            &dir,
            "test_main_binary-amd64_Packages",
            r#"Package: bash
Version: 5.2-3

Package: zsh
Version: 5.9-1

"#,
        );

        let reader = AptListsReader::build(&dir).unwrap();
        assert_eq!(reader.len(), 2);
        assert!(reader.has_package("bash"));
        assert!(reader.has_package("zsh"));
    }

    #[test]
    fn test_multiple_source_files() {
        let (_d, dir) = packages_dir();
        write_packages(
            &dir,
            "repo1_main_binary-amd64_Packages",
            r#"Package: bash
Version: 5.2-3

"#,
        );
        write_packages(
            &dir,
            "repo2_main_binary-amd64_Packages",
            r#"Package: bash
Version: 5.2-3+b1

"#,
        );

        let reader = AptListsReader::build(&dir).unwrap();
        let entries = reader.get("bash").unwrap();
        assert_eq!(entries.len(), 2);
    }

    #[test]
    fn test_non_packages_file_ignored() {
        let (_d, dir) = packages_dir();
        write_packages(
            &dir,
            "some_random_file",
            r#"Package: bash
Version: 5.2-3

"#,
        );
        write_packages(
            &dir,
            "valid_main_binary-amd64_Packages",
            r#"Package: zsh
Version: 5.9-1

"#,
        );

        let reader = AptListsReader::build(&dir).unwrap();
        assert!(!reader.has_package("bash"));
        assert!(reader.has_package("zsh"));
    }

    #[test]
    fn test_blank_lines_between_paragraphs() {
        let (_d, dir) = packages_dir();
        // Multiple blank lines between paragraphs
        write_packages(
            &dir,
            "test_main_binary-amd64_Packages",
            r#"Package: bash
Version: 5.2-3



Package: zsh
Version: 5.9-1

"#,
        );

        let reader = AptListsReader::build(&dir).unwrap();
        assert!(reader.has_package("bash"));
        assert!(reader.has_package("zsh"));
    }

    #[test]
    fn test_empty_directory() {
        let (_d, dir) = packages_dir();
        let reader = AptListsReader::build(&dir).unwrap();
        assert!(reader.is_empty());
    }

    #[test]
    fn test_entry_fields_preserved() {
        let (_d, dir) = packages_dir();
        write_packages(
            &dir,
            "test_main_binary-amd64_Packages",
            r#"Package: apt
Version: 2.7.0
Architecture: amd64
Maintainer: APT Development Team <deity@lists.debian.org>
Installed-Size: 4096
Depends: libc6, libstdc++6
Homepage: https://wiki.debian.org/Apt
Section: admin
Priority: important
Description: commandline package manager
 This is the main APT tool.

"#,
        );

        let reader = AptListsReader::build(&dir).unwrap();
        let entries = reader.get("apt").unwrap();
        assert_eq!(entries.len(), 1);
        let e = &entries[0];
        assert_eq!(e.package, "apt");
        assert_eq!(e.version.as_deref(), Some("2.7.0"));
        assert_eq!(e.architecture.as_deref(), Some("amd64"));
        assert_eq!(
            e.maintainer.as_deref(),
            Some("APT Development Team <deity@lists.debian.org>")
        );
        assert_eq!(e.installed_size, Some(4096));
        assert_eq!(e.depends.as_deref(), Some("libc6, libstdc++6"));
        assert_eq!(e.homepage.as_deref(), Some("https://wiki.debian.org/Apt"));
        assert_eq!(e.section.as_deref(), Some("admin"));
        assert_eq!(e.priority.as_deref(), Some("important"));
    }
}
