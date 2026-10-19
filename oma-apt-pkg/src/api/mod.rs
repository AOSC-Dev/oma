//! Public package API: the arch-first package state and its views.

pub(crate) mod arch;
pub(crate) mod model;
pub(crate) mod query;
pub(crate) mod state;

pub use arch::{Arch, PackageKey};
pub use model::{Group, Package, Version};
pub use query::{QueryResolution, ResolvedPackage};
pub use state::AptState;

pub use crate::internal::lists::{IndexSource as Source, PackageEntry as Control};
