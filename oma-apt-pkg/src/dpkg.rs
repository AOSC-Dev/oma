use std::collections::HashSet;
use std::path::Path;

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
#[derive(Debug, Clone)]
pub struct DpkgPackage {
    pub name: String,
    pub version: Option<String>,
    pub architecture: Option<String>,
    pub status: Option<String>,
    pub selection_state: SelectionState,
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
        let name = match para.get("Package") {
            Some(n) => n.to_string(),
            None => continue,
        };

        let status_raw = para.get("Status");
        let selection_state = status_raw
            .as_deref()
            .map(SelectionState::from_status)
            .unwrap_or(SelectionState::Unknown);

        packages.push(DpkgPackage {
            name,
            version: para.get("Version"),
            architecture: para.get("Architecture"),
            status: status_raw,
            selection_state,
        });
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
}
