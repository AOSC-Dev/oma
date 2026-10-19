//! Parsers and caches used by the crate. Nothing here is exported directly:
//! callers work through the public API.

#[cfg(feature = "apt-lists")]
pub(crate) mod cache;
#[cfg(feature = "apt-config")]
pub(crate) mod config_parser;
#[cfg_attr(not(feature = "apt-lists"), allow(dead_code))]
pub(crate) mod dpkg;
#[cfg(feature = "apt-lists")]
pub(crate) mod lists;
#[cfg(feature = "apt-lists")]
pub(crate) mod tmp;
