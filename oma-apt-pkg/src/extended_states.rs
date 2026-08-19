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

use std::cell::RefCell;
use std::collections::HashMap;
use std::path::{Path, PathBuf};

use deb822_fast::{Deb822, FromDeb822, FromDeb822Paragraph};

/// A single entry from `/var/lib/apt/extended_states`.
#[derive(Debug, FromDeb822)]
struct ExtendedStateEntry {
    package: String,
    #[deb822(field = "Auto-Installed")]
    auto_installed: Option<String>,
}

/// APT extended states, providing the auto-installed flag per package.
///
/// Constructed lazily via [`AptExtendedStates::from_file_lazy`]: the file is
/// scanned only until the queried package is found, so `oma show` never
/// parses the whole file.
#[derive(Debug, Clone)]
pub struct AptExtendedStates {
    /// Extended states file path.
    path: PathBuf,
    /// Lazily-answered `is_auto_installed` results (name → auto-installed),
    /// filled by partial scans.
    answers: RefCell<HashMap<String, bool>>,
}

impl AptExtendedStates {
    /// Record the extended states file path without parsing it;
    /// [`Self::is_auto_installed`] then scans until the queried package is
    /// found. Infallible — a missing or unreadable file simply reports
    /// nothing as auto-installed.
    pub fn from_file_lazy(path: impl AsRef<Path>) -> Self {
        Self {
            path: path.as_ref().to_path_buf(),
            answers: RefCell::new(HashMap::new()),
        }
    }

    /// Whether the given package was automatically installed as a dependency.
    pub fn is_auto_installed(&self, name: &str) -> bool {
        let mut answers = self.answers.borrow_mut();
        if let Some(&auto) = answers.get(name) {
            return auto;
        }

        let file = match std::fs::File::open(&self.path) {
            Ok(f) => f,
            Err(_) => return false,
        };

        for para in Deb822::iter_paragraphs_from_reader(std::io::BufReader::new(file)) {
            let Ok(para) = para else { continue };
            let Ok(entry) = ExtendedStateEntry::from_paragraph(&para) else {
                continue;
            };
            // apt parses `Auto-Installed` as an integer (`FindI` → `strtol`):
            // only numeric values count, and anything > 0 means auto.
            // `yes`/`no`/`true`/`false` parse as 0 → not auto, and apt
            // itself always writes `1`/`0`.
            //
            // See:
            // - https://salsa.debian.org/apt-team/apt/-/blob/main/apt-pkg/depcache.cc?ref_type=heads#L312
            //   (reading: `FindI("Auto-Installed", 0)` then `reason > 0`)
            // - https://salsa.debian.org/apt-team/apt/-/blob/main/apt-pkg/tagfile.cc?ref_type=heads#L761-L786
            //   (the `strtol` integer parse behind `FindI`)
            // - https://salsa.debian.org/apt-team/apt/-/blob/main/apt-pkg/depcache.cc?ref_type=heads#L407
            //   (writing: always `"1"` / `"0"`)
            let auto = entry
                .auto_installed
                .as_deref()
                .and_then(|v| v.trim().parse::<i64>().ok())
                .is_some_and(|n| n > 0);
            answers.insert(entry.package.clone(), auto);
            if entry.package == name {
                return auto;
            }
        }

        answers.insert(name.to_string(), false);
        false
    }
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
        // `yes` is not a number → strtol gives 0 → not auto (matches apt).
        assert!(!states.is_auto_installed("zsh"));
        // any integer > 0 counts as auto (matches apt's `reason > 0`).
        assert!(states.is_auto_installed("vim"));
        assert!(!states.is_auto_installed("nosuchpkg"));
    }
}
