//! APT extended states — tracks which packages were auto-installed.
//!
//! APT maintains `/var/lib/apt/extended_states` in deb822 format to record
//! whether each package was installed automatically as a dependency:
//!
//! ```text
//! Package: foo
//! Architecture: amd64
//! Auto-Installed: 1
//! ```
//!
//! [`AptExtendedStates::from_file`] reads the file into a lossless tree;
//! [`AptExtendedStates::mark_auto`] and [`AptExtendedStates::mark_manual`]
//! flip the `Auto-Installed` flag, like `apt-mark auto` / `apt-mark manual`,
//! and [`AptExtendedStates::to_file`] writes the tree back.

use std::cell::{OnceCell, RefCell};
use std::collections::{HashMap, HashSet};
use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf};

use deb822_fast::{FromDeb822, FromDeb822Paragraph};
use deb822_lossless::Deb822;
use thiserror::Error;

/// Errors that can occur when reading or writing APT extended states.
#[derive(Debug, Error)]
pub enum ExtendedStatesError {
    /// Failed to open the extended states file.
    #[error("Failed to open extended states file: {0}")]
    Io(#[from] std::io::Error),
    /// Failed to parse the extended states file.
    #[error("Failed to parse extended states file: {0}")]
    Deb822(#[from] deb822_lossless::Error),
    /// `mark_auto` / `mark_manual` were called on a lazily-loaded state
    /// ([`AptExtendedStates::from_file_lazy`]), which has no tree to mutate.
    #[error("Extended states were loaded lazily; cannot mark packages")]
    NotLoaded,
}

/// A single entry from `/var/lib/apt/extended_states`.
#[derive(Debug, FromDeb822)]
struct ExtendedStateEntry {
    package: String,
    #[deb822(field = "Auto-Installed")]
    auto_installed: Option<String>,
}

/// Parsed APT extended states, providing the auto-installed flag per package.
///
/// Read from `/var/lib/apt/extended_states` via [`AptExtendedStates::from_file`].
/// The default is empty — no package auto-installed (e.g. when the file does
/// not exist).
///
/// A read-only consumer like `oma show` can instead construct the state
/// lazily via [`AptExtendedStates::from_file_lazy`]: the file is scanned only
/// until the queried package is found, so the whole file is never parsed.
#[derive(Debug, Clone)]
pub struct AptExtendedStates {
    /// The lossless extended-states tree — the single source of truth;
    /// [`Self::to_file`] writes it back. Empty in lazy mode
    /// ([`Self::from_file_lazy`]).
    tree: Deb822,
    /// Lazily-built auto-installed set, so `from_file` only parses the tree.
    auto_installed: OnceCell<HashSet<String>>,
    /// Extended states file path, kept when constructed lazily; `None` in
    /// eager mode. [`Self::is_auto_installed`] then scans the file only until
    /// the queried package is found.
    lazy_path: Option<PathBuf>,
    /// Lazily-answered `is_auto_installed` results (name → auto-installed),
    /// filled by partial scans. Only used in lazy mode.
    lazy_answers: RefCell<HashMap<String, bool>>,
}

impl Default for AptExtendedStates {
    fn default() -> Self {
        Self {
            tree: Deb822::new(),
            auto_installed: OnceCell::new(),
            lazy_path: None,
            lazy_answers: RefCell::new(HashMap::new()),
        }
    }
}

impl AptExtendedStates {
    /// Parse the extended states file at the given path. A missing file is
    /// not an error — it just means nothing was auto-installed yet.
    pub fn from_file(path: impl AsRef<Path>) -> Result<Self, ExtendedStatesError> {
        let tree = match Deb822::from_file(path) {
            Ok(tree) => tree,
            Err(deb822_lossless::Error::IoError(e)) if e.kind() == std::io::ErrorKind::NotFound => {
                Deb822::new()
            }
            Err(e) => return Err(e.into()),
        };
        Ok(Self {
            tree,
            auto_installed: OnceCell::new(),
            lazy_path: None,
            lazy_answers: RefCell::new(HashMap::new()),
        })
    }

    /// Record the extended states file path without parsing it;
    /// [`Self::is_auto_installed`] then scans until the queried package is
    /// found. Infallible — a missing or unreadable file simply reports
    /// nothing as auto-installed. The marking methods
    /// ([`Self::mark_auto`] / [`Self::mark_manual`]) are unsupported on a
    /// lazy state (there is no tree to mutate) and return
    /// [`ExtendedStatesError::NotLoaded`].
    pub fn from_file_lazy(path: impl AsRef<Path>) -> Self {
        Self {
            tree: Deb822::new(),
            auto_installed: OnceCell::new(),
            lazy_path: Some(path.as_ref().to_path_buf()),
            lazy_answers: RefCell::new(HashMap::new()),
        }
    }

    /// The lazily-built auto-installed set.
    fn auto_installed(&self) -> &HashSet<String> {
        self.auto_installed.get_or_init(|| {
            self.tree
                .paragraphs()
                .filter_map(|para| {
                    let entry = ExtendedStateEntry::from_paragraph(&para).ok()?;
                    let is_auto = entry
                        .auto_installed
                        .as_deref()
                        .is_some_and(|v| v == "1" || v == "yes");
                    is_auto.then_some(entry.package)
                })
                .collect()
        })
    }

    /// Whether the given package was automatically installed as a dependency.
    /// In lazy mode ([`Self::from_file_lazy`]) this scans the file only until
    /// `name` is found.
    pub fn is_auto_installed(&self, name: &str) -> bool {
        if let Some(path) = &self.lazy_path {
            self.is_auto_installed_lazy(name, path)
        } else {
            self.auto_installed().contains(name)
        }
    }

    /// Lazily answer `is_auto_installed` for `name`: scan the file, stopping
    /// as soon as the package is found (or at EOF), and cache every
    /// package's state seen along the way. A later query for a package not
    /// reached by an earlier early stop re-scans from the start. (This
    /// deb822_lossless release has no reader-based iterator, so paragraphs
    /// are gathered line-by-line and parsed one at a time.)
    fn is_auto_installed_lazy(&self, name: &str, path: &Path) -> bool {
        let mut answers = self.lazy_answers.borrow_mut();
        if let Some(&auto) = answers.get(name) {
            return auto;
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
                        && let Some(auto) = scan_extended_paragraph(&paragraph, name, &mut answers)
                    {
                        return auto;
                    }
                    break;
                }
                Ok(_) => {
                    if line.trim().is_empty() {
                        if !paragraph.is_empty() {
                            if let Some(auto) =
                                scan_extended_paragraph(&paragraph, name, &mut answers)
                            {
                                return auto;
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

    /// Write the loaded tree back to `path` — marks set by [`Self::mark_auto`]
    /// / [`Self::mark_manual`] are already in the tree, so this is a plain
    /// lossless write (everything else is preserved). The counterpart of
    /// [`Self::from_file`].
    pub fn to_file(&self, path: impl AsRef<Path>) -> Result<(), ExtendedStatesError> {
        std::fs::write(path, self.tree.to_string())?;
        Ok(())
    }

    /// Record `Auto-Installed: 1` for `names`, like `apt-mark auto`: each
    /// package's paragraph is updated (or appended if absent), everything
    /// else is preserved, and [`Self::to_file`] writes it back.
    pub fn mark_auto(
        &mut self,
        names: impl IntoIterator<Item = impl AsRef<str>>,
    ) -> Result<(), ExtendedStatesError> {
        self.mark(names, true)
    }

    /// Remove the `Auto-Installed` flag for `names`, like `apt-mark manual`:
    /// the packages are then treated as manually installed.
    pub fn mark_manual(
        &mut self,
        names: impl IntoIterator<Item = impl AsRef<str>>,
    ) -> Result<(), ExtendedStatesError> {
        self.mark(names, false)
    }

    fn mark(
        &mut self,
        names: impl IntoIterator<Item = impl AsRef<str>>,
        auto: bool,
    ) -> Result<(), ExtendedStatesError> {
        if self.lazy_path.is_some() {
            return Err(ExtendedStatesError::NotLoaded);
        }
        // The iterator may be single-use, so materialize it once (callers
        // pass small name sets).
        let names: Vec<String> = names.into_iter().map(|n| n.as_ref().to_string()).collect();
        if names.is_empty() {
            return Ok(());
        }
        // The lossless tree is mutable: set/remove on a paragraph handle
        // changes the tree in place (like `tree[pkg].Auto-Installed = …`).
        let mut changed = false;
        for mut paragraph in self.tree.paragraphs() {
            let Some(package) = paragraph.get("Package") else {
                continue;
            };
            if !names.iter().any(|n| n.as_str() == package) {
                continue;
            }
            let flag = paragraph.get("Auto-Installed");
            if auto && flag.as_deref() != Some("1") {
                paragraph.set("Auto-Installed", "1");
                changed = true;
            } else if !auto && flag.is_some() {
                paragraph.remove("Auto-Installed");
                changed = true;
            }
        }
        if auto {
            let existing: HashSet<String> = self
                .tree
                .paragraphs()
                .filter_map(|p| p.get("Package"))
                .collect();
            for name in &names {
                if existing.contains(name) {
                    continue;
                }
                // Append a fresh paragraph and set its fields — in place too.
                let mut paragraph = self.tree.add_paragraph();
                paragraph.set("Package", name);
                paragraph.set("Auto-Installed", "1");
                changed = true;
            }
        }
        if changed {
            // The lazily-derived set is now stale — rebuild on next access.
            self.auto_installed = OnceCell::new();
        }
        Ok(())
    }
}

/// Parse one extended-states paragraph and record its package's
/// auto-installed state. Returns `Some` (the queried package's state) when
/// the queried package is found, `None` when this paragraph is a different
/// package (or unparsable).
fn scan_extended_paragraph(
    text: &str,
    name: &str,
    answers: &mut HashMap<String, bool>,
) -> Option<bool> {
    let tree = Deb822::parse(text).tree();
    let entry = ExtendedStateEntry::from_paragraph(&tree.paragraphs().next()?).ok()?;
    if entry.package.is_empty() {
        return None;
    }
    let auto = entry
        .auto_installed
        .as_deref()
        .is_some_and(|v| v == "1" || v == "yes");
    answers.insert(entry.package.clone(), auto);
    (entry.package == name).then_some(auto)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    const EXTENDED: &str = "\
Package: fish
Architecture: amd64
Auto-Installed: 1

Package: bash
Architecture: amd64
Auto-Installed: 0

Package: zsh
Architecture: amd64
Auto-Installed: yes
";

    #[test]
    fn lazy_is_auto_installed() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("extended_states");
        std::fs::File::create(&path)
            .unwrap()
            .write_all(EXTENDED.as_bytes())
            .unwrap();

        let states = AptExtendedStates::from_file_lazy(&path);
        assert!(states.is_auto_installed("fish"));
        assert!(!states.is_auto_installed("bash"));
        assert!(states.is_auto_installed("zsh"));
        assert!(!states.is_auto_installed("nosuchpkg"));
        // A missing file reports nothing as auto-installed.
        let missing = AptExtendedStates::from_file_lazy("/nonexistent/extended_states");
        assert!(!missing.is_auto_installed("fish"));
        // Lazy mode cannot mark — no tree to mutate.
        let mut states = states;
        assert!(matches!(
            states.mark_auto(["liba".to_string()]),
            Err(ExtendedStatesError::NotLoaded)
        ));
    }

    #[test]
    fn mark_auto_creates_and_merges() {
        let path = std::env::temp_dir().join("oma-extended-states-test");
        let _ = std::fs::remove_file(&path);

        // A missing file reads as empty; marking auto + to_file creates it.
        let mut ext = AptExtendedStates::from_file(&path).unwrap();
        ext.mark_auto(&["liba".to_string()]).unwrap();
        ext.to_file(&path).unwrap();
        let states = AptExtendedStates::from_file(&path).unwrap();
        assert!(states.is_auto_installed("liba"));
        assert!(!states.is_auto_installed("libb"));

        // A later write appends new names and keeps existing entries (the
        // already-recorded package is not duplicated).
        let mut ext = AptExtendedStates::from_file(&path).unwrap();
        ext.mark_auto(&["libb".to_string(), "liba".to_string()])
            .unwrap();
        ext.to_file(&path).unwrap();
        let states = AptExtendedStates::from_file(&path).unwrap();
        assert!(states.is_auto_installed("liba"));
        assert!(states.is_auto_installed("libb"));

        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn mark_auto_empty_is_noop() {
        let path = std::env::temp_dir().join("oma-extended-states-empty-test");
        let _ = std::fs::remove_file(&path);
        let mut ext = AptExtendedStates::from_file(&path).unwrap();
        ext.mark_auto(std::iter::empty::<&str>()).unwrap();
        assert!(!path.exists());
    }

    #[test]
    fn mark_manual_removes_flag() {
        let path = std::env::temp_dir().join("oma-extended-states-manual-test");
        let _ = std::fs::remove_file(&path);

        let mut ext = AptExtendedStates::from_file(&path).unwrap();
        ext.mark_auto(&["liba".to_string()]).unwrap();
        ext.to_file(&path).unwrap();
        assert!(
            AptExtendedStates::from_file(&path)
                .unwrap()
                .is_auto_installed("liba")
        );

        let mut ext = AptExtendedStates::from_file(&path).unwrap();
        ext.mark_manual(&["liba".to_string()]).unwrap();
        ext.to_file(&path).unwrap();
        assert!(
            !AptExtendedStates::from_file(&path)
                .unwrap()
                .is_auto_installed("liba")
        );

        // mark_manual on a package without the flag is a no-op.
        let mut ext = AptExtendedStates::from_file(&path).unwrap();
        ext.mark_manual(&["libb".to_string()]).unwrap();
        ext.to_file(&path).unwrap();
        let states = AptExtendedStates::from_file(&path).unwrap();
        assert!(!states.is_auto_installed("liba"));
        assert!(!states.is_auto_installed("libb"));

        let _ = std::fs::remove_file(&path);
    }
}
