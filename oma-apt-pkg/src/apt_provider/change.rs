//! The state-change model of a transaction: what happens to each package
//! (the unordered mark set [`ChangeSet`]) and the same set ordered into a
//! dpkg-safe execution plan ([`Transaction`]).
//!
//! For the dpkg operation list (`Remv` / `Inst` / `Conf`) built from a
//! [`ChangeSet`], see [`super::dpkg_plan`].

use oma_fetch::Event;
use reqwest_middleware::ClientWithMiddleware;

use crate::{AptConfig, AptDb};

use super::dpkg_plan::{DpkgOp, DpkgPlan};
use super::{Executor, ExecutorError, Orderable, order_by_deps};

/// The kind of change a package undergoes in a transaction.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChangeKind {
    /// Not currently installed — will be installed.
    Install,
    /// Installed at an older version — will be upgraded.
    Upgrade,
    /// Installed at a newer version — will be downgraded.
    Downgrade,
    /// Installed at the same version — will be reinstalled anyway (as with
    /// `apt install foo --reinstall`).
    Reinstall,
    /// Will be removed.
    Remove,
}

/// A single package change in a transaction.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Change {
    pub kind: ChangeKind,
    pub package: String,
    /// Currently installed version, if any.
    pub from_version: Option<String>,
    /// Version that will be installed, if any.
    pub to_version: Option<String>,
    /// `Installed-Size` of the currently installed version (from the index),
    /// when that version is still present in the repository.
    pub old_size: Option<u64>,
    /// `Installed-Size` of the version being installed, when known.
    pub new_size: Option<u64>,
    /// Download `Size` of the version being installed, when known.
    pub download_size: Option<u64>,
    /// Hard dependencies within the same change set (already concrete plan
    /// entries). Used by the order phase ([`ChangeSet::into_transaction`])
    /// to topologically sort install-side changes, mirroring apt's
    /// `pkgOrderList`. Empty for removals.
    pub depends_on: Vec<String>,
    /// Whether the resolver pulled this package in as a dependency rather
    /// than it being an explicitly requested root — apt's `Auto-Installed`
    /// flag. Used after a successful install to record which packages were
    /// auto-installed (so they can be autoremoved later). Only meaningful for
    /// install-side changes; always `false` for removals.
    pub auto_installed: bool,
}

impl Change {
    /// Installed size after applying this change — the `Installed-Size` of
    /// the version being installed. `None` for removals (the package is gone)
    /// or when the size is not in the index.
    pub fn installed_size(&self) -> Option<u64> {
        self.new_size
    }

    /// Whether this change installs a package automatically, as a dependency
    /// pulled in by the resolver (not explicitly requested by the user) —
    /// apt's `Auto-Installed` flag.
    pub fn is_auto_installed(&self) -> bool {
        self.auto_installed
    }

    /// Download size of the version being installed. `None` for removals or
    /// when the size is not in the index.
    pub fn download_size(&self) -> Option<u64> {
        self.download_size
    }
}

impl Orderable for Change {
    fn order_name(&self) -> &str {
        &self.package
    }

    fn order_deps(&self) -> &[String] {
        &self.depends_on
    }
}

/// An unordered set of package changes — the *mark* phase, mirroring apt's
/// `pkgDepCache` marking.
///
/// The resolver decides *what* happens to each package (install / upgrade /
/// downgrade / reinstall / remove) without committing to an execution order.
/// The marks themselves are **unordered**, exactly like apt's `MarkInstall`;
/// the dependency order is computed later, in the order phase
/// ([`ChangeSet::into_transaction`]), the same way apt's `pkgOrderList` runs
/// only when the execution plan is built.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ChangeSet {
    /// The decided changes; installs/upgrades/downgrades first (dependency
    /// order), removals appended.
    pub changes: Vec<Change>,
}

impl ChangeSet {
    /// All marks in this set.
    pub fn get_changes(&self) -> &[Change] {
        &self.changes
    }

    pub fn is_empty(&self) -> bool {
        self.changes.is_empty()
    }

    pub fn len(&self) -> usize {
        self.changes.len()
    }

    /// `Install` marks (packages not currently installed).
    pub fn installs(&self) -> impl Iterator<Item = &Change> {
        self.of_kind(ChangeKind::Install)
    }

    /// `Upgrade` marks (installed at an older version).
    pub fn upgrades(&self) -> impl Iterator<Item = &Change> {
        self.of_kind(ChangeKind::Upgrade)
    }

    /// `Downgrade` marks (installed at a newer version).
    pub fn downgrades(&self) -> impl Iterator<Item = &Change> {
        self.of_kind(ChangeKind::Downgrade)
    }

    /// `Reinstall` marks (installed at the same version, will be reinstalled).
    pub fn reinstalls(&self) -> impl Iterator<Item = &Change> {
        self.of_kind(ChangeKind::Reinstall)
    }

    /// `Remove` marks.
    pub fn removals(&self) -> impl Iterator<Item = &Change> {
        self.of_kind(ChangeKind::Remove)
    }

    /// Marks of exactly the given kind.
    pub fn of_kind(&self, kind: ChangeKind) -> impl Iterator<Item = &Change> {
        self.changes.iter().filter(move |c| c.kind == kind)
    }

    /// Order the marks into a [`Transaction`] — the state changes in
    /// dpkg-safe order: `Remove` marks first (conflicting/obsolete packages
    /// must be gone before anything is unpacked), then the install-side
    /// changes in dependency order, computed here (apt's `pkgOrderList` runs
    /// at this same execution-plan stage, not at mark time).
    pub fn into_transaction(self) -> Transaction {
        let mut removals = Vec::new();
        let mut install_side = Vec::new();
        for change in self.changes {
            match change.kind {
                ChangeKind::Remove => removals.push(change),
                ChangeKind::Install
                | ChangeKind::Upgrade
                | ChangeKind::Downgrade
                | ChangeKind::Reinstall => install_side.push(change),
            }
        }

        // Order phase: order the install-side marks by their hard-dependency
        // edges, like apt's `pkgOrderList` — consumes the marks and returns
        // them reordered (no clones).
        let install_side = order_by_deps(install_side);

        let mut changes = Vec::with_capacity(removals.len() + install_side.len());
        changes.extend(removals);
        changes.extend(install_side);

        Transaction { changes }
    }
}

/// The state changes of a transaction in dpkg-safe execution order (built
/// from a [`ChangeSet`] via [`ChangeSet::into_transaction`]).
///
/// This describes *what happens* to each package (its before/after state
/// change). For the dpkg operation list (`Remv` / `Inst` / `Conf`), see
/// [`DpkgPlan`].
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Transaction {
    /// All changes, in execution order.
    pub changes: Vec<Change>,
}

impl Transaction {
    /// All changes in this transaction, in execution order.
    pub fn get_changes(&self) -> &[Change] {
        &self.changes
    }

    pub fn is_empty(&self) -> bool {
        self.changes.is_empty()
    }

    pub fn len(&self) -> usize {
        self.changes.len()
    }

    /// `Install` changes in execution order.
    pub fn installs(&self) -> impl Iterator<Item = &Change> {
        self.of_kind(ChangeKind::Install)
    }

    /// `Upgrade` changes in execution order.
    pub fn upgrades(&self) -> impl Iterator<Item = &Change> {
        self.of_kind(ChangeKind::Upgrade)
    }

    /// `Downgrade` changes in execution order.
    pub fn downgrades(&self) -> impl Iterator<Item = &Change> {
        self.of_kind(ChangeKind::Downgrade)
    }

    /// `Reinstall` changes in execution order.
    pub fn reinstalls(&self) -> impl Iterator<Item = &Change> {
        self.of_kind(ChangeKind::Reinstall)
    }

    /// `Remove` changes in execution order.
    pub fn removals(&self) -> impl Iterator<Item = &Change> {
        self.of_kind(ChangeKind::Remove)
    }

    /// Changes of exactly the given kind, in execution order.
    pub fn of_kind(&self, kind: ChangeKind) -> impl Iterator<Item = &Change> {
        self.changes.iter().filter(move |c| c.kind == kind)
    }

    /// The names of every package this transaction auto-installs — pulled in
    /// as a dependency by the resolver, not explicitly requested. Feed these
    /// to [`crate::AptExtendedStates::mark_auto`] (or the
    /// [`crate::apt_provider::Executor`], which does it automatically) after
    /// the install succeeds. Borrows the transaction — no allocation.
    pub fn auto_installed_names(&self) -> impl Iterator<Item = &str> {
        self.changes
            .iter()
            .filter(|c| c.auto_installed)
            .map(|c| c.package.as_str())
    }

    /// Build the dpkg operation list — *what dpkg does* with each package —
    /// in the order `apt install --dry-run` reports it (`Remv`, then `Inst`,
    /// then `Conf`).
    ///
    /// This is a separate concern from the state changes in [`Change`]: a
    /// `Remove` change becomes a single `dpkg --remove`, while every
    /// install-side change becomes two operations — `dpkg --unpack` then
    /// `dpkg --configure`.
    ///
    /// The changes are already in execution order (built by
    /// [`ChangeSet::into_transaction`], apt's `pkgOrderList`), so this only
    /// groups them into operations — no re-ordering, no index access. Borrows
    /// the transaction rather than consuming it, so the same [`Transaction`]
    /// also feeds the download flow (see
    /// [`Executor::build_download_list`](crate::apt_provider::Executor::build_download_list))
    /// and the UI — no clone needed.
    pub fn to_dpkg_plan(&self) -> DpkgPlan<'_> {
        let mut ops = Vec::new();
        // Removals first, then every install-side package unpacked in
        // dependency order, then the same set configured — apt's list order.
        // Operations borrow from `self.changes` — nothing is cloned.
        ops.extend(
            self.changes
                .iter()
                .filter(|c| c.kind == ChangeKind::Remove)
                .map(|c| DpkgOp::Remove {
                    package: c.package.as_str(),
                    version: c.from_version.as_deref(),
                }),
        );

        let install_side = self.changes.iter().filter(|c| c.kind != ChangeKind::Remove);

        ops.extend(install_side.clone().map(|c| DpkgOp::Unpack {
            package: c.package.as_str(),
            version: c.to_version.as_deref(),
        }));

        ops.extend(install_side.map(|c| DpkgOp::Configure {
            package: c.package.as_str(),
            version: c.to_version.as_deref(),
        }));

        DpkgPlan { ops }
    }

    /// Commit phase: apply the whole transaction — build an [`Executor`]
    /// from `cfg` (dpkg root from `RootDir`, download directory from
    /// `Dir::Cache::archives`, auto-installed records from
    /// `Dir::State::extended_states`) with `client`, download every `.deb`
    /// with `oma-fetch`, run the dpkg plan, and record which packages were
    /// auto-installed. Mirrors apt's `pkgCommit`; the work itself is
    /// [`Executor::execute`].
    pub fn commit(
        self,
        index: &AptDb,
        cfg: &AptConfig,
        client: ClientWithMiddleware,
        callback: impl FnMut(Event),
    ) -> Result<(), ExecutorError> {
        // Takes the locks itself (None); callers that want the locks held
        // from before the review prompt use [`Executor::lock`] + execute.
        // Consumes the transaction — the whole execution is one step.
        Executor::from_config(cfg, client).execute(index, self, None, callback)
    }
}
