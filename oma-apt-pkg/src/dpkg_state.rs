//! Real-time dpkg status — parsed on every access (fast, single file).

use std::borrow::Cow;
use std::cell::RefCell;
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::sync::OnceLock;

use deb822_fast::{FromDeb822Paragraph, Paragraph};

use crate::dpkg::{DpkgPackage, parse_dpkg_status};

#[cfg(feature = "apt-lists")]
use crate::PackageEntry;
#[cfg(feature = "apt-lists")]
use glob_match::glob_match;

/// Parsed dpkg status information.
///
/// [`Self::from_file`] parses the whole file eagerly, for consumers that need
/// every installed package (e.g. search). [`Self::from_file_lazy`] only
/// records the path and answers [`Self::is_installed`] by scanning until the
/// queried package is found, so a single-package consumer like `oma show`
/// never parses the whole status file. Installed-version lookups in lazy
/// mode scan the file once on first use (cached afterwards). [`Self::entries`]
/// reads the matching packages' full records: from the paragraphs an eager
/// parse already read, or by streaming the file in lazy mode — the file is
/// never read twice.
#[derive(Debug, Clone)]
pub struct DpkgState {
    /// Set of installed package names (eager mode).
    pub(crate) installed: HashSet<String>,
    /// Map from installed package name to its version string (eager mode).
    pub(crate) installed_versions: HashMap<String, String>,
    /// Where the answers come from, and which mode the state is in.
    source: Source,
}

/// Where a [`DpkgState`] answers from.
#[derive(Debug, Clone)]
enum Source {
    /// Parsed at construction: every lookup answers from what that parse read,
    /// and the file is never opened again.
    Eager {
        /// The status file the paragraphs were read from.
        path: PathBuf,
        /// The paragraphs the parse read, kept for [`DpkgState::entries`].
        paragraphs: Vec<Paragraph>,
    },
    /// Read on demand: a lookup scans the file instead of answering from a
    /// parse.
    Lazy {
        /// The status file to scan.
        path: PathBuf,
        /// Lazily-answered `is_installed` results (name → installed), filled
        /// by partial scans.
        answers: RefCell<HashMap<String, bool>>,
        /// Installed-version map, filled by a single full scan on the first
        /// version lookup (the incremental scan above answers only
        /// `is_installed`, and a version cannot be borrowed from a partial
        /// cache).
        versions: OnceLock<HashMap<String, String>>,
    },
}

impl Source {
    /// The status file this state answers from.
    fn path(&self) -> &Path {
        match self {
            Self::Eager { path, .. } | Self::Lazy { path, .. } => path,
        }
    }
}

/// Build the installed name → version map from parsed status packages.
fn installed_versions_map(packages: &[DpkgPackage]) -> HashMap<String, String> {
    packages
        .iter()
        .filter(|p| p.selection_state().is_installed())
        .map(|p| (p.name.clone(), p.version.clone().unwrap_or_default()))
        .collect()
}

/// Full-scan the status file into an installed-version map. Infallible, like
/// lazy mode: an unreadable or unparsable file yields an empty map.
fn load_installed_versions(path: &Path) -> HashMap<String, String> {
    match parse_dpkg_status(path) {
        Ok(packages) => installed_versions_map(&packages),
        Err(_) => HashMap::new(),
    }
}

impl DpkgState {
    /// Parse dpkg status from the given status file path, eagerly.
    ///
    /// The paragraphs are kept, so a later [`Self::entries`] answers from
    /// them instead of reading the file a second time.
    pub fn from_file(path: impl AsRef<Path>) -> Result<Self, crate::error::Error> {
        let path = path.as_ref();

        let mut installed = HashSet::new();
        let mut installed_versions = HashMap::new();
        let mut paragraphs = Vec::new();

        for paragraph in crate::dpkg::paragraphs(path)? {
            let paragraph = paragraph.map_err(crate::dpkg::DpkgError::Deb822)?;
            let pkg = DpkgPackage::from_paragraph(&paragraph).map_err(|e| {
                crate::dpkg::DpkgError::Deb822(deb822_fast::Error::Io(std::io::Error::other(e)))
            })?;

            if pkg.selection_state().is_installed() {
                installed.insert(pkg.name.clone());
                installed_versions
                    .insert(pkg.name.clone(), pkg.version.clone().unwrap_or_default());
            }

            paragraphs.push(paragraph);
        }

        Ok(Self {
            installed,
            installed_versions,
            source: Source::Eager {
                path: path.to_path_buf(),
                paragraphs,
            },
        })
    }

    /// Record the status file path without parsing it; [`Self::is_installed`]
    /// then scans only until the queried package is found, and
    /// [`Self::installed_version`] scans the file once on first use.
    /// Infallible — an unreadable status file simply reports nothing as
    /// installed.
    pub fn from_file_lazy(path: impl AsRef<Path>) -> Self {
        Self {
            installed: HashSet::new(),
            installed_versions: HashMap::new(),
            source: Source::Lazy {
                path: path.as_ref().to_path_buf(),
                answers: RefCell::new(HashMap::new()),
                versions: OnceLock::new(),
            },
        }
    }

    /// The status file this state answers from.
    pub fn path(&self) -> &Path {
        self.source.path()
    }

    /// Whether a package is installed.
    pub fn is_installed(&self, name: &str) -> bool {
        match &self.source {
            Source::Eager { .. } => self.installed.contains(name),
            Source::Lazy { answers, .. } => self.is_installed_lazy(answers, name),
        }
    }

    /// The status file's paragraphs, in file order.
    ///
    /// The ones this state was built from when it was parsed eagerly — a
    /// lookup then never reads the file again — or the file itself, streamed,
    /// when it was not. Empty when the file cannot be read, like the other
    /// lazy lookups: a package that cannot be found is simply not there.
    fn paragraphs(&self) -> Box<dyn Iterator<Item = Cow<'_, Paragraph>> + '_> {
        if let Source::Eager { paragraphs, .. } = &self.source {
            return Box::new(paragraphs.iter().map(Cow::Borrowed));
        }

        match crate::dpkg::paragraphs(self.path()) {
            Ok(paragraphs) => Box::new(paragraphs.filter_map(Result::ok).map(Cow::Owned)),
            Err(_) => Box::new(std::iter::empty()),
        }
    }

    /// Lazily answer `is_installed` for `name`: scan the status file,
    /// stopping as soon as the package is found (or at EOF), and cache every
    /// package's state seen along the way. A later query for a package not
    /// reached by an earlier early stop re-scans from the start.
    fn is_installed_lazy(&self, answers: &RefCell<HashMap<String, bool>>, name: &str) -> bool {
        let mut answers = answers.borrow_mut();
        if let Some(&installed) = answers.get(name) {
            return installed;
        }

        for para in self.paragraphs() {
            let Ok(pkg) = DpkgPackage::from_paragraph(para.as_ref()) else {
                continue;
            };
            let installed = pkg.selection_state().is_installed();
            answers.insert(pkg.name.clone(), installed);
            if pkg.name == name {
                return installed;
            }
        }

        answers.insert(name.to_string(), false);
        false
    }

    /// The installed version of a package, if any.
    ///
    /// In lazy mode this scans the status file once on the first call and
    /// caches the result, so callers get the same answer as eager mode
    /// instead of a silent `None`.
    pub fn installed_version(&self, name: &str) -> Option<&str> {
        match &self.source {
            Source::Lazy { path, versions, .. } => versions
                .get_or_init(|| load_installed_versions(path))
                .get(name)
                .map(String::as_str),
            Source::Eager { .. } => self.installed_versions.get(name).map(String::as_str),
        }
    }

    /// The status entries of the packages matching `patterns`, shaped like
    /// apt index entries.
    ///
    /// A package that is installed but no repository offers it is described
    /// by these: apt shows such a package from the status file itself and
    /// names it as the source (`APT-Sources: /var/lib/dpkg/status`).
    ///
    /// `patterns` are package names or globs, matched the way the package
    /// matcher matches them. Every paragraph a pattern matches is returned —
    /// a package installed for several architectures has one per
    /// architecture — and paragraphs whose current state means dpkg holds no
    /// version of the package (`purge ok not-installed`,
    /// `deinstall ok config-files`) are skipped, as in apt's cache.
    #[cfg(feature = "apt-lists")]
    pub fn entries<'a>(&self, patterns: impl IntoIterator<Item = &'a str>) -> Vec<PackageEntry> {
        let patterns: Vec<&str> = patterns.into_iter().collect();
        if patterns.is_empty() {
            return Vec::new();
        }

        let mut entries = Vec::new();

        for paragraph in self.paragraphs() {
            let Some(name) = paragraph.get("Package") else {
                continue;
            };
            if !patterns.iter().any(|pattern| glob_match(pattern, name)) {
                continue;
            }

            let Ok(pkg) = DpkgPackage::from_paragraph(paragraph.as_ref()) else {
                continue;
            };
            if !pkg.has_version() {
                continue;
            }

            let Ok(entry) = PackageEntry::from_paragraph(paragraph.as_ref()) else {
                continue;
            };

            if let Source::Lazy { answers, .. } = &self.source {
                answers
                    .borrow_mut()
                    .insert(name.to_string(), pkg.selection_state().is_installed());
            }
            entries.push(entry);
        }

        entries
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    const STATUS: &str = "\
Package: bash
Version: 5.2-3
Architecture: amd64
Status: install ok installed

Package: zsh
Version: 5.9-1
Architecture: amd64
Status: hold ok installed

Package: notinstalled
Version: 1.0
Architecture: amd64
Status: deinstall ok config-files

Package: fish
Version: 4.0.0
Architecture: amd64
Status: install ok installed

Package: vendor-tool
Version: 1.0
Architecture: amd64
Status: install ok installed
Section: default
Maintainer: Vendor <vendor@example.invalid>
Installed-Size: 2048
Depends: libc6
Description: 
 vendor synopsis

Package: vendor-tool
Version: 1.0
Architecture: i386
Status: install ok installed
Description: vendor synopsis

Package: half-done
Version: 1.0
Architecture: amd64
Status: install ok unpacked
Description: unpacked but a version
";

    fn write_status() -> tempfile::TempDir {
        let dir = tempfile::tempdir().unwrap();
        let mut f = std::fs::File::create(dir.path().join("status")).unwrap();
        f.write_all(STATUS.as_bytes()).unwrap();
        dir
    }

    #[test]
    fn lazy_is_installed_matches_eager() {
        let dir = write_status();
        let path = dir.path().join("status");

        let eager = DpkgState::from_file(&path).unwrap();
        let lazy = DpkgState::from_file_lazy(&path);

        for name in ["bash", "zsh", "fish", "notinstalled", "nosuchpkg"] {
            assert_eq!(
                lazy.is_installed(name),
                eager.is_installed(name),
                "mismatch for {name}"
            );
        }
    }

    #[test]
    fn lazy_installed_version_matches_eager() {
        let dir = write_status();
        let path = dir.path().join("status");

        let eager = DpkgState::from_file(&path).unwrap();
        let lazy = DpkgState::from_file_lazy(&path);

        // Lazy mode must answer version lookups like eager mode — a silent
        // `None` would make `is_upgradable` report every installed package
        // as not upgradable.
        for name in ["bash", "zsh", "fish", "notinstalled", "nosuchpkg"] {
            assert_eq!(
                lazy.installed_version(name),
                eager.installed_version(name),
                "mismatch for {name}"
            );
        }
    }

    #[test]
    fn eager_state_answers_without_the_file() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("status");
        std::fs::write(&path, STATUS).unwrap();

        let state = DpkgState::from_file(&path).unwrap();

        // The eagerly parsed state keeps what it read: lookups never go back
        // to the file, even once it is gone.
        std::fs::remove_file(&path).unwrap();

        assert!(state.is_installed("bash"));
        assert_eq!(state.installed_version("bash"), Some("5.2-3"));
        assert!(!state.is_installed("absent"));

        #[cfg(feature = "apt-lists")]
        {
            let entries = state.entries(["bash"]);
            assert_eq!(entries.len(), 1);
            assert_eq!(entries[0].package, "bash");
            assert_eq!(entries[0].version.as_deref(), Some("5.2-3"));
        }
    }

    #[test]
    fn path_names_the_status_file() {
        let dir = write_status();
        let path = dir.path().join("status");

        assert_eq!(DpkgState::from_file(&path).unwrap().path(), path);
        assert_eq!(DpkgState::from_file_lazy(&path).path(), path);
    }

    #[cfg(feature = "apt-lists")]
    #[test]
    fn entries_match_between_modes() {
        let dir = write_status();
        let path = dir.path().join("status");

        let eager = DpkgState::from_file(&path).unwrap();
        let lazy = DpkgState::from_file_lazy(&path);

        // Whichever way the state was built, the entries are the same.
        let names = ["bash", "notinstalled", "nosuchpkg", "half-done"];
        let summarize = |entries: Vec<PackageEntry>| {
            entries
                .into_iter()
                .map(|e| (e.package, e.version))
                .collect::<Vec<_>>()
        };

        // `notinstalled` only has its configuration files left, so dpkg
        // holds no version of it; `half-done` is unpacked and does have one.
        let expected = vec![
            ("bash".to_string(), Some("5.2-3".to_string())),
            ("half-done".to_string(), Some("1.0".to_string())),
        ];

        assert_eq!(summarize(eager.entries(names)), expected);
        assert_eq!(summarize(lazy.entries(names)), expected);
    }

    /// The entry carries the fields an apt index entry would, including a
    /// vendor record with an empty synopsis.
    #[cfg(feature = "apt-lists")]
    #[test]
    fn entries_carry_the_fields_an_index_entry_has() {
        let dir = write_status();
        let state = DpkgState::from_file_lazy(dir.path().join("status"));

        // One paragraph per architecture, in file order.
        let entries = state.entries(["vendor-tool", "nosuchpkg"]);
        assert_eq!(entries.len(), 2);

        let entry = &entries[0];
        assert_eq!(entry.package, "vendor-tool");
        assert_eq!(entry.version.as_deref(), Some("1.0"));
        assert_eq!(entry.architecture.as_deref(), Some("amd64"));
        assert_eq!(entry.section.as_deref(), Some("default"));
        assert_eq!(
            entry.maintainer.as_deref(),
            Some("Vendor <vendor@example.invalid>")
        );
        assert_eq!(entry.installed_size, Some(2048));
        assert_eq!(entry.depends.as_deref(), Some("libc6"));
        // A vendor `.deb` may leave the synopsis empty.
        assert_eq!(entry.description.as_deref(), Some("\nvendor synopsis"));
    }

    /// Globs are matched like the package matcher matches them, and every
    /// architecture of a package installed for several comes back.
    #[cfg(feature = "apt-lists")]
    #[test]
    fn entries_match_globs_and_architectures() {
        let dir = write_status();
        let state = DpkgState::from_file_lazy(dir.path().join("status"));

        let found: Vec<_> = state
            .entries(["vendor-*"])
            .into_iter()
            .map(|entry| (entry.version, entry.architecture))
            .collect();

        assert_eq!(
            found,
            vec![
                (Some("1.0".to_string()), Some("amd64".to_string())),
                (Some("1.0".to_string()), Some("i386".to_string())),
            ]
        );

        // A pattern without a wildcard only matches its own name.
        assert!(state.entries(["vendor"]).is_empty());
    }

    /// An `entries` lookup also answers `is_installed` for the packages it
    /// found, without a second scan.
    #[cfg(feature = "apt-lists")]
    #[test]
    fn entries_answer_is_installed() {
        let dir = write_status();
        let state = DpkgState::from_file_lazy(dir.path().join("status"));

        assert_eq!(state.entries(["bash"]).len(), 1);
        assert!(state.is_installed("bash"));

        assert!(state.entries(["nosuchpkg"]).is_empty());
        assert!(!state.is_installed("nosuchpkg"));

        assert_eq!(state.entries(["bash"]).len(), 1);
    }
}
