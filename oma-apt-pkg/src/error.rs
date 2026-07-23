//! Crate-level error type for oma-apt-pkg.

use crate::apt_lists::AptListsError;
use crate::dpkg::DpkgError;

/// Errors that can occur in oma-apt-pkg operations.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// APT list file parsing failed.
    #[error("Failed to parse apt lists: {0}")]
    AptLists(#[from] AptListsError),
    /// dpkg status file parsing failed.
    #[error("Failed to parse dpkg status: {0}")]
    Dpkg(#[from] DpkgError),
}
