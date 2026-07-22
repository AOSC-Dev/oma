use std::collections::HashSet;
use std::path::Path;

use deb822_fast::{FromDeb822, FromDeb822Paragraph};
use deb822_lossless::Deb822;

/// Errors that can occur when parsing dpkg status files.
#[derive(Debug, thiserror::Error)]
pub enum DpkgError {
    #[error("Failed to read dpkg status file: {0}")]
    Io(#[from] std::io::Error),
    #[error("Failed to parse deb822 data: {0}")]
    Deb822(#[from] deb822_lossless::Error),
}

/// Represents the desired (or current) selection state of a package.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SelectionState {
    Install,
    Hold,
    Deinstall,
    Purge,
    Unknown,
}

impl SelectionState {
    fn from_status(status: &str) -> Self {
        // Status field format: "<selection> <eFlags> <eVersion>"
        // e.g. "install ok installed", "hold ok installed", "deinstall ok config-files"
        if let Some(first) = status.split_whitespace().next() {
            match first {
                "install" => Self::Install,
                "hold" => Self::Hold,
                "deinstall" => Self::Deinstall,
                "purge" => Self::Purge,
                _ => Self::Unknown,
            }
        } else {
            Self::Unknown
        }
    }

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
    #[deb822(field = "Auto-Installed", deserialize_with = parse_auto_installed)]
    pub auto_installed: Option<bool>,
}

fn parse_auto_installed(s: &str) -> Result<bool, String> {
    match s {
        "yes" | "1" | "true" => Ok(true),
        "no" | "0" | "false" => Ok(false),
        _ => Err(format!("invalid Auto-Installed value: {s}")),
    }
}

impl DpkgPackage {
    /// Derive selection state from the `Status` field.
    pub fn selection_state(&self) -> SelectionState {
        self.status
            .as_deref()
            .map(SelectionState::from_status)
            .unwrap_or(SelectionState::Unknown)
    }
}

/// Parse `/var/lib/dpkg/status` and return the set of installed package names.
///
/// A package is considered installed if its Status field starts with "install"
/// or "hold" (i.e. "install ok installed" or "hold ok installed").
pub fn parse_installed(path: impl AsRef<Path>) -> Result<HashSet<String>, DpkgError> {
    let deb822 = Deb822::from_file(path.as_ref())?;
    let mut installed = HashSet::new();

    for para in deb822.paragraphs() {
        let Some(status) = para.get("Status") else {
            continue;
        };
        if SelectionState::from_status(&status).is_installed() {
            if let Some(pkg) = para.get("Package") {
                installed.insert(pkg.to_string());
            }
        }
    }

    Ok(installed)
}

/// Parse `/var/lib/dpkg/status` and return full package information.
pub fn parse_dpkg_status(path: impl AsRef<Path>) -> Result<Vec<DpkgPackage>, DpkgError> {
    let deb822 = Deb822::from_file(path.as_ref())?;
    let mut packages = Vec::new();

    for para in deb822.paragraphs() {
        if let Ok(pkg) = DpkgPackage::from_paragraph(&para) {
            packages.push(pkg);
        }
    }

    Ok(packages)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_selection_state_from_status() {
        assert!(SelectionState::from_status("install ok installed").is_installed());
        assert!(SelectionState::from_status("hold ok installed").is_installed());
        assert!(!SelectionState::from_status("deinstall ok config-files").is_installed());
        assert!(!SelectionState::from_status("purge ok not-installed").is_installed());
    }

    #[test]
    fn test_parse_dpkg_status_with_all_fields() {
        let input = "\
Package: zoxide
Version: 0.9.6-1
Architecture: amd64
Status: install ok installed
Auto-Installed: yes

Package: vim
Version: 9.1.0
Architecture: amd64
Status: hold ok installed

Package: old-kernel
Version: 6.0.0
Status: deinstall ok config-files
Auto-Installed: yes

";
        let dir = std::env::temp_dir().join("test_dpkg_status");
        std::fs::create_dir_all(&dir).ok();
        let path = dir.join("status");
        std::fs::write(&path, input).unwrap();

        let packages = parse_dpkg_status(&path).unwrap();
        std::fs::remove_file(&path).ok();

        assert_eq!(packages.len(), 3);

        // zoxide: installed, auto
        assert_eq!(packages[0].name, "zoxide");
        assert_eq!(packages[0].version.as_deref(), Some("0.9.6-1"));
        assert_eq!(packages[0].architecture.as_deref(), Some("amd64"));
        assert!(packages[0].selection_state().is_installed());
        assert_eq!(packages[0].auto_installed, Some(true));

        // vim: hold, no auto_installed field
        assert_eq!(packages[1].name, "vim");
        assert!(packages[1].selection_state().is_installed());
        assert_eq!(packages[1].auto_installed, None);

        // old-kernel: deinstalled, auto flag preserved
        assert_eq!(packages[2].name, "old-kernel");
        assert!(!packages[2].selection_state().is_installed());
        assert_eq!(packages[2].auto_installed, Some(true));
    }

    #[test]
    fn test_parse_installed_only_installed() {
        let input = "\
Package: foo
Status: install ok installed

Package: bar
Status: deinstall ok config-files

Package: baz
Status: purge ok not-installed

";
        let dir = std::env::temp_dir().join("test_installed_set");
        std::fs::create_dir_all(&dir).ok();
        let path = dir.join("status");
        std::fs::write(&path, input).unwrap();

        let installed = parse_installed(&path).unwrap();
        std::fs::remove_file(&path).ok();

        assert!(installed.contains("foo"));
        assert!(!installed.contains("bar"));
        assert!(!installed.contains("baz"));
    }
}
