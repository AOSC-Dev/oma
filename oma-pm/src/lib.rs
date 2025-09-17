//! # oma-pm
//!
//! The package manager component for oma.
//!
//! The oma-pm crate provides essential functionalities for
//! package management operations such as install, remove, upgrade, and search.
//!
//! ## Features
//!
//! 1. **PackageInfo**: Package information to be handled by oma-apt.
//! 2. **Progress**: Reports the progress of package management operations.
//! 3. **Search Result**: Structure and handling of search results.
//! 4. **Communication with rust-apt**: Communicate with `oma-apt`.
//!
//! NOTE: `oma-apt` is another fork of `rust-apt`, maintained by AOSC-Dev
//!
//! ## Modules
//!
//! - `apt`: Handles interactions with `apt`.
//! - `matches`: Provides utilities for matching package information.
//! - `pkginfo`: Contains definitions and structures for package information.
//! - `progress`: Tracks the progress of package management operations.
//! - `search`: Defines the structure and handling of search results.
//! - `dbus`: Manages D-Bus communication.
//!
//! ## Re-exports
//!
//! - `AptErrors`: Error definitions from `oma-apt` crate.
//! - `PkgCurrentState`: Current package management status from oma-apt.
//! - `PackageStatus`: Package status definitions from the `search` module.

pub mod apt;
pub mod matches;
pub mod pkginfo;
pub mod progress;
pub mod search;
pub use search::PackageStatus;
mod commit;
mod dbus;
mod download;
pub mod utils;
pub use commit::CommitNetworkConfig;
pub mod sort;
pub use commit::CustomDownloadMessage;
pub use oma_apt;
mod dpkg;

#[cfg(test)]
mod test {
    use std::sync::{LazyLock, Mutex};
    pub(crate) static TEST_LOCK: LazyLock<Mutex<()>> = LazyLock::new(|| Mutex::new(()));
}
