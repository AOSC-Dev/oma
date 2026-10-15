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

use crate::atomic_write;

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
    /// `mark_auto` / `mark_manual` / `to_file` were called on a
    /// lazily-loaded state ([`AptExtendedStates::from_file_lazy`]), which has
    /// no tree to mutate or write back.
    #[error("Extended states were loaded lazily; cannot mark packages or write them back")]
    NotLoaded,
}

/// A single entry from `/var/lib/apt/extended_states`.
#[derive(Debug, FromDeb822)]
struct ExtendedStateEntry {
    package: String,
    #[deb822(field = "Auto-Installed")]
    auto_installed: Option<String>,
}

/// The leading base-10 integer of `value`, like `strtol(value, &end, 10)`:
/// leading whitespace and a sign are accepted, the digits that follow are
/// the number, and a value with no digits at all yields `None` (apt then
/// takes its `0` default). An out-of-range digit run saturates instead of
/// failing, like the `strtol` result apt still compares against zero.
fn parse_decimal_prefix(value: &str) -> Option<i64> {
    let rest = value.trim_start();
    let (negative, digits) = match rest.strip_prefix('-') {
        Some(digits) => (true, digits),
        None => (false, rest.strip_prefix('+').unwrap_or(rest)),
    };
    let end = digits
        .find(|c: char| !c.is_ascii_digit())
        .unwrap_or(digits.len());
    if end == 0 {
        return None;
    }
    let magnitude = digits[..end].parse::<i64>().unwrap_or(i64::MAX);
    Some(if negative { -magnitude } else { magnitude })
}

/// Whether an `Auto-Installed` value marks a package as automatic.
///
/// apt reads the field as an integer (`FindI` → `strtol`) and treats any
/// value greater than zero as automatic. The numeric prefix decides, so
/// `Auto-Installed: 2` and `1junk` are automatic, while a value with no
/// digits at all — `yes`, say — parses as 0 and stays manual. apt itself
/// always writes `1`/`0`.
///
/// See:
/// - https://salsa.debian.org/apt-team/apt/-/blob/main/apt-pkg/depcache.cc?ref_type=heads#L312
///   (reading: `FindI("Auto-Installed", 0)` then `reason > 0`)
/// - https://salsa.debian.org/apt-team/apt/-/blob/main/apt-pkg/tagfile.cc?ref_type=heads#L761-L786
///   (the `strtol` integer parse behind `FindI`)
/// - https://salsa.debian.org/apt-team/apt/-/blob/main/apt-pkg/depcache.cc?ref_type=heads#L407
///   (writing: always `"1"` / `"0"`)
fn is_auto_flag(value: &str) -> bool {
    parse_decimal_prefix(value).is_some_and(|n| n > 0)
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
        let path = path.as_ref();
        // A missing file reads as empty (and has no stamp to compare
        // against); an existing file is stamped as it is read, so a later
        // `to_file` can tell whether another writer changed it meanwhile.
        let tree = match std::fs::File::open(path) {
            Ok(file) => Deb822::read(&file)?,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Deb822::new(),
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
                    let is_auto = entry.auto_installed.as_deref().is_some_and(is_auto_flag);
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
    ///
    /// A lazily-loaded state ([`Self::from_file_lazy`]) never read the tree,
    /// so writing it back would truncate the extended-states file: that
    /// returns [`ExtendedStatesError::NotLoaded`] instead.
    ///
    /// # Single writer
    ///
    /// The write is atomic (temp file + rename, see [`crate::atomic_write`]):
    /// a reader sees the old file or the complete new one, never a partial
    /// write, and a failed write leaves the file untouched. It is not a merge
    /// — the tree is written as it is — so whoever holds an
    /// [`AptExtendedStates`] across a write-back is responsible for
    /// serializing the whole read-modify-write against everything else that
    /// touches the file, and must not write it back while another package
    /// manager may modify it. oma's transaction layer does that by holding
    /// apt's dpkg frontend lock (`/var/lib/dpkg/lock-frontend`) for the whole
    /// operation.
    ///
    /// A symlinked state file is replaced at its target, not turned into a
    /// regular file.
    pub fn to_file(&self, path: impl AsRef<Path>) -> Result<(), ExtendedStatesError> {
        if self.lazy_path.is_some() {
            return Err(ExtendedStatesError::NotLoaded);
        }
        let path = atomic_write::resolve(path.as_ref());
        crate::atomic_write::write(&path, &self.tree.to_string())?;
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
        let changed = apply_auto_flag(&mut self.tree, &names, auto);
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
    let auto = entry.auto_installed.as_deref().is_some_and(is_auto_flag);
    answers.insert(entry.package.clone(), auto);
    (entry.package == name).then_some(auto)
}

/// Set (`auto`) or remove the `Auto-Installed` flag of `names` on `tree`,
/// appending a stanza for packages that have none. Whether the flag actually
/// changes decides the return value; names that already have the wanted
/// state are left alone.
fn apply_auto_flag(tree: &mut Deb822, names: &[String], auto: bool) -> bool {
    let mut changed = false;
    // The lossless tree is mutable: set/remove on a paragraph handle changes
    // the tree in place (like `tree[pkg].Auto-Installed = …`).
    for mut paragraph in tree.paragraphs() {
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
        let mut existing: HashSet<String> =
            tree.paragraphs().filter_map(|p| p.get("Package")).collect();
        for name in names {
            if !existing.insert(name.clone()) {
                continue;
            }
            // Append a fresh paragraph and set its fields — in place too.
            let mut paragraph = tree.add_paragraph();
            paragraph.set("Package", name);
            paragraph.set("Auto-Installed", "1");
            changed = true;
        }
    }
    changed
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

Package: vim
Architecture: amd64
Auto-Installed: 2

Package: curl
Architecture: amd64
Auto-Installed: 1junk
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
        // apt parses the field as an integer: `yes` is not one and reads as
        // 0 (manual), while `2` is greater than zero (auto).
        assert!(!states.is_auto_installed("zsh"));
        assert!(states.is_auto_installed("vim"));
        // `strtol` reads the numeric prefix, so the trailing junk is ignored.
        assert!(states.is_auto_installed("curl"));
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

    /// Duplicate names must not append one stanza each — apt would then see
    /// several contradictory entries for the same package.
    #[test]
    fn mark_auto_appends_each_name_once() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("extended_states");

        let mut ext = AptExtendedStates::from_file(&path).unwrap();
        ext.mark_auto(&["liba".to_string(), "liba".to_string()])
            .unwrap();
        ext.to_file(&path).unwrap();

        let written = std::fs::read_to_string(&path).unwrap();
        assert_eq!(written.matches("Package: liba").count(), 1);
        assert!(
            AptExtendedStates::from_file(&path)
                .unwrap()
                .is_auto_installed("liba")
        );
    }

    /// Same atomic write-back as the dpkg status file.
    #[test]
    fn to_file_replaces_atomically() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("extended_states");

        let mut ext = AptExtendedStates::from_file(&path).unwrap();
        ext.mark_auto(&["liba".to_string()]).unwrap();
        ext.to_file(&path).unwrap();

        assert!(
            AptExtendedStates::from_file(&path)
                .unwrap()
                .is_auto_installed("liba")
        );
        assert!(!dir.path().join("extended_states.tmp").exists());
    }

    /// A lazy state never read the tree: writing the empty placeholder back
    /// would truncate the extended-states file, so it refuses instead.
    #[test]
    fn lazy_state_refuses_to_write_back() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("extended_states");
        std::fs::File::create(&path)
            .unwrap()
            .write_all(EXTENDED.as_bytes())
            .unwrap();
        let before = std::fs::read_to_string(&path).unwrap();

        let lazy = AptExtendedStates::from_file_lazy(&path);
        assert!(matches!(
            lazy.to_file(&path),
            Err(ExtendedStatesError::NotLoaded)
        ));
        assert_eq!(std::fs::read_to_string(&path).unwrap(), before);
    }

    /// apt's integer parse applies to both readers: the eager index and the
    /// lazy scan must agree on it.
    #[test]
    fn auto_installed_matches_apt_integer_semantics() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("extended_states");
        std::fs::File::create(&path)
            .unwrap()
            .write_all(EXTENDED.as_bytes())
            .unwrap();

        let eager = AptExtendedStates::from_file(&path).unwrap();
        let lazy = AptExtendedStates::from_file_lazy(&path);
        for (name, expected) in [
            ("fish", true),
            ("bash", false),
            ("zsh", false),
            ("vim", true),
            ("curl", true),
        ] {
            assert_eq!(eager.is_auto_installed(name), expected, "eager: {name}");
            assert_eq!(lazy.is_auto_installed(name), expected, "lazy: {name}");
        }
    }

    #[test]
    fn is_auto_flag_follows_apt_parse() {
        // Leading whitespace and a sign are accepted, the numeric prefix is
        // the number (`strtol` stops at the first non-digit), and only a
        // value greater than zero is automatic.
        for (value, expected) in [
            ("1", true),
            ("2", true),
            ("0", false),
            ("-1", false),
            (" 1", true),
            ("+1", true),
            ("1junk", true),
            ("2x", true),
            ("0junk", false),
            ("-1x", false),
            ("yes", false),
            ("true", false),
            ("", false),
            ("garbage", false),
        ] {
            assert_eq!(is_auto_flag(value), expected, "value: {value:?}");
        }
    }

    #[test]
    fn mark_auto_creates_and_merges() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("extended_states");

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
    }

    #[test]
    fn mark_auto_empty_is_noop() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("extended_states");
        let mut ext = AptExtendedStates::from_file(&path).unwrap();
        ext.mark_auto(std::iter::empty::<&str>()).unwrap();
        assert!(!path.exists());
    }

    #[test]
    fn mark_manual_removes_flag() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("extended_states");

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
    }
}
