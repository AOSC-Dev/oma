//! Real-time dpkg status — parsed on every access (fast, single file).

use std::cell::RefCell;
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::sync::OnceLock;

use deb822_fast::{Deb822, FromDeb822Paragraph};

use crate::dpkg::{DpkgPackage, parse_dpkg_status};

/// Parsed dpkg status information.
///
/// [`Self::from_file`] parses the whole file eagerly, for consumers that need
/// every installed package (e.g. search). [`Self::from_file_lazy`] only
/// records the path and answers [`Self::is_installed`] by scanning until the
/// queried package is found, so a single-package consumer like `oma show`
/// never parses the whole status file. Installed-version lookups in lazy
/// mode scan the file once on first use (cached afterwards).
#[derive(Debug, Clone)]
pub struct DpkgState {
    /// Set of installed package names (eager mode).
    pub(crate) installed: HashSet<String>,
    /// Map from installed package name to its version string (eager mode).
    pub(crate) installed_versions: HashMap<String, String>,
    /// Status file path, kept when constructed lazily (`None` in eager mode).
    lazy_path: Option<PathBuf>,
    /// Lazily-answered `is_installed` results (name → installed), filled by
    /// partial scans. Only ever consulted in lazy mode.
    lazy_answers: RefCell<HashMap<String, bool>>,
    /// Installed-version map for lazy mode, filled by a single full scan on
    /// the first version lookup (lazy mode otherwise scans incrementally for
    /// `is_installed` only, and a version cannot be borrowed from a partial
    /// cache).
    lazy_versions: OnceLock<HashMap<String, String>>,
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
    pub fn from_file(path: impl AsRef<Path>) -> Result<Self, crate::error::Error> {
        let dpkg_packages = parse_dpkg_status(path)?;

        let mut installed = HashSet::new();
        for p in &dpkg_packages {
            if p.selection_state().is_installed() {
                installed.insert(p.name.clone());
            }
        }

        Ok(Self {
            installed,
            installed_versions: installed_versions_map(&dpkg_packages),
            lazy_path: None,
            lazy_answers: RefCell::new(HashMap::new()),
            lazy_versions: OnceLock::new(),
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
            lazy_path: Some(path.as_ref().to_path_buf()),
            lazy_answers: RefCell::new(HashMap::new()),
            lazy_versions: OnceLock::new(),
        }
    }

    /// Whether a package is installed.
    pub fn is_installed(&self, name: &str) -> bool {
        if let Some(path) = &self.lazy_path {
            self.is_installed_lazy(name, path)
        } else {
            self.installed.contains(name)
        }
    }

    /// Lazily answer `is_installed` for `name`: scan the status file,
    /// stopping as soon as the package is found (or at EOF), and cache every
    /// package's state seen along the way. A later query for a package not
    /// reached by an earlier early stop re-scans from the start.
    fn is_installed_lazy(&self, name: &str, path: &Path) -> bool {
        let mut answers = self.lazy_answers.borrow_mut();
        if let Some(&installed) = answers.get(name) {
            return installed;
        }

        let file = match std::fs::File::open(path) {
            Ok(f) => f,
            Err(_) => return false,
        };

        for para in Deb822::iter_paragraphs_from_reader(std::io::BufReader::new(file)) {
            let Ok(para) = para else { continue };
            let Ok(pkg) = DpkgPackage::from_paragraph(&para) else {
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
        if let Some(path) = &self.lazy_path {
            let versions = self
                .lazy_versions
                .get_or_init(|| load_installed_versions(path));
            versions.get(name).map(|s| s.as_str())
        } else {
            self.installed_versions.get(name).map(|s| s.as_str())
        }
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
}
