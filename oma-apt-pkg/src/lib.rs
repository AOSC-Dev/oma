//! `oma-apt-pkg` — an arch-first view of APT's package state.
//!
//! [`AptState`] is the only entry point for package data: it loads the
//! `Packages` indexes and the live dpkg status file, and exposes packages by
//! name and architecture (APT's `Group` → `Package` → `Version` model).
//! Parsers and cache formats live under a private `internal` module.

pub mod apt_config;

#[cfg(feature = "apt-lists")]
mod api;
#[cfg(feature = "apt-lists")]
mod internal;

#[cfg(feature = "apt-lists")]
pub use api::{
    AptState, Arch, Control, Group, Package, PackageKey, QueryResolution, ResolvedPackage, Source,
    Version,
};
#[cfg(feature = "apt-lists")]
pub use internal::dpkg::DpkgError;
#[cfg(feature = "apt-lists")]
pub use internal::lists::AptListsError;

#[cfg(feature = "apt-lists")]
pub mod deb;

#[cfg(feature = "apt-sources")]
pub mod apt_sources;

pub mod error;
pub mod extended_states;
pub use apt_config::*;
pub use error::*;
pub use extended_states::AptExtendedStates;

#[cfg(feature = "filename")]
pub mod filename;

#[cfg(any(
    feature = "search-indicium",
    feature = "search-strsim",
    feature = "search-text"
))]
pub mod search;

#[cfg(feature = "filename")]
pub use filename::{AptListFilename, FilenameError, FilenameResult};
#[cfg(any(
    feature = "search-indicium",
    feature = "search-strsim",
    feature = "search-text"
))]
pub use search::*;
