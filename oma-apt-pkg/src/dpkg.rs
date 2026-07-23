use std::path::Path;

use deb822_fast::{Deb822, FromDeb822, FromDeb822Paragraph};

/// Errors that can occur when parsing dpkg status files.
#[derive(Debug, thiserror::Error)]
pub enum DpkgError {
    #[error("Failed to read dpkg status file: {0}")]
    Io(#[from] std::io::Error),
    #[error("Failed to parse deb822 data: {0}")]
    Deb822(#[from] deb822_fast::Error),
}

/// Represents the desired (or current) selection state of a package.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SelectionState {
    Unknown,
    Install,
    Hold,
    Deinstall,
    Purge,
}

impl SelectionState {
    /// Returns true if the package is marked as installed (install or hold).
    pub fn is_installed(&self) -> bool {
        matches!(self, Self::Install | Self::Hold)
    }
}

/// Information about a single package from dpkg status.
#[derive(Debug, Clone, FromDeb822)]
pub struct DpkgPackage {
    #[deb822(field = "Package")]
    pub name: String,
    #[deb822(field = "Version")]
    pub version: Option<String>,
    #[deb822(field = "Architecture")]
    pub architecture: Option<String>,
    #[deb822(field = "Status")]
    pub status: Option<String>,
}

impl DpkgPackage {
    /// Derive selection state from the `Status` field.
    pub fn selection_state(&self) -> SelectionState {
        let s = match self.status.as_deref() {
            Some(s) => s,
            None => return SelectionState::Unknown,
        };
        if s.starts_with("install") {
            SelectionState::Install
        } else if s.starts_with("hold") {
            SelectionState::Hold
        } else if s.starts_with("deinstall") {
            SelectionState::Deinstall
        } else if s.starts_with("purge") {
            SelectionState::Purge
        } else {
            SelectionState::Unknown
        }
    }
}
/// Parse `/var/lib/dpkg/status` and return full package information.
pub fn parse_dpkg_status(path: impl AsRef<Path>) -> Result<Vec<DpkgPackage>, DpkgError> {
    let file = std::fs::File::open(path.as_ref())?;
    let deb822 = Deb822::from_reader(file)?;

    deb822
        .iter()
        .map(|para| {
            DpkgPackage::from_paragraph(para)
                .map_err(|e| DpkgError::Deb822(deb822_fast::Error::Io(std::io::Error::other(e))))
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_selection_state_from_status() {
        let pkg = |status: &str| -> SelectionState {
            DpkgPackage {
                name: "pkg".into(),
                version: None,
                architecture: None,
                status: Some(status.into()),
            }
            .selection_state()
        };

        assert!(pkg("install ok installed").is_installed());
        assert!(pkg("hold ok installed").is_installed());
        assert!(!pkg("deinstall ok config-files").is_installed());
        assert!(!pkg("purge ok not-installed").is_installed());
        assert!(!pkg("unknown ok not-installed").is_installed());
        assert!(!pkg("").is_installed());
    }

    #[test]
    fn test_parse_dpkg_status_with_all_fields() {
        let input = "\
Package: zoxide
Version: 0.9.6-1
Architecture: amd64
Status: install ok installed

Package: vim
Version: 9.1.0
Architecture: amd64
Status: hold ok installed

Package: old-kernel
Version: 6.0.0
Status: deinstall ok config-files

";
        let dir = std::env::temp_dir().join("test_dpkg_status");
        std::fs::create_dir_all(&dir).ok();
        let path = dir.join("status");
        std::fs::write(&path, input).unwrap();

        let packages = parse_dpkg_status(&path).unwrap();
        std::fs::remove_file(&path).ok();

        assert_eq!(packages.len(), 3);

        assert_eq!(packages[0].name, "zoxide");
        assert_eq!(packages[0].version.as_deref(), Some("0.9.6-1"));
        assert_eq!(packages[0].architecture.as_deref(), Some("amd64"));
        assert!(packages[0].selection_state().is_installed());

        assert_eq!(packages[1].name, "vim");
        assert!(packages[1].selection_state().is_installed());

        assert_eq!(packages[2].name, "old-kernel");
        assert!(!packages[2].selection_state().is_installed());
    }
}
