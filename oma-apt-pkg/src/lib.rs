pub mod apt_config;
#[cfg(feature = "apt-lists")]
mod apt_db;
#[cfg(feature = "apt-lists")]
mod cache;
#[cfg(feature = "apt-lists")]
pub mod deb;
#[cfg(feature = "apt-lists")]
pub use deb::*;
#[cfg(feature = "apt-lists")]
pub mod apt_lists_reader;
#[cfg(feature = "apt-lists")]
pub use apt_lists_reader::*;
#[cfg(feature = "apt-lists")]
pub mod package_matcher;
#[cfg(feature = "apt-lists")]
pub use package_matcher::*;
mod apt_lists;
#[cfg(feature = "apt-sources")]
pub mod apt_sources;
pub use apt_config::*;
#[cfg(feature = "apt-config")]
pub(crate) mod config_parser;
mod dpkg;
mod dpkg_state;
pub mod extended_states;
pub use extended_states::AptExtendedStates;
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
pub use apt_lists::*;
pub use dpkg::*;
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
