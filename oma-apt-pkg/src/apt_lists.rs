use std::borrow::Cow;
use std::collections::HashMap;
use std::path::Path;

use deb822_fast::{Deb822, FromDeb822, FromDeb822Paragraph};
use debversion::Version;
use rayon::prelude::*;
use serde::Serialize;

use crate::apt_sources::SourceLookup;
use crate::{DpkgState, extended_states::AptExtendedStates};

/// Errors that can occur when parsing APT list files.
#[derive(Debug, thiserror::Error)]
pub enum AptListsError {
    #[error("Failed to read file: {0}")]
    Io(#[from] std::io::Error),
    #[error("Failed to parse deb822 data in {path}: {detail}")]
    Parse { path: String, detail: String },
}

/// A single package entry from a Packages file
#[derive(Debug, Clone, FromDeb822, Serialize)]
#[cfg_attr(
    feature = "apt-lists",
    derive(rkyv::Archive, rkyv::Serialize, rkyv::Deserialize)
)]
#[serde(rename_all = "PascalCase")]
pub struct PackageEntry {
    pub package: String,
    pub version: Option<String>,
    pub architecture: Option<String>,
    pub description: Option<String>,
    #[deb822(field = "Description-md5")]
    #[serde(rename = "Description-md5")]
    pub description_md5: Option<String>,
    pub maintainer: Option<String>,
    #[deb822(field = "Installed-Size")]
    #[serde(rename = "Installed-Size")]
    pub installed_size: Option<u64>,
    pub depends: Option<String>,
    #[deb822(field = "Pre-Depends")]
    #[serde(rename = "Pre-Depends")]
    pub pre_depends: Option<String>,
    pub recommends: Option<String>,
    pub suggests: Option<String>,
    pub breaks: Option<String>,
    pub conflicts: Option<String>,
    pub replaces: Option<String>,
    pub provides: Option<String>,
    pub section: Option<String>,
    pub priority: Option<String>,
    pub homepage: Option<String>,
    #[deb822(field = "Multi-Arch")]
    #[serde(rename = "Multi-Arch")]
    pub multi_arch: Option<String>,
    pub filename: Option<String>,
    pub size: Option<u64>,
    #[deb822(field = "SHA256")]
    #[serde(rename = "SHA256")]
    pub sha256: Option<String>,
}

impl PackageEntry {
    /// Whether this package is currently installed on the system.
    pub fn is_installed(&self, dpkg: &DpkgState) -> bool {
        dpkg.is_installed(&self.package)
    }

    /// Whether this package was automatically installed as a dependency.
    ///
    /// Requires an `AptExtendedStates` instance (parsed from
    /// `/var/lib/apt/extended_states`); returns `false` if unavailable.
    pub fn is_auto_installed(&self, dpkg: &DpkgState, ext: &AptExtendedStates) -> bool {
        dpkg.is_installed(&self.package) && ext.is_auto_installed(&self.package)
    }

    /// The display full name, `name:arch`, like apt's `Package:` line.
    ///
    /// Mirrors apt's `PkgIterator::FullName(Pretty)`: with `pretty == false`
    /// the `:arch` qualifier is always shown (`foo:amd64`, `foo:all`, ...);
    /// with `pretty == true` it is omitted when the package's architecture
    /// equals `native_arch` or is `all`/`any`/unset — so a native amd64
    /// `apt` shows `apt`, an `Architecture: all` package shows `foo`, and a
    /// foreign `foo:i386` shows `foo:i386`.
    pub fn fullname(&self, pretty: bool, native_arch: &str) -> Cow<'_, str> {
        match self.architecture.as_deref() {
            Some(arch) if !arch.is_empty() => {
                let omit = pretty && (arch == "all" || arch == "any" || arch == native_arch);
                if omit {
                    Cow::Borrowed(&self.package)
                } else {
                    Cow::Owned(format!("{}:{arch}", self.package))
                }
            }
            _ => Cow::Borrowed(&self.package),
        }
    }
}

/// Parse contents of a single Packages file
#[derive(Debug, Clone)]
pub struct PackagesFile {
    /// The source filename (e.g. `archive_dists_sid_main_binary-amd64_Packages`)
    pub source: String,
    pub entries: Vec<PackageEntry>,
}

/// The source an index entry came from, resolved against `sources.list`
/// once at database build time.
///
/// Consumers — download URLs, the `APT-Sources` display, branch matching —
/// read these fields directly; no `sources.list` re-resolution is needed
/// after the database is built.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[cfg_attr(
    feature = "apt-lists",
    derive(rkyv::Archive, rkyv::Serialize, rkyv::Deserialize)
)]
pub struct IndexSource {
    /// Canonical base URL from `sources.list` (e.g.
    /// `https://mirrors.example.com/debian`). For a local `.deb` this is
    /// its `file:` URI.
    pub base_url: String,
    /// The suite (e.g. `stable`), or `local-deb` for local `.deb`s.
    pub suite: String,
    /// The component (e.g. `main`), or `local-deb` for local `.deb`s;
    /// `None` for flat repositories.
    pub component: Option<String>,
    /// The architecture from `binary-<arch>`; `None` for flat repositories.
    pub arch: Option<String>,
}

/// Parse `/var/lib/apt/lists/` and return all package entries paired with
/// the [`IndexSource`] each came from.
///
/// Forward data flow, like apt's cache generation: `lookup` (parsed from
/// `sources.list`) generates the lists filenames of each source's index
/// targets — every component × architecture plus `binary-all`, and the flat
/// `Packages` — and exactly those files are read. Lists files that no
/// configured source produces (e.g. orphans from removed/disabled sources)
/// are never touched.
///
/// `archs` are the architectures to read (see
/// [`AptConfig::architectures`](crate::AptConfig::architectures)).
///
/// The two `Vec`s have the same length and are indexed in parallel.
pub fn parse_apt_lists_dir_with_sources(
    path: impl AsRef<Path>,
    lookup: &SourceLookup,
    archs: &[String],
) -> Result<(Vec<PackageEntry>, Vec<IndexSource>), AptListsError> {
    let dir = path.as_ref();

    // Generate the lists filenames from the source list, then parse each
    // file that exists, pairing every entry with its source and folding
    // into flat parallel vecs.
    lookup
        .index_files(archs)
        .par_iter()
        .map(|(filename, source)| {
            let file = dir.join(filename);
            if !file.is_file() {
                return Ok((Vec::new(), Vec::new()));
            }
            parse_single_packages_file(&file).map(|entries| {
                let sources = vec![source.clone(); entries.len()];
                (entries, sources)
            })
        })
        .try_fold(
            || (Vec::new(), Vec::new()),
            |(mut pkgs, mut srcs), item| {
                let (entries, sources) = item?;
                pkgs.extend(entries);
                srcs.extend(sources);
                Ok((pkgs, srcs))
            },
        )
        .try_reduce(
            || (Vec::new(), Vec::new()),
            |(mut a_pkgs, mut a_srcs), (b_pkgs, b_srcs)| {
                a_pkgs.extend(b_pkgs);
                a_srcs.extend(b_srcs);
                Ok((a_pkgs, a_srcs))
            },
        )
}

/// Parse a single `*_Packages` file (deb822 format).
///
/// Streams the file through [`Deb822::iter_paragraphs_from_reader`], so a
/// large `Packages` file (tens of MB on full mirrors) is never buffered
/// whole — only one paragraph plus the collected [`PackageEntry`]s are in
/// memory at once.
pub fn parse_single_packages_file(
    path: impl AsRef<Path>,
) -> Result<Vec<PackageEntry>, AptListsError> {
    let path = path.as_ref();
    let file = std::fs::File::open(path).map_err(AptListsError::Io)?;

    let entries: Vec<PackageEntry> =
        Deb822::iter_paragraphs_from_reader(std::io::BufReader::new(file))
            .map(|paragraph| {
                let paragraph = paragraph.map_err(|e| AptListsError::Parse {
                    path: path.to_string_lossy().to_string(),
                    detail: e.to_string(),
                })?;

                PackageEntry::from_paragraph(&paragraph).map_err(|e| AptListsError::Parse {
                    path: path.to_string_lossy().to_string(),
                    detail: e,
                })
            })
            .collect::<Result<Vec<_>, _>>()?;

    Ok(entries)
}

/// Build a description cache map (package name → summary) from parsed entries.
pub fn build_description_map(entries: &[PackageEntry]) -> HashMap<String, String> {
    let mut map = HashMap::new();
    for entry in entries {
        if let Some(ref desc) = entry.description {
            // Description field format: first line is the summary
            let summary = desc.lines().next().unwrap_or(desc);
            map.entry(entry.package.clone())
                .or_insert_with(|| summary.to_string());
        }
    }
    map
}

/// One `PackageVersion` per (package, version): a version seen in several
/// mirrors/suites/components is stored once, with `sources` listing each
/// place it appears. This mirrors apt's `pkgCache::Version` + `VerFile`
/// model — a version carries a list of index files instead of duplicating
/// the record once per source.
#[derive(Debug, Clone)]
#[cfg_attr(
    feature = "apt-lists",
    derive(rkyv::Archive, rkyv::Serialize, rkyv::Deserialize)
)]
pub struct PackageVersion {
    /// The version metadata (name, version, description, dependencies, ...).
    pub entry: PackageEntry,
    /// Every source this version is available from, resolved against
    /// `sources.list` at build time. Empty for entries built without source
    /// tracking.
    pub sources: Vec<IndexSource>,
}

impl PackageVersion {
    /// The parsed version. Parsed on demand (a version parse is
    /// microseconds; the `OnceCell` cache was dropped when the database
    /// moved to an immutable memory-mapped archive). Returns `None` when
    /// the version is absent or fails to parse; `None` compares less than
    /// any parsed version.
    pub fn parsed_version(&self) -> Option<Version> {
        self.entry.version.as_deref().and_then(|v| v.parse().ok())
    }
}

/// Common interface for package data sources.
///
/// Both [`AptDb`](crate::AptDb) (eager, cached) and
/// [`AptListsReader`](crate::AptListsReader) (lazy, offset-based) implement
/// this, allowing consumers to switch between them transparently.
///
/// The unit of data is a [`PackageVersion`] — one record per (package,
/// version) carrying every source it is available from — rather than a
/// per-source row, so a version shared by several mirrors is stored once.
///
/// Methods return [`Cow`] so that implementations owning the data can
/// borrow a slice/value, while implementations that parse on demand can
/// return owned data.
pub trait PackageIndex {
    /// Check whether a package name exists.
    fn has_package(&self, name: &str) -> bool;

    /// Return all package names known to this index.
    fn packages(&self) -> Box<dyn Iterator<Item = &str> + '_>;

    /// Return all versions of a package, each with its sources.
    fn get_all(&self, name: &str) -> Cow<'_, [PackageVersion]>;

    /// Return the candidate version of a package — the version a bare
    /// install gets by default.
    ///
    /// Like apt's `pkgPolicy::GetCandidateVer`, this is a *policy* choice
    /// (the highest version) and does not run the resolver.
    fn get_candidate(&self, name: &str) -> Option<Cow<'_, PackageVersion>>;

    /// Return the version for an exact version string, or `None` if that
    /// version is not in the index. The default scans [`Self::get_all`];
    /// implementations with indexed storage may override it.
    fn get_version(&self, name: &str, version: &str) -> Option<Cow<'_, PackageVersion>> {
        // Borrow from the slice when the implementation hands out a borrow,
        // move out when it parses on demand — never clone.
        match self.get_all(name) {
            Cow::Borrowed(slice) => slice
                .iter()
                .find(|v| v.entry.version.as_deref() == Some(version))
                .map(Cow::Borrowed),
            Cow::Owned(vec) => vec
                .into_iter()
                .find(|v| v.entry.version.as_deref() == Some(version))
                .map(Cow::Owned),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_single_paragraph() {
        let input = "\
Package: zoxide
Version: 0.9.6-1
Architecture: amd64
Maintainer: AOSC Maintainers <maintainers@aosc.io>
Description: A smarter cd command for your terminal
Depends: libc6

";
        let deb822: Deb822 = input.parse().unwrap();
        let entry = PackageEntry::from_paragraph(deb822.iter().next().unwrap()).unwrap();

        assert_eq!(entry.package, "zoxide");
        assert_eq!(entry.version.as_deref(), Some("0.9.6-1"));
        assert_eq!(entry.architecture.as_deref(), Some("amd64"));
        assert_eq!(
            entry.description.as_deref(),
            Some("A smarter cd command for your terminal")
        );
        assert_eq!(entry.depends.as_deref(), Some("libc6"));
    }

    #[test]
    fn test_fullname() {
        let parse = |control: &str| {
            let deb822: Deb822 = control.parse().unwrap();
            PackageEntry::from_paragraph(deb822.iter().next().unwrap()).unwrap()
        };

        let native = "amd64";
        // `pretty == true`: native arch → bare name
        assert_eq!(
            parse("Package: apt\nVersion: 1\nArchitecture: amd64\n\n").fullname(true, native),
            "apt"
        );
        // `pretty == true`: `all` → bare name
        assert_eq!(
            parse("Package: foo\nVersion: 1\nArchitecture: all\n\n").fullname(true, native),
            "foo"
        );
        // `pretty == true`: foreign arch → `name:arch`
        assert_eq!(
            parse("Package: foo\nVersion: 1\nArchitecture: i386\n\n").fullname(true, native),
            "foo:i386"
        );
        // `pretty == true`: `any` / unset → bare name
        assert_eq!(
            parse("Package: foo\nVersion: 1\nArchitecture: any\n\n").fullname(true, native),
            "foo"
        );
        assert_eq!(
            parse("Package: foo\nVersion: 1\n\n").fullname(true, native),
            "foo"
        );

        // `pretty == false`: qualifier always shown, even native/`all`/`any`.
        assert_eq!(
            parse("Package: apt\nVersion: 1\nArchitecture: amd64\n\n").fullname(false, native),
            "apt:amd64"
        );
        assert_eq!(
            parse("Package: foo\nVersion: 1\nArchitecture: all\n\n").fullname(false, native),
            "foo:all"
        );
        assert_eq!(
            parse("Package: foo\nVersion: 1\nArchitecture: any\n\n").fullname(false, native),
            "foo:any"
        );
        assert_eq!(
            parse("Package: foo\nVersion: 1\nArchitecture: i386\n\n").fullname(false, native),
            "foo:i386"
        );
        // unset arch has nothing to qualify with
        assert_eq!(
            parse("Package: foo\nVersion: 1\n\n").fullname(false, native),
            "foo"
        );
    }

    #[test]
    fn test_parse_multiple_paragraphs() {
        let input = "\
Package: foo
Version: 1.0
Description: Foo package
Status: install ok installed

Package: bar
Version: 2.0
Description: Bar package
Status: deinstall ok config-files

";
        let deb822: Deb822 = input.parse().unwrap();
        let entries: Vec<PackageEntry> = deb822
            .iter()
            .filter_map(|p| PackageEntry::from_paragraph(p).ok())
            .collect();

        assert_eq!(entries.len(), 2);
        assert_eq!(entries[0].package, "foo");
        assert_eq!(entries[1].package, "bar");
    }

    #[test]
    fn test_build_description_map() {
        let entries = vec![
            PackageEntry {
                package: "foo".into(),
                version: Some("1.0".into()),
                architecture: None,
                description: Some("First line\nSecond line".into()),
                description_md5: None,
                maintainer: None,
                installed_size: None,
                depends: None,
                pre_depends: None,
                recommends: None,
                suggests: None,
                breaks: None,
                conflicts: None,
                replaces: None,
                provides: None,
                section: None,
                priority: None,
                homepage: None,
                multi_arch: None,
                filename: None,
                size: None,
                sha256: None,
            },
            PackageEntry {
                package: "bar".into(),
                version: Some("2.0".into()),
                architecture: None,
                description: Some("Bar description".into()),
                description_md5: None,
                maintainer: None,
                installed_size: None,
                depends: None,
                pre_depends: None,
                recommends: None,
                suggests: None,
                breaks: None,
                conflicts: None,
                replaces: None,
                provides: None,
                section: None,
                priority: None,
                homepage: None,
                multi_arch: None,
                filename: None,
                size: None,
                sha256: None,
            },
        ];

        let map = build_description_map(&entries);
        assert_eq!(map.get("foo").map(|s| s.as_str()), Some("First line"));
        assert_eq!(map.get("bar").map(|s| s.as_str()), Some("Bar description"));
    }

    #[test]
    fn test_parse_all_fields() {
        let input = "\
Package: bash
Version: 5.2.37-1
Architecture: amd64
Section: shells
Priority: optional
Maintainer: AOSC Maintainers <maintainers@aosc.io>
Installed-Size: 2048
Depends: libc6 (>= 2.38), ncurses (>= 6.5)
Pre-Depends: libc6
Recommends: bash-completion
Suggests: bash-doc
Breaks: old-bash (<< 5.0)
Conflicts: bash-pre-v2
Replaces: bash-compat
Provides: sh
Homepage: https://www.gnu.org/software/bash/
Multi-Arch: foreign
Description: GNU Bourne Again SHell
 The standard shell for GNU/Linux systems.
Description-md5: abc123
Filename: pool/main/b/bash/bash_5.2.37-1_amd64.deb
Size: 1234567
SHA256: deadbeef1234567890abcdef01234567890abcdef01234567890abcdef012345678

";
        let deb822: Deb822 = input.parse().unwrap();
        let entry = PackageEntry::from_paragraph(deb822.iter().next().unwrap()).unwrap();

        assert_eq!(entry.package, "bash");
        assert_eq!(entry.version.as_deref(), Some("5.2.37-1"));
        assert_eq!(entry.architecture.as_deref(), Some("amd64"));
        assert_eq!(entry.section.as_deref(), Some("shells"));
        assert_eq!(entry.priority.as_deref(), Some("optional"));
        assert_eq!(
            entry.maintainer.as_deref(),
            Some("AOSC Maintainers <maintainers@aosc.io>")
        );
        assert_eq!(entry.installed_size, Some(2048));
        assert_eq!(
            entry.depends.as_deref(),
            Some("libc6 (>= 2.38), ncurses (>= 6.5)")
        );
        assert_eq!(entry.pre_depends.as_deref(), Some("libc6"));
        assert_eq!(entry.recommends.as_deref(), Some("bash-completion"));
        assert_eq!(entry.suggests.as_deref(), Some("bash-doc"));
        assert_eq!(entry.breaks.as_deref(), Some("old-bash (<< 5.0)"));
        assert_eq!(entry.conflicts.as_deref(), Some("bash-pre-v2"));
        assert_eq!(entry.replaces.as_deref(), Some("bash-compat"));
        assert_eq!(entry.provides.as_deref(), Some("sh"));
        assert_eq!(
            entry.homepage.as_deref(),
            Some("https://www.gnu.org/software/bash/")
        );
        assert_eq!(entry.multi_arch.as_deref(), Some("foreign"));
        assert_eq!(
            entry.description.as_deref(),
            Some("GNU Bourne Again SHell\nThe standard shell for GNU/Linux systems.")
        );
        assert_eq!(
            entry.filename.as_deref(),
            Some("pool/main/b/bash/bash_5.2.37-1_amd64.deb")
        );
        assert_eq!(entry.size, Some(1234567));
        assert_eq!(
            entry.sha256.as_deref(),
            Some("deadbeef1234567890abcdef01234567890abcdef01234567890abcdef012345678")
        );
    }

    #[test]
    fn test_parse_optional_fields_absent() {
        // Minimal paragraph — only required field
        let input = "Package: minimal\n\n";
        let deb822: Deb822 = input.parse().unwrap();
        let entry = PackageEntry::from_paragraph(deb822.iter().next().unwrap()).unwrap();

        assert_eq!(entry.package, "minimal");
        assert!(entry.version.is_none());
        assert!(entry.architecture.is_none());
        assert!(entry.description.is_none());
        assert!(entry.maintainer.is_none());
        assert!(entry.installed_size.is_none());
        assert!(entry.depends.is_none());
        assert!(entry.pre_depends.is_none());
        assert!(entry.recommends.is_none());
        assert!(entry.suggests.is_none());
        assert!(entry.breaks.is_none());
        assert!(entry.conflicts.is_none());
        assert!(entry.replaces.is_none());
        assert!(entry.provides.is_none());
        assert!(entry.section.is_none());
        assert!(entry.priority.is_none());
        assert!(entry.homepage.is_none());
        assert!(entry.multi_arch.is_none());
        assert!(entry.filename.is_none());
        assert!(entry.size.is_none());
        assert!(entry.sha256.is_none());
    }
}
