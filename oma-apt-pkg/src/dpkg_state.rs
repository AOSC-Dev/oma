//! Real-time dpkg status — parsed on every access (fast, single file).

use std::collections::{HashMap, HashSet};
use std::path::Path;

use crate::dpkg::parse_dpkg_status;

/// Freshly-parsed dpkg status information.
///
/// Always parsed from `/var/lib/dpkg/status` — no caching since it's a single
/// small file and changes frequently (after every package operation).
#[derive(Debug, Clone)]
pub struct DpkgState {
    /// Set of installed package names.
    pub installed: HashSet<String>,
    /// Map from installed package name to its version string.
    pub installed_versions: HashMap<String, String>,
}

impl DpkgState {
    /// Parse dpkg status under the given sysroot.
    pub fn from_sysroot(sysroot: impl AsRef<Path>) -> Result<Self, String> {
        let dpkg_packages =
            parse_dpkg_status(sysroot.as_ref().join("var/lib/dpkg/status"))
                .map_err(|e| format!("Failed to parse dpkg status: {e}"))?;

        let installed: HashSet<String> = dpkg_packages
            .iter()
            .filter(|p| p.selection_state().is_installed())
            .map(|p| p.name.clone())
            .collect();

        let installed_versions: HashMap<String, String> = dpkg_packages
            .iter()
            .filter_map(|p| {
                p.selection_state()
                    .is_installed()
                    .then(|| (p.name.clone(), p.version.clone().unwrap_or_default()))
            })
            .collect();

        Ok(Self {
            installed,
            installed_versions,
        })
    }

    /// Whether a package is installed.
    pub fn is_installed(&self, name: &str) -> bool {
        self.installed.contains(name)
    }

    /// The installed version of a package, if any.
    pub fn installed_version(&self, name: &str) -> Option<&str> {
        self.installed_versions.get(name).map(|s| s.as_str())
    }
}
