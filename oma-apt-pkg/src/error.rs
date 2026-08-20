//! Crate-level error type for oma-apt-pkg.

#[cfg(feature = "apt-lists")]
use crate::apt_lists::AptListsError;
use crate::dpkg::DpkgError;

/// Errors that can occur in oma-apt-pkg operations.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// APT list file parsing failed.
    #[cfg(feature = "apt-lists")]
    #[error("Failed to parse apt lists: {0}")]
    AptLists(#[from] AptListsError),
    /// dpkg status file parsing failed.
    #[error("Failed to parse dpkg status: {0}")]
    Dpkg(#[from] DpkgError),
    /// APT-Sources formatting failed (e.g. missing architecture).
    #[error("{0}")]
    AptSources(String),
    /// SQLite (FTS5 search index) operation failed.
    #[cfg(feature = "search-fts")]
    #[error("Failed to operate SQLite search index: {0}")]
    Sqlite(#[from] rusqlite::Error),
}
