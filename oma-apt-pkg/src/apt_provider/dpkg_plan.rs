//! The dpkg execution plan — *what dpkg does* with each package (`Remv` /
//! `Inst` / `Conf`), in the order `apt install --dry-run` prints it.
//!
//! Built from a [`Transaction`] via [`Transaction::to_dpkg_plan`]; see
//! [`super::change`] for the state-change model.

/// A dpkg operation applied to one package — *what dpkg does* with it,
/// matching a line of `apt install --dry-run` (`Remv` / `Inst` / `Conf`).
///
/// This is deliberately separate from [`crate::apt_provider::Change`], which
/// describes a package's before/after state change: a dpkg operation carries
/// only what the dpkg invocation needs (package + version), not the semantic
/// change kind.
///
/// Operations borrow from the [`Transaction`](crate::apt_provider::Transaction)
/// they were built from, so building a plan clones nothing.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DpkgOp<'a> {
    /// `dpkg --remove`: uninstall the package, before anything is unpacked.
    Remove {
        package: &'a str,
        /// The currently installed version being removed, when known.
        version: Option<&'a str>,
    },
    /// `dpkg --unpack`: unpack the package's files, in dependency order.
    Unpack {
        package: &'a str,
        /// The version being unpacked, when known.
        version: Option<&'a str>,
    },
    /// `dpkg --configure`: run the package's configure scripts, in dependency
    /// order, after all unpacks.
    Configure {
        package: &'a str,
        /// The version being configured, when known.
        version: Option<&'a str>,
    },
}

/// The dpkg operation sequence to reach the desired state — the list order
/// `apt install --dry-run` prints: all removals first, then every install-side
/// package unpacked in dependency order, then the same set configured in
/// dependency order. Apply it with [`DpkgPlan::commit`].
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct DpkgPlan<'a> {
    /// Operations in execution order.
    pub ops: Vec<DpkgOp<'a>>,
}

impl<'a> DpkgPlan<'a> {
    /// All operations in this plan, in execution order.
    pub fn ops(&self) -> &[DpkgOp<'a>] {
        &self.ops
    }

    pub fn is_empty(&self) -> bool {
        self.ops.is_empty()
    }

    pub fn len(&self) -> usize {
        self.ops.len()
    }

    /// `dpkg --remove` operations, in execution order.
    pub fn removals(&self) -> impl Iterator<Item = &DpkgOp<'a>> {
        self.ops
            .iter()
            .filter(|op| matches!(op, DpkgOp::Remove { .. }))
    }

    /// `dpkg --unpack` operations, in dependency order.
    pub fn unpacks(&self) -> impl Iterator<Item = &DpkgOp<'a>> {
        self.ops
            .iter()
            .filter(|op| matches!(op, DpkgOp::Unpack { .. }))
    }

    /// `dpkg --configure` operations, in dependency order.
    pub fn configures(&self) -> impl Iterator<Item = &DpkgOp<'a>> {
        self.ops
            .iter()
            .filter(|op| matches!(op, DpkgOp::Configure { .. }))
    }
}
