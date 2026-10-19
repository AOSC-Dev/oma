//! Public package API: the arch-first package state and its views.

pub(crate) mod arch;
pub(crate) mod state;

pub use arch::{Arch, PackageKey};
pub use state::{
    AptState, Control, Group, Package, QueryResolution, ResolvedPackage, Source, Version,
};
