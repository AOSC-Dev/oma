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
use std::sync::OnceLock;

use deb822_fast::FromDeb822Paragraph;
use deb822_lossless::Deb822;

use crate::atomic_write;
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
        for p in packages_best_effort(tree) {
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
    /// Installed-version map for lazy mode, filled by a single full scan on
    /// the first version lookup — the index is built from the tree, and lazy
    /// mode never loads one.
    lazy_versions: OnceLock<HashMap<String, String>>,
}

/// Parse the status tree best-effort: a paragraph that does not parse as a
/// package entry is skipped, like the crate's other deb822 readers — one
/// malformed stanza must not hide every other package.
fn packages_best_effort(tree: &Deb822) -> impl Iterator<Item = DpkgPackage> + '_ {
    tree.paragraphs()
        .filter_map(|paragraph| DpkgPackage::from_paragraph(&paragraph).ok())
}

/// Full-scan the status file into an installed-version map. Infallible, like
/// lazy mode: an unreadable file yields an empty map, and a malformed
/// paragraph is skipped rather than emptying the map — the eager index
/// ([`DpkgIndex::from_tree`]) answers the same way.
fn load_installed_versions(path: &Path) -> HashMap<String, String> {
    let Ok(tree) = read_status_tree(path) else {
        return HashMap::new();
    };
    packages_best_effort(&tree)
        .filter(|p| p.selection_state().is_installed())
        .map(|p| (p.name, p.version.unwrap_or_default()))
        .collect()
}

impl DpkgState {
    /// Parse dpkg status from the given status file path. The file is read
    /// once into a lossless tree (kept, so [`Self::mark_held`] can write
    /// holds and [`Self::to_file`] can write it back); the query sets are
    /// derived lazily on first use.
    ///
    pub fn from_file(path: impl AsRef<Path>) -> Result<Self, crate::error::Error> {
        let file = std::fs::File::open(path.as_ref()).map_err(crate::dpkg::DpkgError::from)?;
        let tree = Deb822::read(&file).map_err(crate::dpkg::DpkgError::from)?;
        Ok(Self::from_tree(tree))
    }

    /// Build a state that answers [`Self::is_installed`] by scanning the
    /// status file only until the queried package is found, instead of
    /// parsing every installed package up front — a single-package consumer
    /// like `oma show` never parses the whole status file. Infallible: an
    /// unreadable status file simply reports nothing as installed.
    /// [`Self::installed_version`] scans the file once on first use and
    /// answers like an eager state; the tree-backed queries (holds,
    /// essential, ...) stay unavailable, as no tree is loaded.
    pub fn from_file_lazy(path: impl AsRef<Path>) -> Self {
        Self {
            tree: Deb822::default(),
            index: OnceCell::new(),
            lazy_path: Some(path.as_ref().to_path_buf()),
            lazy_answers: RefCell::new(HashMap::new()),
            lazy_versions: OnceLock::new(),
        }
    }

    /// Build a state from an already-parsed status tree.
    pub(crate) fn from_tree(tree: Deb822) -> Self {
        Self {
            tree,
            index: OnceCell::new(),
            lazy_path: None,
            lazy_answers: RefCell::new(HashMap::new()),
            lazy_versions: OnceLock::new(),
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
    ///
    /// A lazily-loaded state ([`Self::from_file_lazy`]) never read the tree,
    /// so writing it back would truncate the status file: that returns
    /// [`DpkgError::NotLoaded`](crate::dpkg::DpkgError::NotLoaded) instead.
    ///
    /// # Single writer
    ///
    /// The write is atomic (temp file + rename, see [`crate::atomic_write`]):
    /// a reader sees the old status file or the complete new one, never a
    /// partial write, and a failed write leaves the file untouched. It is not
    /// a merge — the tree is written as it is — so whoever holds a
    /// [`DpkgState`] across a write-back is responsible for serializing the
    /// whole read-modify-write against everything else that touches the
    /// status file, and must not write it back while another package manager
    /// (dpkg itself, apt, another oma) may modify it. oma's transaction layer
    /// does that by holding apt's dpkg frontend lock
    /// (`/var/lib/dpkg/lock-frontend`) for the whole operation.
    ///
    /// A symlinked status file is replaced at its target, not turned into a
    /// regular file.
    pub fn to_file(&self, path: impl AsRef<Path>) -> Result<(), crate::dpkg::DpkgError> {
        self.loaded_tree()?;
        let path = atomic_write::resolve(path.as_ref());
        crate::atomic_write::write(&path, &self.tree.to_string())?;
        Ok(())
    }

    /// The loaded tree, or [`DpkgError::NotLoaded`](crate::dpkg::DpkgError::NotLoaded)
    /// for a lazily-loaded state — which never read one, so any write-back
    /// or mark would act on the empty placeholder instead of the file.
    fn loaded_tree(&self) -> Result<&Deb822, crate::dpkg::DpkgError> {
        if self.lazy_path.is_some() {
            return Err(crate::dpkg::DpkgError::NotLoaded);
        }
        Ok(&self.tree)
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
    ///
    /// In lazy mode this scans the status file once on the first call and
    /// caches the result, so callers get the same answer as eager mode
    /// instead of a silent `None`.
    pub fn installed_version(&self, name: &str) -> Option<&str> {
        if let Some(path) = &self.lazy_path {
            let versions = self
                .lazy_versions
                .get_or_init(|| load_installed_versions(path));
            versions.get(name).map(String::as_str)
        } else {
            self.index()
                .installed_versions
                .get(name)
                .map(String::as_str)
        }
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
    /// Like `apt-mark hold`: refuses packages that are not installed, and
    /// re-marking a held package is idempotent (the same selection is
    /// written again). A lazily-loaded state has no tree to set the status
    /// on, so it returns
    /// [`DpkgError::NotLoaded`](crate::dpkg::DpkgError::NotLoaded).
    ///
    /// Every *installed* stanza of `name` gets the hold — no early return on
    /// [`Self::is_held`], which is name-keyed and true as soon as *one*
    /// architecture is held. Returning then would leave the other
    /// architectures unheld, disagreeing with the index. Stanzas that are not
    /// installed (a leftover `deinstall …` of another architecture) are left
    /// alone, like `apt-mark hold` refusing packages that are not installed.
    pub fn mark_held(
        &mut self,
        name: impl Into<String>,
    ) -> Result<&mut Self, crate::dpkg::DpkgError> {
        let name = name.into();
        self.loaded_tree()?;
        if !self.is_installed(&name) {
            return Err(crate::dpkg::DpkgError::NotInstalled(name));
        }
        // Only touch the cached index once the tree actually changed —
        // otherwise a failed mark would leave the index claiming the package
        // is held, and a retry would report a success that was never written.
        crate::dpkg::set_pkg_status(&mut self.tree, &name, SelectionState::Hold)?;
        self.index_mut().held.insert(name.clone());
        Ok(self)
    }

    /// Remove the hold from `name` — it is no longer protected from
    /// autoremove, and `Status: install` is set on its paragraph in the
    /// loaded status tree, so [`Self::to_file`] writes it back. The
    /// counterpart of [`Self::mark_held`], like `apt-mark unhold`.
    ///
    /// Like `apt-mark unhold`: refuses packages that are not installed, and
    /// is a no-op when the package is not held. A lazily-loaded state has
    /// no tree to set the status on, so it returns
    /// [`DpkgError::NotLoaded`](crate::dpkg::DpkgError::NotLoaded).
    pub fn mark_unheld(
        &mut self,
        name: impl Into<String>,
    ) -> Result<&mut Self, crate::dpkg::DpkgError> {
        let name = name.into();
        self.loaded_tree()?;
        if !self.is_installed(&name) {
            return Err(crate::dpkg::DpkgError::NotInstalled(name));
        }
        if !self.is_held(&name) {
            return Ok(self);
        }
        // Same order as `mark_held`: update the cache only after the tree.
        crate::dpkg::set_pkg_status(&mut self.tree, &name, SelectionState::Install)?;
        self.index_mut().held.remove(&name);
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
        // Version queries answer like an eager state even though the index
        // was never built: the lookup falls back to one full scan.
        let eager = DpkgState::from_file(dir.path().join("status")).unwrap();
        for name in ["bash", "zzz", "not-there"] {
            assert_eq!(
                state.installed_version(name),
                eager.installed_version(name),
                "mismatch for {name}"
            );
        }
    }

    /// A single malformed stanza must not hide the rest of the status file:
    /// the index skips only the paragraph it cannot parse.
    #[test]
    fn index_skips_malformed_paragraphs() {
        let dir = write_status(
            "Package: foo\nStatus: install ok installed\nVersion: 1\n\n\
             Package: broken\nStatus: install ok installed extra\n\n\
             Package: bar\nStatus: hold ok installed\nVersion: 2\n",
        );
        let state = DpkgState::from_file(dir.path().join("status")).unwrap();

        assert!(state.is_installed("foo"));
        assert!(state.is_installed("bar"));
        assert_eq!(state.installed_version("bar"), Some("2"));
        // The unparsable paragraph is skipped, not the whole file, and the
        // queries for the remaining paragraphs still resolve.
        assert!(!state.is_installed("broken"));
        assert!(state.is_held("bar"));
        assert!(!state.is_held("foo"));
    }

    /// The lazy version lookup reads the whole file at once, so it must skip
    /// malformed paragraphs like the eager index instead of emptying the map.
    #[test]
    fn lazy_versions_skip_malformed_paragraphs() {
        let dir = write_status(
            "Package: foo\nStatus: install ok installed\nVersion: 1\n\n\
             Package: broken\nStatus: install ok installed extra\n\n\
             Package: bar\nStatus: hold ok installed\nVersion: 2\n",
        );
        let lazy = DpkgState::from_file_lazy(dir.path().join("status"));
        assert_eq!(lazy.installed_version("foo"), Some("1"));
        assert_eq!(lazy.installed_version("bar"), Some("2"));
        assert_eq!(lazy.installed_version("broken"), None);

        // Same answers as the eager state, which builds the index instead.
        let eager = DpkgState::from_file(dir.path().join("status")).unwrap();
        assert_eq!(eager.installed_version("foo"), Some("1"));
        assert_eq!(eager.installed_version("bar"), Some("2"));
    }

    /// A lazy state never read the tree: writing the empty placeholder back
    /// would truncate the status file, so the write paths refuse instead.
    #[test]
    fn lazy_state_refuses_to_write_back() {
        let dir = write_status("Package: foo\nStatus: install ok installed\nVersion: 1\n");
        let path = dir.path().join("status");
        let before = fs::read_to_string(&path).unwrap();

        let lazy = DpkgState::from_file_lazy(&path);
        assert!(matches!(
            lazy.to_file(&path),
            Err(crate::dpkg::DpkgError::NotLoaded)
        ));
        assert_eq!(fs::read_to_string(&path).unwrap(), before);

        let mut lazy = DpkgState::from_file_lazy(&path);
        assert!(matches!(
            lazy.mark_held("foo"),
            Err(crate::dpkg::DpkgError::NotLoaded)
        ));
        assert!(matches!(
            lazy.mark_unheld("foo"),
            Err(crate::dpkg::DpkgError::NotLoaded)
        ));
        // Neither mark wrote anything: the file is untouched.
        assert_eq!(fs::read_to_string(&path).unwrap(), before);
    }

    /// The guard is lazy-only: an eager state still marks and writes back.
    #[test]
    fn eager_state_still_writes_back() {
        let dir = write_status("Package: foo\nStatus: install ok installed\nVersion: 1\n");
        let path = dir.path().join("status");
        let out = dir.path().join("status.out");

        let mut state = DpkgState::from_file(&path).unwrap();
        state.mark_held("foo").unwrap();
        assert!(state.is_held("foo"));
        state.to_file(&out).unwrap();

        let written = fs::read_to_string(&out).unwrap();
        assert!(written.contains("Package: foo"));
        assert!(written.contains("Status: hold ok installed"));
    }

    /// A malformed stanza used to abort the mark (the paragraph walk parsed
    /// strictly) and leave the cached index claiming success, so a retry
    /// reported a hold that was never written.
    #[test]
    fn mark_held_survives_malformed_paragraphs() {
        let dir = write_status(
            "Package: broken\nStatus: install ok installed extra\n\n\
             Package: foo\nStatus: install ok installed\nVersion: 1\n",
        );
        let path = dir.path().join("status");

        let mut state = DpkgState::from_file(&path).unwrap();
        state.mark_held("foo").unwrap();
        assert!(state.is_held("foo"));
        state.to_file(&path).unwrap();

        // Write-back then reload: the hold is actually in the file.
        let reloaded = DpkgState::from_file(&path).unwrap();
        assert!(reloaded.is_held("foo"));
        assert_eq!(reloaded.installed_version("foo"), Some("1"));
        // The malformed stanza is preserved verbatim, not dropped, and the
        // write leaves no temp file behind.
        let written = fs::read_to_string(&path).unwrap();
        assert!(written.contains("Package: broken"));
        assert!(!dir.path().join("status.tmp").exists());
    }

    /// A mark that fails after the index was built must not cache its
    /// success — the index reflects the tree, which never changed.
    #[test]
    fn failed_mark_does_not_poison_the_index() {
        let dir = write_status("Package: foo\nStatus: install ok installed\nVersion: 1\n");
        let path = dir.path().join("status");
        let mut state = DpkgState::from_file(&path).unwrap();
        assert!(state.is_installed("foo")); // builds the index from the tree

        // The paragraph vanishes before the edit lands (as if the file were
        // rewritten underneath us): the mark fails...
        state.tree = Deb822::default();
        assert!(state.mark_held("foo").is_err());
        // ...and nothing was cached, so a retry fails the same way instead of
        // reporting a phantom success.
        assert!(!state.is_held("foo"));
        assert!(state.mark_held("foo").is_err());
        assert!(!state.is_held("foo"));
    }

    /// The write-back replaces the file atomically, so it cannot be caught
    /// half-written.
    #[test]
    fn to_file_replaces_atomically() {
        let dir = write_status("Package: foo\nStatus: install ok installed\nVersion: 1\n");
        let path = dir.path().join("status");

        let mut state = DpkgState::from_file(&path).unwrap();
        state.mark_held("foo").unwrap();
        state.to_file(&path).unwrap();

        let written = fs::read_to_string(&path).unwrap();
        assert!(written.contains("Status: hold ok installed"));
        assert!(!dir.path().join("status.tmp").exists());
    }

    /// Multi-arch: every stanza of the package is updated, matching the
    /// name-keyed index — updating only the first would let a later
    /// architecture stay held while `is_held` already reports it unheld.
    #[test]
    fn mark_covers_all_architectures() {
        let dir = write_status(
            "Package: foo\nArchitecture: amd64\nStatus: hold ok installed\nVersion: 1\n\n\
             Package: foo\nArchitecture: i386\nStatus: install ok installed\nVersion: 1\n",
        );
        let path = dir.path().join("status");

        let mut state = DpkgState::from_file(&path).unwrap();
        state.mark_unheld("foo").unwrap();
        assert!(!state.is_held("foo"));
        state.to_file(&path).unwrap();
        let reloaded = DpkgState::from_file(&path).unwrap();
        assert!(!reloaded.is_held("foo"));

        let mut state = DpkgState::from_file(&path).unwrap();
        state.mark_held("foo").unwrap();
        assert!(state.is_held("foo"));
        state.to_file(&path).unwrap();

        // Both stanzas carry the hold on disk.
        let written = fs::read_to_string(&path).unwrap();
        assert_eq!(written.matches("Status: hold ok installed").count(), 2);
        assert!(DpkgState::from_file(&path).unwrap().is_held("foo"));
    }

    /// Mixed architectures: one stanza held, one not. The name-keyed
    /// `is_held` already reports the package as held, but `mark_held` must
    /// still normalize every stanza instead of returning early.
    #[test]
    fn mark_held_normalizes_partially_held_arches() {
        let dir = write_status(
            "Package: foo\nArchitecture: amd64\nStatus: install ok installed\nVersion: 1\n\n\
             Package: foo\nArchitecture: i386\nStatus: hold ok installed\nVersion: 1\n",
        );
        let path = dir.path().join("status");

        let mut state = DpkgState::from_file(&path).unwrap();
        assert!(state.is_held("foo")); // via the i386 stanza
        state.mark_held("foo").unwrap();
        state.to_file(&path).unwrap();

        // Both stanzas carry the hold on disk.
        let written = fs::read_to_string(&path).unwrap();
        assert_eq!(written.matches("Status: hold ok installed").count(), 2);

        // Re-marking is idempotent: the file stays as written.
        let mut state = DpkgState::from_file(&path).unwrap();
        state.mark_held("foo").unwrap();
        state.to_file(&path).unwrap();
        assert_eq!(fs::read_to_string(&path).unwrap(), written);
    }

    /// A leftover stanza of another architecture (`deinstall …
    /// config-files`) is not installed: marking the installed architecture
    /// must leave it alone. Rewriting it to `hold` would change dpkg's
    /// intent and make the stale stanza count as installed on the next load.
    #[test]
    fn mark_held_leaves_not_installed_stanzas_alone() {
        let dir = write_status(
            "Package: foo\nArchitecture: amd64\nStatus: install ok installed\nVersion: 1\n\n\
             Package: foo\nArchitecture: i386\nStatus: deinstall ok config-files\nVersion: 2\n",
        );
        let path = dir.path().join("status");

        let mut state = DpkgState::from_file(&path).unwrap();
        state.mark_held("foo").unwrap();
        state.to_file(&path).unwrap();

        let written = fs::read_to_string(&path).unwrap();
        assert_eq!(written.matches("Status: hold ok installed").count(), 1);
        assert!(written.contains("Status: deinstall ok config-files"));

        let reloaded = DpkgState::from_file(&path).unwrap();
        assert!(reloaded.is_held("foo"));
        // The stale stanza is still not installed: if it had been rewritten
        // to `hold`, its version would be the one recorded for `foo`.
        assert_eq!(reloaded.installed_version("foo"), Some("1"));
    }

    /// Writing back and then editing further still works: the state re-stamps
    /// the file it wrote.
    #[test]
    fn repeated_write_backs_stay_valid() {
        let dir = write_status(
            "Package: foo\nStatus: install ok installed\nVersion: 1\n\n\
             Package: bar\nStatus: install ok installed\nVersion: 1\n",
        );
        let path = dir.path().join("status");

        let mut state = DpkgState::from_file(&path).unwrap();
        state.mark_held("foo").unwrap();
        state.to_file(&path).unwrap();

        state.mark_held("bar").unwrap();
        state.to_file(&path).unwrap();

        let reloaded = DpkgState::from_file(&path).unwrap();
        assert!(reloaded.is_held("foo"));
        assert!(reloaded.is_held("bar"));
    }

    /// A symlinked status file is replaced at its target: the link stays a
    /// link, and the target gets the new contents.
    #[test]
    fn to_file_replaces_a_symlinks_target() {
        use std::os::unix::fs::symlink;

        let dir = write_status("Package: foo\nStatus: install ok installed\nVersion: 1\n");
        let target = dir.path().join("status");
        let link = dir.path().join("status.link");
        symlink(&target, &link).unwrap();

        let mut state = DpkgState::from_file(&link).unwrap();
        state.mark_held("foo").unwrap();
        state.to_file(&link).unwrap();

        assert!(
            fs::symlink_metadata(&link)
                .unwrap()
                .file_type()
                .is_symlink()
        );
        assert!(
            fs::read_to_string(&link)
                .unwrap()
                .contains("Status: hold ok installed")
        );
        assert!(DpkgState::from_file(&target).unwrap().is_held("foo"));
    }

    #[test]
    fn lazy_missing_file_reports_not_installed() {
        let state = DpkgState::from_file_lazy("/nonexistent/status");
        assert!(!state.is_installed("bash"));
    }
}
