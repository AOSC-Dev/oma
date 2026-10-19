//! Parsers and caches used by the public API. Nothing here is exported from
//! the crate root: callers work through [`AptState`](crate::AptState).

pub(crate) mod cache;
#[cfg(feature = "apt-config")]
pub(crate) mod config_parser;
pub(crate) mod dpkg;
pub(crate) mod lists;
