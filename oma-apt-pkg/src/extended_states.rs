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

use std::collections::HashSet;
use std::path::Path;

use deb822_fast::{Deb822, FromDeb822, FromDeb822Paragraph};
use thiserror::Error;

/// Errors that can occur when reading APT extended states.
#[derive(Debug, Error)]
pub enum ExtendedStatesError {
    /// Failed to open the extended states file.
    #[error("Failed to open extended states file: {0}")]
    Io(#[from] std::io::Error),
    /// Failed to parse the extended states file.
    #[error("Failed to parse extended states file: {0}")]
    Deb822(#[from] deb822_fast::Error),
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
#[derive(Debug, Clone)]
pub struct AptExtendedStates {
    auto_installed: HashSet<String>,
}

impl AptExtendedStates {
    /// Parse the extended states file at the given path.
    ///
    /// The file is streamed paragraph by paragraph via
    /// [`Deb822::iter_paragraphs_from_reader`] — only one paragraph is in
    /// memory at a time instead of the whole file. Malformed paragraphs are
    /// skipped (best-effort), like elsewhere in this crate.
    pub fn from_file(path: impl AsRef<Path>) -> Result<Self, ExtendedStatesError> {
        let file = std::fs::File::open(path.as_ref())?;

        let auto_installed = Deb822::iter_paragraphs_from_reader(std::io::BufReader::new(file))
            .filter_map(|para| {
                let entry = ExtendedStateEntry::from_paragraph(&para.ok()?).ok()?;
                let is_auto = entry
                    .auto_installed
                    .as_deref()
                    .is_some_and(|v| v == "1" || v == "yes");
                if is_auto { Some(entry.package) } else { None }
            })
            .collect();

        Ok(Self { auto_installed })
    }

    /// Whether the given package was automatically installed as a dependency.
    pub fn is_auto_installed(&self, name: &str) -> bool {
        self.auto_installed.contains(name)
    }
}
