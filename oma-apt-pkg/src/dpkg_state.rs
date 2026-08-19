//! Real-time dpkg status — parsed on every access (fast, single file).
//!
//! The status file is parsed once into a lossless deb822 tree, which is
//! kept for writing back: [`DpkgState::mark_held`] sets `Status: hold` on the
//! held package's paragraph in place (the lossless tree is mutable —
//! `tree[pkg].status = …`) and [`DpkgState::to_file`] writes the tree back —
//! so holds survive restarts like `apt-mark hold`, without a separate state
//! file. The query sets (installed, versions, holds, …) are derived from the
//! tree lazily, on first use.

use std::cell::{OnceCell, RefCell};
use std::collections::{HashMap, HashSet};
use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf};

use deb822_fast::FromDeb822Paragraph;
use deb822_lossless::Deb822;

use crate::dpkg::{DpkgPackage, SelectionState, read_status_tree};

/// The query state derived from the status tree, built once on first access
/// so [`DpkgState::from_file`] stays a cheap parse of the tree alone.
#[derive(Debug, Clone, Default)]
pub(crate) struct DpkgIndex {
    installed: HashSet<String>,
    installed_versions: HashMap<String, String>,
    needs_reinstall: HashSet<String>,
    essential: HashSet<String>,
    protected: HashSet<String>,
    held: HashSet<String>,
}

impl DpkgIndex {
    fn from_tree(tree: &Deb822) -> Self {
        let mut index = Self::default();
        // Best-effort: a paragraph that does not parse as a package entry is
        // skipped, like the crate's other deb822 readers.
        let Ok(packages) = crate::dpkg::packages_from_tree(tree) else {
            return index;
        };
        for p in &packages {
            if p.selection_state().is_installed() {
                index.installed.insert(p.name.clone());
                index
                    .installed_versions
                    .insert(p.name.clone(), p.version.clone().unwrap_or_default());
                if p.needs_reinstall() {
                    index.needs_reinstall.insert(p.name.clone());
                }
                if p.essential == Some(true) {
                    index.essential.insert(p.name.clone());
                }
                if p.protected == Some(true) {
                    index.protected.insert(p.name.clone());
                }
                if p.selection_state() == SelectionState::Hold {
                    index.held.insert(p.name.clone());
                }
            }
        }
        index
    }

    /// Record an installed package at `version`, optionally essential/held.
    /// For callers that build the index directly instead of from a status
    /// tree — the EDSP resolver synthesises its installed state from the
    /// universe, which has no `/var/lib/dpkg/status` file.
    #[cfg(feature = "resolver")]
    pub(crate) fn add_installed(
        &mut self,
        name: String,
        version: String,
        essential: bool,
        held: bool,
    ) {
        self.installed.insert(name.clone());
        self.installed_versions.insert(name.clone(), version);
        if essential {
            self.essential.insert(name.clone());
        }
        if held {
            self.held.insert(name);
        }
    }
}

/// Parsed dpkg status information.
///
/// Always parsed from `/var/lib/dpkg/status` — the lossless tree is the
/// single source of truth; the query sets are derived lazily.
#[derive(Debug, Clone)]
pub struct DpkgState {
    /// The lossless dpkg status tree, kept so [`Self::mark_held`] can set
    /// holds on it and [`Self::to_file`] writes it back without re-reading.
    /// Empty in lazy mode ([`Self::from_file_lazy`]).
    pub(crate) tree: Deb822,
    /// Lazily-built query index, so `from_file` only parses the tree.
    index: OnceCell<DpkgIndex>,
    /// Status file path, kept when constructed lazily (`None` in eager
    /// mode). [`Self::is_installed`] then scans the file only until the
    /// queried package is found.
    lazy_path: Option<PathBuf>,
    /// Lazily-answered `is_installed` results (name → installed), filled by
    /// partial scans. Only consulted in lazy mode.
    lazy_answers: RefCell<HashMap<String, bool>>,
}

impl DpkgState {
    /// Parse dpkg status from the given status file path. The file is read
    /// once into a lossless tree (kept, so [`Self::mark_held`] can write
    /// holds and [`Self::to_file`] can write it back); the query sets are
    /// derived lazily on first use.
    pub fn from_file(path: impl AsRef<Path>) -> Result<Self, crate::error::Error> {
        Ok(Self::from_tree(read_status_tree(path)?))
    }

    /// Build a state that answers [`Self::is_installed`] by scanning the
    /// status file only until the queried package is found, instead of
    /// parsing every installed package up front — a single-package consumer
    /// like `oma show` never parses the whole status file. Infallible: an
    /// unreadable status file simply reports nothing as installed. The other
    /// queries (versions, holds, ...) are unsupported on a lazy state (no
    /// tree is loaded); only `is_installed` is meaningful.
    pub fn from_file_lazy(path: impl AsRef<Path>) -> Self {
        Self {
            tree: Deb822::default(),
            index: OnceCell::new(),
            lazy_path: Some(path.as_ref().to_path_buf()),
            lazy_answers: RefCell::new(HashMap::new()),
        }
    }

    /// Build a state from an already-parsed status tree.
    pub(crate) fn from_tree(tree: Deb822) -> Self {
        Self {
            tree,
            index: OnceCell::new(),
            lazy_path: None,
            lazy_answers: RefCell::new(HashMap::new()),
        }
    }

    /// Build a state from a pre-computed index, without a dpkg status tree —
    /// used by the EDSP resolver, whose installed state comes from the
    /// universe rather than `/var/lib/dpkg/status`. The `mark_*` / `to_file`
    /// methods are unusable on such a state (there is no tree to write
    /// back).
    #[cfg(feature = "resolver")]
    pub(crate) fn from_index(index: DpkgIndex) -> Self {
        Self {
            tree: Deb822::default(),
            index: OnceCell::from(index),
            lazy_path: None,
            lazy_answers: RefCell::new(HashMap::new()),
        }
    }

    /// The lazily-built query index.
    fn index(&self) -> &DpkgIndex {
        self.index.get_or_init(|| DpkgIndex::from_tree(&self.tree))
    }

    /// The query index, ensuring it is built so it can be updated alongside
    /// the tree (used by the `mark_*` methods).
    fn index_mut(&mut self) -> &mut DpkgIndex {
        if self.index.get().is_none() {
            let index = DpkgIndex::from_tree(&self.tree);
            let _ = self.index.set(index);
        }
        self.index.get_mut().expect("initialized above")
    }

    /// Write the loaded status tree back to `path` — holds set by
    /// [`Self::mark_held`] are already in the tree, so this is a plain
    /// lossless write (everything else is preserved). The counterpart of
    /// [`Self::from_file`], like `apt-mark hold`.
    pub fn to_file(&self, path: impl AsRef<Path>) -> Result<(), crate::dpkg::DpkgError> {
        std::fs::write(path, self.tree.to_string())?;
        Ok(())
    }

    /// Whether a package is installed. In lazy mode ([`Self::from_file_lazy`])
    /// this scans the status file only until `name` is found.
    pub fn is_installed(&self, name: &str) -> bool {
        if let Some(path) = &self.lazy_path {
            self.is_installed_lazy(name, path)
        } else {
            self.index().installed.contains(name)
        }
    }

    /// Lazily answer `is_installed` for `name`: scan the status file,
    /// stopping as soon as the package is found (or at EOF), and cache every
    /// package's state seen along the way. A later query for a package not
    /// reached by an earlier early stop re-scans from the start. (This
    /// deb822_lossless release has no reader-based iterator, so paragraphs
    /// are gathered line-by-line and parsed one at a time.)
    fn is_installed_lazy(&self, name: &str, path: &Path) -> bool {
        let mut answers = self.lazy_answers.borrow_mut();
        if let Some(&installed) = answers.get(name) {
            return installed;
        }

        let file = match std::fs::File::open(path) {
            Ok(f) => f,
            Err(_) => return false,
        };

        let mut reader = BufReader::new(file);
        let mut paragraph = String::new();
        let mut line = String::new();

        loop {
            line.clear();
            match reader.read_line(&mut line) {
                Ok(0) => {
                    // EOF — flush a trailing paragraph with no blank line.
                    if !paragraph.is_empty()
                        && let Some(installed) = scan_paragraph(&paragraph, name, &mut answers)
                    {
                        return installed;
                    }
                    break;
                }
                Ok(_) => {
                    if line.trim().is_empty() {
                        if !paragraph.is_empty() {
                            if let Some(installed) = scan_paragraph(&paragraph, name, &mut answers)
                            {
                                return installed;
                            }
                            paragraph.clear();
                        }
                    } else {
                        paragraph.push_str(&line);
                    }
                }
                Err(_) => return false,
            }
        }

        answers.insert(name.to_string(), false);
        false
    }

    /// The installed version of a package, if any.
    pub fn installed_version(&self, name: &str) -> Option<&str> {
        self.index()
            .installed_versions
            .get(name)
            .map(|s| s.as_str())
    }

    /// Iterate over the names of all installed packages.
    pub fn installed_packages(&self) -> impl Iterator<Item = &str> {
        self.index().installed.iter().map(String::as_str)
    }

    /// Whether the installed package is in a state that needs reinstalling
    /// (e.g. flagged `reinstreq`, or half-installed / unpacked / etc.).
    pub fn needs_reinstall(&self, name: &str) -> bool {
        self.index().needs_reinstall.contains(name)
    }

    /// Whether an installed package is essential (marked `Essential: yes` in
    /// dpkg status) — apt refuses to remove such packages.
    pub fn is_essential(&self, name: &str) -> bool {
        self.index().essential.contains(name)
    }

    /// Whether an installed package is protected (marked `Protected: yes` in
    /// dpkg status) — like essential, apt refuses to remove such packages.
    pub fn is_protected(&self, name: &str) -> bool {
        self.index().protected.contains(name)
    }

    /// Whether an installed package is held (`Status: hold ...` in dpkg
    /// status) — apt refuses to change such packages unless the user asks
    /// explicitly.
    pub fn is_held(&self, name: &str) -> bool {
        self.index().held.contains(name)
    }

    /// Mark `name` as held: it is then protected from autoremove and
    /// closure-driven removal, and `Status: hold` is set on its paragraph in
    /// the loaded status tree — like `tree[pkg].status.selection = Hold` — so
    /// [`Self::to_file`] writes it back without re-reading or re-setting.
    ///
    /// Like `apt-mark hold`: refuses packages that are not installed, and is
    /// a no-op when the package is already held.
    pub fn mark_held(
        &mut self,
        name: impl Into<String>,
    ) -> Result<&mut Self, crate::dpkg::DpkgError> {
        let name = name.into();
        if !self.is_installed(&name) {
            return Err(crate::dpkg::DpkgError::NotInstalled(name));
        }
        if self.is_held(&name) {
            return Ok(self);
        }
        self.index_mut().held.insert(name.clone());
        crate::dpkg::set_pkg_status(&mut self.tree, &name, SelectionState::Hold)?;
        Ok(self)
    }

    /// Remove the hold from `name` — it is no longer protected from
    /// autoremove, and `Status: install` is set on its paragraph in the
    /// loaded status tree, so [`Self::to_file`] writes it back. The
    /// counterpart of [`Self::mark_held`], like `apt-mark unhold`.
    ///
    /// Like `apt-mark unhold`: refuses packages that are not installed, and
    /// is a no-op when the package is not held.
    pub fn mark_unheld(
        &mut self,
        name: impl Into<String>,
    ) -> Result<&mut Self, crate::dpkg::DpkgError> {
        let name = name.into();
        if !self.is_installed(&name) {
            return Err(crate::dpkg::DpkgError::NotInstalled(name));
        }
        if !self.is_held(&name) {
            return Ok(self);
        }
        self.index_mut().held.remove(&name);
        crate::dpkg::set_pkg_status(&mut self.tree, &name, SelectionState::Install)?;
        Ok(self)
    }
}

/// Parse one deb822 status paragraph and record its package's installed
/// state. Returns `Some` (the queried package's state) when the queried
/// package is found, `None` when this paragraph is a different package (or
/// unparsable).
fn scan_paragraph(text: &str, name: &str, answers: &mut HashMap<String, bool>) -> Option<bool> {
    let tree = Deb822::parse(text).tree();
    let pkg = DpkgPackage::from_paragraph(&tree.paragraphs().next()?).ok()?;
    if pkg.name.is_empty() {
        return None;
    }
    let installed = pkg.selection_state().is_installed();
    answers.insert(pkg.name.clone(), installed);
    (pkg.name == name).then_some(installed)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    fn write_status(contents: &str) -> tempfile::TempDir {
        let dir = tempfile::tempdir().unwrap();
        fs::write(dir.path().join("status"), contents).unwrap();
        dir
    }

    #[test]
    fn lazy_scans_only_until_target_package() {
        // The queried package is second-to-last; a full parse would read the
        // trailing `zzz` paragraph too, but the lazy scan stops at `bash`.
        let dir = write_status(
            "Package: foo\nStatus: deinstall ok config-files\n\n\
             Package: bash\nStatus: install ok installed\n\n\
             Package: zzz\nStatus: deinstall ok config-files\n",
        );
        let state = DpkgState::from_file_lazy(dir.path().join("status"));

        assert!(state.is_installed("bash"));
        // Already answered: served from the cache without re-reading.
        assert!(state.is_installed("bash"));
        // The paragraph *after* the early stop still resolves on a re-scan.
        assert!(!state.is_installed("zzz"));
        // A package not in the file at all.
        assert!(!state.is_installed("not-there"));
        // Lazy mode does not build the index: version queries are unsupported.
        assert_eq!(state.installed_version("bash"), None);
    }

    #[test]
    fn lazy_missing_file_reports_not_installed() {
        let state = DpkgState::from_file_lazy("/nonexistent/status");
        assert!(!state.is_installed("bash"));
    }
}
