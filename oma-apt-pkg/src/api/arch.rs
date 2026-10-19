//! Architecture as a first-class package identity, the way APT models it.
//!
//! APT keeps one [`Group`](crate::state::Group) per package name, and one
//! package per architecture inside it: a package name alone is not unique.
//! `Architecture: all` is not a package architecture in APT — the version is
//! flagged `all`, while the package it belongs to is the native build
//! (`pkgCacheGenerator::NewPackage` maps `all` to `APT::Architecture`).

use std::borrow::Borrow;
use std::fmt;

/// A Debian architecture name, or one of APT's architecture selectors.
///
/// Selectors are kept distinct from concrete names; resolve them with
/// [`AptState`](crate::AptState) rather than comparing strings.
#[derive(Clone, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
#[cfg_attr(
    feature = "apt-lists",
    derive(rkyv::Archive, rkyv::Serialize, rkyv::Deserialize)
)]
#[cfg_attr(feature = "apt-lists", rkyv(derive(Hash, Eq, PartialEq)))]
pub struct Arch(String);

impl Arch {
    /// `Architecture: all` — a version property, not a package architecture.
    pub const ALL: &'static str = "all";
    /// APT's selector for this system's architecture.
    pub const NATIVE: &'static str = "native";
    /// APT's selector for the preferred package of a group.
    pub const ANY: &'static str = "any";

    /// Wrap a raw architecture word.
    pub fn new(arch: impl Into<String>) -> Self {
        Self(arch.into())
    }

    /// The raw architecture word.
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// Whether this is the `all` pseudo-architecture.
    pub fn is_all(&self) -> bool {
        self.0 == Self::ALL
    }

    /// Whether this is an APT selector (`native`/`any`) rather than a
    /// concrete architecture.
    pub fn is_selector(&self) -> bool {
        matches!(self.0.as_str(), Self::NATIVE | Self::ANY)
    }
}

impl fmt::Display for Arch {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl From<&str> for Arch {
    fn from(arch: &str) -> Self {
        Self::new(arch)
    }
}

impl From<String> for Arch {
    fn from(arch: String) -> Self {
        Self(arch)
    }
}

impl AsRef<str> for Arch {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

impl Borrow<str> for Arch {
    fn borrow(&self) -> &str {
        &self.0
    }
}

/// The identity of one APT package: a name and an architecture.
///
/// Two records with the same name but different architectures are different
/// packages (APT's `pkgCache::Package`); versions are attached to them.
#[derive(Clone, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct PackageKey {
    /// Package name (`pkgCache::Group::Name`).
    pub name: String,
    /// Package architecture, never `all` (see [`Arch`]).
    pub arch: Arch,
}

impl PackageKey {
    /// Build a key from its parts.
    pub fn new(name: impl Into<String>, arch: impl Into<Arch>) -> Self {
        Self {
            name: name.into(),
            arch: arch.into(),
        }
    }

    /// The package name.
    pub fn name(&self) -> &str {
        &self.name
    }

    /// The package architecture.
    pub fn arch(&self) -> &Arch {
        &self.arch
    }
}

impl fmt::Display for PackageKey {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}:{}", self.name, self.arch)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn selectors_and_all_are_distinct() {
        assert!(Arch::new("all").is_all());
        assert!(Arch::new("native").is_selector());
        assert!(Arch::new("any").is_selector());
        assert!(!Arch::new("amd64").is_selector());
        assert_eq!(Arch::new("amd64").to_string(), "amd64");
    }
}
