use std::fmt;

use debversion::Version;
use resolvo::utils::VersionSet;

use debian_control::relations::VersionConstraint;

/// A version constraint on a package, used as resolvo's `VersionSet`.
///
/// `None` matches any version; `Some((rel, ver))` matches versions satisfying
/// the deb822 relation against `ver` (compared with debversion semantics).
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum AptVersionSet {
    /// No constraint — any version matches.
    Any,
    /// A single relational constraint (e.g. `>= 2.0`).
    Constraint(VersionConstraint, String),
    /// Complement of `= ver` — everything except exactly `ver`.
    NotEqual(String),
    /// Matches nothing — the complement of [`AptVersionSet::Any`], used to
    /// express an unversioned `Breaks`/`Conflicts` exclusion.
    Empty,
}

impl VersionSet for AptVersionSet {
    type V = String;
}

impl AptVersionSet {
    /// Whether a version string matches this set, using debian version
    /// comparison.
    pub fn matches(&self, version: &str) -> bool {
        match self {
            Self::Any => true,
            Self::Empty => false,
            Self::Constraint(rel, want) => {
                let Some(v) = Version::parse_lenient(version).ok() else {
                    return false;
                };
                let Some(w) = Version::parse_lenient(want).ok() else {
                    return false;
                };
                match rel {
                    VersionConstraint::LessThan => v < w,
                    VersionConstraint::LessThanEqual => v <= w,
                    VersionConstraint::Equal => v == w,
                    VersionConstraint::GreaterThanEqual => v >= w,
                    VersionConstraint::GreaterThan => v > w,
                }
            }
            Self::NotEqual(want) => {
                let Some(v) = Version::parse_lenient(version).ok() else {
                    return false;
                };
                let Some(w) = Version::parse_lenient(want).ok() else {
                    return false;
                };
                v != w
            }
        }
    }

    /// The set of versions *not* matched by this set.
    ///
    /// Used to express `Breaks`/`Conflicts`: resolvo's `constrains` says "if
    /// this package is selected it must match this set", so an exclusion is
    /// encoded as the complement of the forbidden range.
    pub fn complement(&self) -> AptVersionSet {
        match self {
            Self::Any => Self::Empty,
            Self::Empty => Self::Any,
            Self::NotEqual(ver) => Self::Constraint(VersionConstraint::Equal, ver.clone()),
            Self::Constraint(rel, ver) => match rel {
                VersionConstraint::LessThan => {
                    Self::Constraint(VersionConstraint::GreaterThanEqual, ver.clone())
                }
                VersionConstraint::LessThanEqual => {
                    Self::Constraint(VersionConstraint::GreaterThan, ver.clone())
                }
                VersionConstraint::Equal => Self::NotEqual(ver.clone()),
                VersionConstraint::GreaterThanEqual => {
                    Self::Constraint(VersionConstraint::LessThan, ver.clone())
                }
                VersionConstraint::GreaterThan => {
                    Self::Constraint(VersionConstraint::LessThanEqual, ver.clone())
                }
            },
        }
    }
}

impl fmt::Display for AptVersionSet {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Any => write!(f, "(any)"),
            Self::Empty => write!(f, "(none)"),
            Self::NotEqual(ver) => write!(f, "(!= {ver})"),
            Self::Constraint(rel, ver) => write!(f, "({rel} {ver})"),
        }
    }
}
