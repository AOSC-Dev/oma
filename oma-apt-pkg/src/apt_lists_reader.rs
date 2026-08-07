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
use std::fs::File;
use std::io::{BufRead, BufReader, Seek, SeekFrom};
use std::path::{Path, PathBuf};
use std::sync::Arc;

use deb822_fast::{Deb822, FromDeb822Paragraph};
use rayon::prelude::*;

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
    /// [`IndexSource::none`] when no lookup was supplied. Shared via `Arc`
    /// across every entry of the file, so the index build clones it once
    /// per file instead of once per paragraph.
    pub index_source: Arc<IndexSource>,
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
/// let cfg = AptConfig::new();
/// let lookup = SourceLookup::build(&cfg);
/// let archs = cfg.architectures();
/// let reader = AptListsReader::build_with_sources("/var/lib/apt/lists", &lookup, &archs)?;
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
    /// Build a new reader that scans exactly the lists files the source
    /// list generates (see [`SourceLookup::index_files`]): every component
    /// × architecture plus `binary-all` per source, skipping files that are
    /// not present. Files that no configured source produces are never
    /// scanned, exactly like [`AptDb`](crate::AptDb).
    ///
    /// The files are scanned in parallel with rayon; each file's
    /// per-package entries are merged into one index afterwards.
    pub fn build_with_sources(
        lists_dir: impl AsRef<Path>,
        lookup: &SourceLookup,
        archs: &[String],
    ) -> Result<Self, AptListsError> {
        let lists_dir = lists_dir.as_ref();
        // Collect the existing lists files (with the path each resolves to)
        // up front, so rayon can work on a plain slice.
        let files: Vec<(String, IndexSource, PathBuf)> = lookup
            .index_files(archs)
            .into_iter()
            .filter_map(|(filename, index_source)| {
                let path = lists_dir.join(&filename);
                path.is_file().then_some((filename, index_source, path))
            })
            .collect();

        let file_map: HashMap<String, PathBuf> = files
            .iter()
            .map(|(filename, _, path)| (filename.clone(), path.clone()))
            .collect();

        // Scan each file in parallel, merging every file's per-package
        // entry lists into the shared index.
        let index: HashMap<String, Vec<ListIndexEntry>> = files
            .par_iter()
            .map(|(filename, index_source, path)| {
                Self::scan_file(path, filename, index_source.clone())
            })
            .try_reduce(HashMap::new, |mut acc, entries| {
                for (pkg, list) in entries {
                    acc.entry(pkg).or_default().extend(list);
                }
                Ok(acc)
            })?;

        Ok(Self { index, file_map })
    }

    /// Scan a single `*_Packages` file, returning the per-package entry
    /// offsets found (`package name → entries`).
    ///
    /// Streams the file with a [`BufReader`] one line at a time (reusing a
    /// single buffer, growing only to the longest line) instead of loading
    /// it all, recording the exact byte offset of every `Package:` line so
    /// [`parse_at`](Self::parse_at) can seek straight to a paragraph later.
    ///
    /// The result is a per-file map, so files can be scanned in parallel
    /// and merged afterwards (see [`build_with_sources`](Self::build_with_sources)).
    fn scan_file(
        path: &Path,
        source: &str,
        index_source: IndexSource,
    ) -> Result<HashMap<String, Vec<ListIndexEntry>>, AptListsError> {
        let file = File::open(path).map_err(AptListsError::Io)?;
        let mut reader = BufReader::new(file);
        // Shared by every entry of this file: entries only bump the
        // refcount instead of deep-copying the whole `IndexSource`.
        let index_source = Arc::new(index_source);
        // Reused per line, so memory stays bounded by the longest line.
        let mut line = Vec::new();
        // Byte offset of the line about to be read.
        let mut byte_pos: u64 = 0;
        let mut pending_para = true;
        let mut index: HashMap<String, Vec<ListIndexEntry>> = HashMap::new();

        loop {
            line.clear();
            let n = reader
                .read_until(b'\n', &mut line)
                .map_err(AptListsError::Io)?;
            if n == 0 {
                break;
            }

            // Strip the trailing newline for inspection; `byte_pos` still
            // points at the start of this line — the paragraph start when
            // this is a `Package:` line.
            let content = match line.last() {
                Some(b'\n') => &line[..line.len() - 1],
                _ => line.as_slice(),
            };

            if content.iter().all(u8::is_ascii_whitespace) {
                pending_para = true;
            } else if pending_para {
                pending_para = false;
                if let Some(suffix) = content.strip_prefix(b"Package: ") {
                    let pkg_name = String::from_utf8_lossy(suffix).trim().to_string();
                    index.entry(pkg_name).or_default().push(ListIndexEntry {
                        source: source.to_string(),
                        index_source: Arc::clone(&index_source),
                        offset: byte_pos,
                    });
                }
            }

            byte_pos += n as u64;
        }

        Ok(index)
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
                if !existing.sources.contains(&*entry.index_source) {
                    existing.sources.push((*entry.index_source).clone());
                }
            } else {
                versions.push(PackageVersion {
                    entry: pkg,
                    sources: vec![(*entry.index_source).clone()],
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

    /// A `stable/main` deb822 source, used by [`build_reader`] and friends.
    const STABLE_MAIN: &str = "Types: deb\n\
        URIs: https://example.com/debs\n\
        Suites: stable\n\
        Components: main\n\
        Signed-By: /dev/null\n";

    /// Build a `SourceLookup` from a single deb822 `.sources` file.
    fn lookup_from(text: &str) -> SourceLookup {
        let dir = tempfile::tempdir().unwrap();
        let list = dir.path().join("test.sources");
        std::fs::write(&list, text).unwrap();
        SourceLookup::from_paths(&[list], |_| {})
    }

    /// The `binary-amd64` lists filename the given lookup generates.
    fn amd64_file(lookup: &SourceLookup, archs: &[String]) -> String {
        lookup
            .index_files(archs)
            .into_iter()
            .find(|(_, src)| src.arch.as_deref() == Some("amd64"))
            .unwrap()
            .0
    }

    /// Build a reader over one `stable/main` deb822 source (arch `amd64`),
    /// writing `content` to the `binary-amd64` Packages file that source
    /// generates. The temp dir is returned alongside so the files stay
    /// alive for the reader's lazy parsing.
    fn build_reader(content: &str) -> (tempfile::TempDir, AptListsReader) {
        let (dir_handle, dir) = packages_dir();
        let lookup = lookup_from(STABLE_MAIN);
        let archs = vec!["amd64".to_string()];
        let name = amd64_file(&lookup, &archs);
        write_packages(&dir, &name, content);
        let reader = AptListsReader::build_with_sources(&dir, &lookup, &archs).unwrap();
        (dir_handle, reader)
    }

    #[test]
    fn test_basic_lookup() {
        let (_d, reader) = build_reader(
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
        assert!(reader.has_package("bash"));
        let entries = reader.get("bash").unwrap();
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].package, "bash");
        assert_eq!(entries[0].version.as_deref(), Some("5.2-3"));
        assert_eq!(entries[0].depends.as_deref(), Some("libc6"));
    }

    #[test]
    fn test_not_found() {
        let (_d, reader) = build_reader("Package: bash\nVersion: 5.2-3\n\n");
        assert!(!reader.has_package("nonexistent"));
        assert!(reader.get("nonexistent").unwrap().is_empty());
    }

    #[test]
    fn test_multiple_entries_same_package() {
        let (_d, reader) = build_reader(
            r#"Package: bash
Version: 5.2-3

Package: bash
Version: 5.1-1

"#,
        );
        let entries = reader.get("bash").unwrap();
        assert_eq!(entries.len(), 2);
        // Order matches file order
        assert_eq!(entries[0].version.as_deref(), Some("5.2-3"));
        assert_eq!(entries[1].version.as_deref(), Some("5.1-1"));
    }

    #[test]
    fn test_multiple_packages() {
        let (_d, reader) = build_reader(
            "Package: bash\nVersion: 5.2-3\n\nPackage: zsh\nVersion: 5.9-1\n\n",
        );
        assert_eq!(reader.len(), 2);
        assert!(reader.has_package("bash"));
        assert!(reader.has_package("zsh"));
    }

    #[test]
    fn test_multiple_source_files() {
        let (_d, dir) = packages_dir();
        // Two suites of the same repo: `stable` and `preview`.
        let lookup = lookup_from(
            "Types: deb\n\
             URIs: https://example.com/debs\n\
             Suites: stable\n\
             Components: main\n\
             Signed-By: /dev/null\n\
             \n\
             Types: deb\n\
             URIs: https://example.com/debs\n\
             Suites: preview\n\
             Components: main\n\
             Signed-By: /dev/null\n",
        );
        let archs = vec!["amd64".to_string()];
        for (suite, content) in [
            ("stable", "Package: bash\nVersion: 5.2-3\n\n"),
            ("preview", "Package: bash\nVersion: 5.2-3+b1\n\n"),
        ] {
            let name = lookup
                .index_files(&archs)
                .into_iter()
                .find(|(_, src)| src.suite == suite && src.arch.as_deref() == Some("amd64"))
                .unwrap()
                .0;
            write_packages(&dir, &name, content);
        }

        let reader = AptListsReader::build_with_sources(&dir, &lookup, &archs).unwrap();
        let entries = reader.get("bash").unwrap();
        assert_eq!(entries.len(), 2);
    }

    #[test]
    fn test_non_packages_file_ignored() {
        let (_d, dir) = packages_dir();
        let lookup = lookup_from(STABLE_MAIN);
        let archs = vec!["amd64".to_string()];
        let name = amd64_file(&lookup, &archs);
        // Only the lists files the source generates are ever scanned.
        write_packages(&dir, "some_random_file", "Package: bash\nVersion: 5.2-3\n\n");
        write_packages(&dir, &name, "Package: zsh\nVersion: 5.9-1\n\n");

        let reader = AptListsReader::build_with_sources(&dir, &lookup, &archs).unwrap();
        assert!(!reader.has_package("bash"));
        assert!(reader.has_package("zsh"));
    }

    #[test]
    fn test_blank_lines_between_paragraphs() {
        let (_d, reader) = build_reader(
            r#"Package: bash
Version: 5.2-3



Package: zsh
Version: 5.9-1

"#,
        );
        assert!(reader.has_package("bash"));
        assert!(reader.has_package("zsh"));
    }

    #[test]
    fn test_empty_directory() {
        let (_d, dir) = packages_dir();
        let lookup = lookup_from(STABLE_MAIN);
        let archs = vec!["amd64".to_string()];
        // No lists files present: the reader scans nothing.
        let reader = AptListsReader::build_with_sources(&dir, &lookup, &archs).unwrap();
        assert!(reader.is_empty());
    }

    #[test]
    fn test_entry_fields_preserved() {
        let (_d, reader) = build_reader(
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
