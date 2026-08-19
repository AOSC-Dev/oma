pub mod apt_config;
#[cfg(feature = "apt-lists")]
mod apt_db;
#[cfg(feature = "apt-lists")]
pub(crate) mod cache;
#[cfg(feature = "apt-lists")]
pub mod deb;
#[cfg(feature = "apt-lists")]
pub use deb::*;
#[cfg(feature = "apt-lists")]
pub mod package_matcher;
#[cfg(feature = "apt-lists")]
pub use package_matcher::*;
#[cfg(feature = "apt-lists")]
mod apt_lists;
#[cfg(feature = "apt-sources")]
pub mod apt_sources;
pub use apt_config::*;
#[cfg(feature = "apt-config")]
pub(crate) mod config_parser;
mod dpkg;
mod dpkg_state;
pub mod extended_states;
pub use extended_states::{AptExtendedStates, ExtendedStatesError};
pub mod error;

#[cfg(feature = "filename")]
pub mod filename;
#[cfg(any(
    feature = "search-indicium",
    feature = "search-strsim",
    feature = "search-text"
))]
pub mod search;

#[cfg(feature = "apt-lists")]
pub use apt_db::*;
#[cfg(feature = "apt-lists")]
pub use apt_lists::*;
pub use dpkg::*;
#[cfg(feature = "resolver")]
pub(crate) use dpkg_state::DpkgIndex;
pub use dpkg_state::*;
pub use error::*;

#[cfg(feature = "filename")]
pub use filename::{AptListFilename, FilenameError, FilenameResult};
#[cfg(any(
    feature = "search-indicium",
    feature = "search-strsim",
    feature = "search-text"
))]
pub use search::*;

#[cfg(feature = "resolver")]
pub mod apt_provider;
#[cfg(feature = "resolver")]
pub use apt_provider::*;
