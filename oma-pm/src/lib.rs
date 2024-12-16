//! # oma-pm
//!
//! The package manager component for oma.
//!
//! The oma-pm crate serves as the main library for oma,
//! providing essential functionalities for package management operations such as install, remove, upgrade, and search.
//!
//! ## Features
//!
//! 1. **PackageInfo**: Provides definitions of package information and handle them to `oma-apt`.
//! 2. **Progress**: Tracks the progress of package management operations.
//! 3. **Search Result**: Defines the structure and handling of search results.
//! 4. **Communication with rust-apt**: Communicate with `oma-apt`.
//!
//! NOTE: `oma-apt` is another fork of `rust-apt` maintained by AOSC-Dev
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
//! - `AptErrors`: Error definitions from `oma_apt` crate.
//! - `PkgCurrentState`: Current state definitions from `oma_apt` crate.
//! - `PackageStatus`: Status definitions from the `search` module.

pub mod apt;
pub mod matches;
pub mod pkginfo;
pub mod progress;
pub mod search;
pub use oma_apt::error::AptErrors;
pub use oma_apt::PkgCurrentState;
pub use search::PackageStatus;
mod dbus;

#[cfg(test)]
mod test {
    use std::sync::{LazyLock, Mutex};
    pub(crate) static TEST_LOCK: LazyLock<Mutex<()>> = LazyLock::new(|| Mutex::new(()));
}
