//! APT → resolvo adapter: expose an [`AptDb`] as a
//! [`resolvo::DependencyProvider`] so the CDCL solver can compute a set of
//! consistent package versions.
//!
//! Layout:
//! - [`version_set`] — the `AptVersionSet` version constraint used as
//!   resolvo's `VersionSet`
//! - [`provider`] — the `AptProvider`/`AptPool` resolver glue (internal)
//! - [`solve`] — entry points returning a consistent `(name, version)` set
//! - [`plan`] — `InstallItem` + dependency-ordered install plans
//! - [`transaction`] — the `TransactionPlanner` (install/remove/upgrade)
//! - [`change`] — the `Change`/`ChangeSet`/`Transaction` state model
//! - [`dpkg_plan`] — the `DpkgOp`/`DpkgPlan` dpkg execution plan
//!
//! Mapping (see the analysis in the repo docs):
//! - `Depends` / `Pre-Depends` → `KnownDependencies.requirements`
//! - `Recommends` → `requirements` (install-recommends policy)
//! - `Breaks` / `Conflicts` → `KnownDependencies.constrains`, encoded as the
//!   *complement* of the forbidden range; self-conflicts are dropped
//! - `Provides` → virtual package names whose `get_candidates` returns the
//!   providing packages
//!
//! This is experimental and the API is expected to evolve.

use thiserror::Error;

use crate::AptConfig;

mod change;
mod dpkg_plan;
/// EDSP (External Dependency Solving Protocol) — the protocol apt uses to
/// delegate dependency resolution to an external solver binary. See the
/// module docs for the wire format and the `oma-edsp` binary.
pub mod edsp;
mod executor;
mod lock;
mod plan;
mod provider;
mod solve;
#[cfg(test)]
mod tests;
mod transaction;
mod version_set;

// --- public API ---
pub use change::{Change, ChangeKind, ChangeSet, Transaction};
pub use dpkg_plan::{DpkgOp, DpkgPlan};
pub use executor::{DownloadList, Executor, ExecutorError, ExecutorLocks};
pub use lock::{LockError, LockGuard};
pub use plan::{InstallItem, resolve_install_order, resolve_install_order_with};
pub use solve::{SharedSolver, solve_packages, solve_requirements, solve_requirements_with};
pub use transaction::{TransactionPlanner, UpgradeMode};
pub use version_set::AptVersionSet;

// --- internal items shared between submodules ---
pub(crate) use plan::{Orderable, order_by_deps, resolve_plan};
pub(crate) use provider::AptProvider;
pub(crate) use solve::{dep_version_set, group_to_requirement, solve_with};

/// Errors that can occur while resolving a package set.
#[derive(Debug, Error)]
pub enum ResolveError {
    /// No consistent version set exists for the requested roots; the message
    /// describes the conflicting packages.
    #[error("unsolvable: {0}")]
    Unsolvable(String),
    /// The solver was cancelled before reaching a result.
    #[error("solver cancelled: {0}")]
    Cancelled(String),
    /// The plan would require removing an essential (protected) package,
    /// which apt refuses to do.
    #[error("cannot proceed: {0}")]
    Essential(String),
    /// The plan would require removing a protected package (`Protected: yes`,
    /// dpkg 1.19+), which apt refuses to do.
    #[error("cannot proceed: {0}")]
    Protected(String),
    /// The plan would require removing a held package (`Status: hold` in dpkg
    /// status), which apt refuses to do unless the user asks explicitly.
    #[error("cannot proceed: {0}")]
    Held(String),
}

/// Options controlling how a resolution is performed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ResolveOptions {
    /// Treat `Recommends` as required dependencies.
    ///
    /// Defaults to true, matching apt's `APT::Install-Recommends`. Set to
    /// false for a hard-only closure (`Pre-Depends` + `Depends`), comparable
    /// with `apt-cache depends --recurse --important`.
    pub install_recommends: bool,
    /// Treat `Suggests` as required dependencies.
    ///
    /// Defaults to false, matching apt's `APT::Install-Suggests` (apt does not
    /// install suggests by default).
    pub install_suggests: bool,
    /// Prefer keeping the currently installed version of a package when it
    /// satisfies the constraints (apt semantics), instead of always taking
    /// the newest candidate.
    ///
    /// Defaults to true. Only effective when the resolver is given the
    /// installed state (e.g. through [`TransactionPlanner`]).
    pub prefer_installed: bool,
}

/// Build options from apt configuration: reads `APT::Install-Recommends` and
/// `APT::Install-Suggests`.
impl From<&AptConfig> for ResolveOptions {
    fn from(cfg: &AptConfig) -> Self {
        Self {
            install_recommends: cfg.get_bool("APT::Install-Recommends", true),
            install_suggests: cfg.get_bool("APT::Install-Suggests", false),
            prefer_installed: true,
        }
    }
}

impl Default for ResolveOptions {
    fn default() -> Self {
        Self {
            install_recommends: true,
            install_suggests: false,
            prefer_installed: true,
        }
    }
}
