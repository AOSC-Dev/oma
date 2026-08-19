//! The transaction planner: the engine that turns user intent (install /
//! remove / upgrade marks) into a [`ChangeSet`], mirroring apt's two-phase
//! mark/resolve model.
//!
//! The state-change model (`Change` / `ChangeSet` / `Transaction`) lives in
//! `super::change`; the dpkg operation plan (`DpkgOp` / `DpkgPlan`) in
//! `super::dpkg_plan`.

use std::collections::{HashMap, HashSet};

use debversion::Version;
use regex::Regex;

use crate::{AptDb, DpkgState, PackageEntry, ParsedDeps, RelationExt};
use debian_control::relations::VersionConstraint;

use super::change::{Change, ChangeKind, ChangeSet};
use super::{AptVersionSet, InstallItem, ResolveError, ResolveOptions, SharedSolver, resolve_plan};

/// The three apt upgrade modes, controlling what a mass upgrade may do
/// beyond upgrading already-installed packages.
///
/// Mirrors apt's `apt full-upgrade` / `apt upgrade` / `apt-get upgrade`
/// (the `APT::Get::Upgrade-Allow-New` and `APT::Get::Upgrade-Allow-Remove`
/// configuration knobs):
///
/// | mode             | may install new | may remove |
/// |------------------|-----------------|------------|
/// | `FullUpgrade`    | yes             | yes        |
/// | `SafeUpgrade`    | yes             | no         |
/// | `MinimalUpgrade` | no              | no         |
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UpgradeMode {
    /// `apt full-upgrade` (apt-get `dist-upgrade`): may install new packages
    /// and remove installed ones.
    FullUpgrade,
    /// `apt upgrade` (safe-upgrade): may install new packages (e.g. new
    /// dependencies) but never removes installed packages.
    SafeUpgrade,
    /// `apt-get upgrade` (classic): only upgrades installed packages — never
    /// installs new packages and never removes installed ones. Upgrades that
    /// would need to are held back.
    MinimalUpgrade,
}

/// `Installed-Size` and download `Size` for an exact version.
///
/// This deliberately does **not** fall back to the candidate's sizes when the
/// exact version is absent from the index (e.g. an installed version no
/// longer in the repo) — that would report the wrong version's sizes.
fn size_fields(index: &AptDb, name: &str, version: &str) -> (Option<u64>, Option<u64>) {
    match index.get_version(name, version) {
        Some(entry) => (entry.entry.installed_size, entry.entry.size),
        None => (None, None),
    }
}

/// Plans package transactions (install / remove / upgrade) against an
/// [`AptDb`] and a [`DpkgState`].
///
/// Mirrors apt's two-phase model: the caller resolves the *candidate* — the
/// version to install — *before* marking (apt's `TryToInstall` finds the
/// candidate, then `MarkInstall` installs it), and passes it as a
/// [`PackageEntry`] to [`TransactionPlanner::mark_install`];
/// [`TransactionPlanner::mark_remove`] records removals. [`Self::resolve`]
/// (or [`Self::upgrade`], which marks every outdated installed package) then
/// runs the dependency resolution and *consumes* the pending marks, so the
/// same planner can be reused for the next transaction (apt's
/// `pkgProblemResolver::Resolve`).
///
/// The `marked_*` methods ([`Self::marked_install`],
/// [`Self::marked_new_install`], [`Self::marked_reinstall`],
/// [`Self::marked_remove`]) are read-only predicates over the pending marks,
/// mirroring apt's `PkgIterator::marked_*` (used e.g. to tell a fresh
/// install from an upgrade when presenting a transaction).
pub struct TransactionPlanner<'a> {
    index: &'a AptDb,
    dpkg: &'a DpkgState,
    /// How install marks are resolved — fixed for the whole transaction,
    /// like apt's config.
    options: ResolveOptions,
    /// Pending install marks (user intent: the candidate entries the caller
    /// resolved before marking) — an installed package may be upgraded to the
    /// marked version. Consumed by [`Self::resolve`].
    install: Vec<PackageEntry>,
    /// Pending *reinstall* marks (user intent, `pkg --reinstall`) — a package
    /// already installed at the same version is reinstalled. Consumed by
    /// [`Self::resolve`].
    reinstall: Vec<PackageEntry>,
    /// Pending removal marks (user intent), consumed by [`Self::resolve`].
    remove: Vec<PackageEntry>,
}

impl<'a> TransactionPlanner<'a> {
    /// Create a planner over `index` (repository state), `dpkg` (currently
    /// installed state) and the `options` that govern how install marks are
    /// resolved.
    pub fn new(index: &'a AptDb, dpkg: &'a DpkgState, options: ResolveOptions) -> Self {
        Self {
            index,
            dpkg,
            options,
            install: Vec::new(),
            reinstall: Vec::new(),
            remove: Vec::new(),
        }
    }

    /// The repository index this planner reads package data from.
    pub fn index(&self) -> &'a AptDb {
        self.index
    }

    /// The installed state this planner diffs against.
    pub fn dpkg(&self) -> &'a DpkgState {
        self.dpkg
    }

    /// Mark phase: record `target` — the package's candidate entry, resolved
    /// by the caller *before* marking (apt's `TryToInstall` finds the
    /// candidate, then `MarkInstall(Pkg)` installs it) — for installation at
    /// its version. No dependency resolution happens here — the solver runs
    /// only in [`Self::resolve`]. Mirrors apt's `pkgDepCache::MarkInstall`;
    /// call again for more packages, the marks accumulate. If the package is
    /// already installed, it is upgraded (or downgraded) to that version.
    ///
    /// With `reinstall` set (apt's `--reinstall`), a package already
    /// installed at the same version is marked for reinstall instead of being
    /// left untouched — it resolves to a [`ChangeKind::Reinstall`] change.
    pub fn mark_install(&mut self, target: PackageEntry, reinstall: bool) -> &mut Self {
        if reinstall {
            self.reinstall.push(target);
        } else {
            self.install.push(target);
        }
        self
    }

    /// Mark phase: record `target` for removal. No resolution happens here —
    /// applied in [`Self::resolve`]. Mirrors apt's `pkgDepCache::MarkDelete`;
    /// call again for more packages, the marks accumulate.
    pub fn mark_remove(&mut self, target: PackageEntry) -> &mut Self {
        self.remove.push(target);
        self
    }

    /// Whether `name` is pending as an install mark (recorded via
    /// [`Self::mark_install`]), mirroring apt's `PkgIterator::marked_install`.
    pub fn marked_install(&self, name: &str) -> bool {
        self.install.iter().any(|e| e.package == name)
    }

    /// Whether `name` is pending as a *fresh* install mark: it is marked for
    /// install and is not currently installed. Mirrors apt's
    /// `PkgIterator::marked_new_install`, which the UI uses to tell a fresh
    /// install from an upgrade.
    pub fn marked_new_install(&self, name: &str) -> bool {
        self.marked_install(name) && !self.dpkg.is_installed(name)
    }

    /// Whether `name` is pending as a *reinstall* mark (recorded via
    /// [`Self::mark_install`] with `reinstall: true`), mirroring apt's
    /// `PkgIterator::marked_reinstall`.
    pub fn marked_reinstall(&self, name: &str) -> bool {
        self.reinstall.iter().any(|e| e.package == name)
    }

    /// Whether `name` is pending as a removal mark (recorded via
    /// [`Self::mark_remove`]), mirroring apt's `PkgIterator::marked_delete`.
    pub fn marked_remove(&self, name: &str) -> bool {
        self.remove.iter().any(|e| e.package == name)
    }

    /// Clear every pending mark (install / reinstall / remove) without
    /// resolving — the planner returns to a clean state, so a fresh
    /// transaction can be marked. [`Self::resolve`] also consumes the marks,
    /// but this drops them without producing a [`ChangeSet`].
    pub fn clear_marked(&mut self) -> &mut Self {
        self.install.clear();
        self.reinstall.clear();
        self.remove.clear();
        self
    }

    /// Resolve phase — the single entry point that starts dependency
    /// resolution: solve the accumulated install marks against the installed
    /// state (adding the removals the solution requires: conflicting
    /// packages, their reverse hard-dependents, essential/protected
    /// protection) and apply the accumulated removal marks (reverse
    /// hard-dependency closure). Each install mark is the candidate entry the
    /// caller resolved before marking, so its version is pinned. **Consumes
    /// the pending marks**, so the planner can be reused for a fresh
    /// transaction. Returns the unordered [`ChangeSet`];
    /// [`ChangeSet::into_transaction`] orders it for dpkg. Mirrors apt's
    /// `pkgProblemResolver::Resolve`.
    pub fn resolve(&mut self) -> Result<ChangeSet, ResolveError> {
        // Take the pending marks: resolve consumes them, so the same planner
        // can be reused for the next transaction.
        let install = std::mem::take(&mut self.install);
        let reinstall = std::mem::take(&mut self.reinstall);
        let remove = std::mem::take(&mut self.remove);

        let mut changes = Vec::new();
        if !install.is_empty() || !reinstall.is_empty() {
            // Reinstall marks force a `Reinstall` change for a package that
            // is already present at the selected version; remember which
            // roots asked for it.
            let reinstall_names: HashSet<String> =
                reinstall.iter().map(|e| e.package.clone()).collect();
            let mut roots = install;
            roots.extend(reinstall);
            changes.extend(self.resolve_install(roots, &reinstall_names)?);
        }
        if !remove.is_empty() {
            changes.extend(self.resolve_remove(remove)?);
        }
        // A package can be picked up by both removal paths (a plan conflict
        // and an explicit remove mark) — emit a single Remove change.
        let mut seen_removed: HashSet<String> = HashSet::new();
        changes.retain(|c| {
            if c.kind == ChangeKind::Remove && !seen_removed.insert(c.package.clone()) {
                return false;
            }
            true
        });
        Ok(ChangeSet { changes })
    }

    /// Mark every installed package that has a newer candidate for upgrade,
    /// honoring `mode` — like `apt upgrade`'s marking step. Each outdated
    /// package is marked with its candidate entry (via [`Self::mark_install`]),
    /// so it is upgraded to the newest available version.
    ///
    /// In [`UpgradeMode::SafeUpgrade`] and [`UpgradeMode::MinimalUpgrade`],
    /// upgrades that would violate the mode — remove an installed package,
    /// or (for `MinimalUpgrade`) install a new one — are *held back*: the
    /// package is left at its current version, exactly like apt. Does **not**
    /// resolve — call [`Self::resolve`] afterwards.
    pub fn upgrade(&mut self, mode: UpgradeMode) {
        let outdated = self.outdated_entries();
        match mode {
            UpgradeMode::FullUpgrade => {
                for target in outdated {
                    self.mark_install(target, false);
                }
            }
            UpgradeMode::SafeUpgrade | UpgradeMode::MinimalUpgrade => {
                // Installed providers are shared by every safety check —
                // building the map once beats re-scanning the installed set
                // per outdated package. The per-package exact solves share a
                // single provider/solver too (the intern pool and reverse
                // indexes are built once, not per outdated package).
                let providers = self.installed_providers();
                let mut solver = SharedSolver::new(self.index, Some(self.dpkg), self.options);
                let safe: Vec<bool> = outdated
                    .iter()
                    .map(|target| self.upgrade_is_safe(target, mode, &providers, &mut solver))
                    .collect();
                // `solver` borrows `self.index`/`self.dpkg`, so apply the
                // marks only after it is dropped.
                drop(solver);
                for (target, ok) in outdated.into_iter().zip(safe) {
                    if ok {
                        self.mark_install(target, false);
                    }
                }
            }
        }
    }

    /// Mark phase: find installed packages whose *installed* version has an
    /// unsatisfied hard dependency (`Pre-Depends`/`Depends` — dpkg's notion
    /// of broken) and mark them for install, so [`Self::resolve`] fixes them:
    /// the resolver installs the missing dependencies (or upgrades/reinstalls
    /// the package). Mirrors `apt --fix-broken` (`pkgDepCache::FixBroken`).
    ///
    /// The marks carry **no pinned version**, so the resolver keeps the
    /// installed version when it is satisfiable (apt keeps the package where
    /// it is and only adds what's missing). Returns the names of the broken
    /// packages found. Does **not** resolve — call [`Self::resolve`]
    /// afterwards.
    pub fn fix_broken(&mut self) -> Result<Vec<String>, ResolveError> {
        // Installed providers of each virtual name, built once — the broken
        // check would otherwise re-parse every installed package's Provides
        // for each dependency alternative it tests.
        let providers = self.installed_providers();
        let mut broken = Vec::new();
        for name in self.dpkg.installed_packages() {
            if self.installed_version_broken(name, &providers) {
                self.mark_install(
                    PackageEntry {
                        package: name.to_string(),
                        version: None,
                        ..PackageEntry::default()
                    },
                    false,
                );
                broken.push(name.to_string());
            }
        }
        Ok(broken)
    }

    /// Mark phase: find installed auto-installed packages that are no longer
    /// needed — not hard-depended on, transitively, by any *manually*
    /// installed package — and mark them for removal. Mirrors
    /// `apt-get autoremove` (`pkgDepCache::MarkAndSweep`):
    ///
    /// - the used set starts from the installed roots (packages that are not
    ///   auto-installed, plus essential / protected / held / `Priority:
    ///   required` packages)
    /// - it grows to every installed hard dependency (`Pre-Depends` /
    ///   `Depends`) that satisfies a used package — the real package at a
    ///   matching version, or an installed provider of a virtual name
    /// - the sweep removes installed auto packages that are not in the used
    ///   set (essential/protected are never removed)
    ///
    /// `auto_installed` is the set recorded with `Auto-Installed: 1` (see
    /// [`crate::AptExtendedStates`]); `never_auto_remove` is the
    /// `APT::NeverAutoRemove` regex list (from
    /// `cfg.keys_under("APT::NeverAutoRemove")`) — packages matching any
    /// pattern are kept as roots, like apt's `01autoremove` kernel/firmware
    /// entries. Returns the names marked for removal. Does **not** resolve —
    /// call [`Self::resolve`] afterwards.
    ///
    /// Note: the `APT::VersionedKernelPackages` logic (keeping the running
    /// kernel and the newest installed ones) is not implemented — such
    /// packages are only protected when a `NeverAutoRemove` pattern matches.
    pub fn autoremove(
        &mut self,
        auto_installed: &HashSet<String>,
        never_auto_remove: &[String],
    ) -> Vec<String> {
        // Installed providers of each virtual name, so a hard dep on a
        // virtual package keeps its installed provider.
        let providers = self.installed_providers();

        // Packages matching APT::NeverAutoRemove are never autoremoved (e.g.
        // the kernel/firmware patterns in apt's 01autoremove); patterns that
        // do not compile are skipped, like apt.
        let never_auto_remove: Vec<Regex> = never_auto_remove
            .iter()
            .filter_map(|pat| Regex::new(pat).ok())
            .collect();

        // Mark phase: the used set starts from the installed roots and grows
        // through their satisfied hard dependencies.
        let mut used: HashSet<String> = HashSet::new();
        let mut queue: Vec<String> = Vec::new();
        for name in self.dpkg.installed_packages() {
            let root = !auto_installed.contains(name)
                || self.dpkg.is_essential(name)
                || self.dpkg.is_protected(name)
                || self.dpkg.is_held(name)
                || self.installed_is_required(name)
                || never_auto_remove.iter().any(|re| re.is_match(name));
            if root && used.insert(name.to_string()) {
                queue.push(name.to_string());
            }
        }
        while let Some(name) = queue.pop() {
            self.mark_satisfied_hard_deps_used(&name, &providers, &mut used, &mut queue);
        }

        // Sweep phase: installed auto packages that nothing used depends on
        // are no longer needed.
        let mut removable = Vec::new();
        for name in self.dpkg.installed_packages() {
            if !auto_installed.contains(name) || used.contains(name) {
                continue;
            }
            if self.dpkg.is_essential(name)
                || self.dpkg.is_protected(name)
                || self.dpkg.is_held(name)
            {
                continue;
            }
            self.mark_remove(PackageEntry {
                package: name.to_string(),
                ..PackageEntry::default()
            });
            removable.push(name.to_string());
        }
        removable
    }

    /// Whether the *installed* version of `name` is broken: some hard
    /// dependency (`Pre-Depends`/`Depends`) group has no alternative
    /// satisfied by an installed package. This is apt's fix-broken notion of
    /// broken (an installed package whose dependencies are unmet) — *not* the
    /// solver's installability check, which ignores what's installed.
    fn installed_version_broken(
        &self,
        name: &str,
        providers: &HashMap<String, Vec<String>>,
    ) -> bool {
        let Some(version) = self.dpkg.installed_version(name) else {
            return false; // not installed → nothing to fix
        };
        let Some(deps) = self.index.deps_of(name, version) else {
            return false; // no index entry for the installed version — leave it alone
        };
        for group in deps.pre_depends.iter().chain(deps.depends.iter()) {
            if !group
                .iter()
                .any(|alt| self.installed_dep_satisfied(alt, providers))
            {
                return true;
            }
        }
        false
    }

    /// Whether dependency alternative `alt` is satisfied by some *installed*
    /// package: a package named `alt.name` at a matching version, or an
    /// installed package that `Provides` it at a matching version.
    fn installed_dep_satisfied(
        &self,
        alt: &debian_control::lossy::Relation,
        providers: &HashMap<String, Vec<String>>,
    ) -> bool {
        // Direct: an installed package with the same name, at a matching
        // version. The arch-qualified name (which equals the bare name in
        // single-arch indexes) matches the dpkg state keying.
        if let Some(version) = self.dpkg.installed_version(&alt.qualified_name())
            && dep_version_matches(alt, version)
        {
            return true;
        }
        // Virtual: an installed provider of `alt.name` at a matching version
        // (a provider's Provides constraint is usually its own version).
        let Some(ps) = providers.get(&alt.name) else {
            return false;
        };
        ps.iter().any(|other| {
            let Some(version) = self.dpkg.installed_version(other) else {
                return false;
            };
            dep_version_matches(alt, version)
        })
    }

    /// Installed providers of each virtual name — virtual name → the
    /// installed packages that `Provides` it (checked against each package's
    /// *installed* version entry). Built once and reused by
    /// [`Self::autoremove`] and [`Self::fix_broken`], so testing a dependency
    /// against a virtual name never re-scans (and re-parses) every installed
    /// package.
    fn installed_providers(&self) -> HashMap<String, Vec<String>> {
        let mut providers: HashMap<String, Vec<String>> = HashMap::new();
        for other in self.dpkg.installed_packages() {
            let Some(version) = self.dpkg.installed_version(other) else {
                continue;
            };
            let Some(deps) = self.index.deps_of(other, version) else {
                continue;
            };
            for dep in &deps.provides {
                providers
                    .entry(dep.name.clone())
                    .or_default()
                    .push(other.to_string());
            }
        }
        providers
    }

    /// Whether the *installed* version of `name` has `Priority: required` —
    /// apt never autoremoves such packages.
    fn installed_is_required(&self, name: &str) -> bool {
        let Some(version) = self.dpkg.installed_version(name) else {
            return false;
        };
        let Some(entry) = self.index.get_version(name, version) else {
            return false;
        };
        entry.entry.priority.as_deref() == Some("required")
    }

    /// Mark the installed hard dependencies of `name` (the currently used
    /// package) as used, pushing newly-used ones onto `queue` for recursion.
    ///
    /// Like apt's `MarkPackage`, only dependencies satisfied by an installed
    /// version are followed — the real package at a matching version, or an
    /// installed provider of a virtual name at a matching version.
    fn mark_satisfied_hard_deps_used(
        &self,
        name: &str,
        providers: &HashMap<String, Vec<String>>,
        used: &mut HashSet<String>,
        queue: &mut Vec<String>,
    ) {
        let Some(version) = self.dpkg.installed_version(name) else {
            return;
        };
        let Some(deps) = self.index.deps_of(name, version) else {
            return;
        };
        let mut targets: Vec<String> = Vec::new();
        for group in deps.pre_depends.iter().chain(deps.depends.iter()) {
            for alt in group {
                // The real package, installed at a version satisfying the
                // constraint…
                if let Some(installed) = self.dpkg.installed_version(&alt.name)
                    && dep_version_matches(alt, installed)
                {
                    targets.push(alt.name.clone());
                    continue;
                }
                // …or an installed provider of a virtual name at a
                // matching version.
                if let Some(ps) = providers.get(&alt.name) {
                    for p in ps {
                        let Some(pv) = self.dpkg.installed_version(p) else {
                            continue;
                        };
                        if dep_version_matches(alt, pv) {
                            targets.push(p.clone());
                        }
                    }
                }
            }
        }
        for t in targets {
            if used.insert(t.clone()) {
                queue.push(t);
            }
        }
    }

    /// The candidate entry of every installed package that has a newer
    /// version available — what [`Self::upgrade`] would upgrade to.
    fn outdated_entries(&self) -> Vec<PackageEntry> {
        let mut outdated = Vec::new();
        for name in self.dpkg.installed_packages() {
            let Some(installed) = self.dpkg.installed_version(name) else {
                continue;
            };
            let Some(candidate) = self.index.candidate_version(name) else {
                continue;
            };
            let Some(new) = candidate.parsed_version() else {
                continue;
            };
            let Some(old) = Version::parse_lenient(installed).ok() else {
                continue;
            };
            if new > old {
                outdated.push(candidate.into_owned().entry);
            }
        }
        outdated
    }

    /// Whether upgrading `target` to its marked version respects `mode` —
    /// used to hold back unsafe upgrades in [`Self::upgrade`].
    ///
    /// Two tiers, to avoid a full solve per outdated package:
    ///
    /// 1. *Fast pass* (apt's `IsUpgradeOk`-style graph check on the
    ///    candidate): under `MinimalUpgrade` every hard dependency is
    ///    already satisfied (by name+version) by the installed set, and
    ///    under `SafeUpgrade`/`MinimalUpgrade` the candidate Breaks/Conflicts
    ///    nothing installed. When this holds the upgrade is *provably* safe
    ///    — no new package, no removal — so it is accepted without solving.
    /// 2. *Exact pass*: otherwise fall back to a full per-package solve
    ///    (the dependency may merely need upgrading, which the graph check
    ///    cannot see without apt's incremental mark state), keeping the
    ///    hold-back decision identical to the old always-solve behaviour.
    fn upgrade_is_safe(
        &self,
        target: &PackageEntry,
        mode: UpgradeMode,
        providers: &HashMap<String, Vec<String>>,
        solver: &mut SharedSolver<'_>,
    ) -> bool {
        let Some(version) = target.version.as_deref() else {
            return true;
        };
        let Some(deps) = self.index.deps_of(&target.package, version) else {
            return false;
        };

        let deps_installed = deps
            .pre_depends
            .iter()
            .chain(deps.depends.iter())
            .all(|group| {
                group
                    .iter()
                    .any(|alt| self.installed_dep_satisfied(alt, providers))
            });
        let no_conflict = !deps
            .breaks
            .iter()
            .flatten()
            .chain(deps.conflicts.iter().flatten())
            .any(|dep| self.installed_dep_satisfied(dep, providers));
        // Fast pass: when the graph check covers exactly what the mode
        // forbids and it holds, the upgrade is provably safe — skip the solve.
        let fast_ok = match mode {
            UpgradeMode::MinimalUpgrade => deps_installed && no_conflict,
            UpgradeMode::SafeUpgrade => no_conflict,
            UpgradeMode::FullUpgrade => true,
        };
        if fast_ok {
            return true;
        }

        // Exact pass: full solve (sharing the caller's provider/solver),
        // mirroring the original always-solve logic.
        let roots = [(
            target.package.as_str(),
            AptVersionSet::Constraint(VersionConstraint::Equal, version.to_string()),
        )];
        let Ok(solution) = solver.solve(&roots) else {
            return false; // unsolvable → hold back
        };
        let plan: Vec<InstallItem> = solution
            .into_iter()
            .map(|(name, version)| InstallItem {
                depends_on: Vec::new(),
                name,
                version,
                installed_size: None,
                download_size: None,
            })
            .collect();
        // `apt-get upgrade` (Minimal): never install new packages.
        if matches!(mode, UpgradeMode::MinimalUpgrade) {
            for item in &plan {
                if item.name != target.package && !self.dpkg.is_installed(&item.name) {
                    return false;
                }
            }
        }
        // `apt upgrade` / `apt-get upgrade` (Safe/Minimal): never remove
        // installed packages.
        if matches!(mode, UpgradeMode::SafeUpgrade | UpgradeMode::MinimalUpgrade)
            && !self.plan_removal_targets(&plan).is_empty()
        {
            return false;
        }
        true
    }

    /// Installed packages (names) that `plan` conflicts with or breaks — the
    /// ones that would have to be removed for `plan` to apply. Does **not**
    /// run the reverse hard-dependency closure or the essential/protected
    /// checks; callers decide how to react.
    fn plan_removal_targets(&self, plan: &[InstallItem]) -> HashSet<String> {
        let plan_names: HashSet<&str> = plan.iter().map(|item| item.name.as_str()).collect();

        // Precompute each plan entry's Conflicts/Breaks groups once, instead
        // of re-looking-up and re-parsing them for every installed package.
        let plan_conflicts: Vec<Vec<debian_control::lossy::Relation>> = plan
            .iter()
            .flat_map(|item| {
                self.index
                    .deps_of(&item.name, &item.version)
                    .map(|deps| conflict_groups(&deps))
                    .unwrap_or_default()
            })
            .collect();

        let mut to_remove: HashSet<String> = HashSet::new();
        for installed in self.dpkg.installed_packages() {
            if plan_names.contains(installed) {
                continue;
            }
            let installed_version = self.dpkg.installed_version(installed).unwrap_or("");
            // Plan packages' Conflicts/Breaks against the installed package,
            // using the precomputed groups.
            let conflicted = plan_conflicts.iter().any(|g| {
                g.iter()
                    .any(|dep| dep_matches(dep, installed, installed_version))
            }) || self
                .index
                .deps_of(installed, installed_version)
                .is_some_and(|deps| {
                    // Installed package's own Conflicts/Breaks against the
                    // plan — checked against the *installed* version's entry
                    // (what is actually active on the system), not the
                    // candidate's.
                    let own = conflict_groups(&deps);
                    plan.iter().any(|item| {
                        own.iter().any(|g| {
                            g.iter()
                                .any(|dep| dep_matches(dep, &item.name, &item.version))
                        })
                    })
                });
            if conflicted {
                to_remove.insert(installed.to_string());
            }
        }
        to_remove
    }

    /// Install side of [`Self::resolve`]: solve `roots` with the planner's
    /// options and diff against the installed state. The result carries the
    /// install-side changes (`Install`/`Upgrade`/`Downgrade`/`Reinstall`) for
    /// everything the resolver selects that isn't already present at that
    /// version in a healthy state, plus `Remove` changes for installed
    /// packages the solution conflicts with or breaks (including their
    /// reverse hard-dependents, still refusing to remove essential/protected
    /// packages). Roots named in `reinstall` are forced to a
    /// [`ChangeKind::Reinstall`] change when already present at the selected
    /// version.
    fn resolve_install(
        &self,
        roots: Vec<PackageEntry>,
        reinstall: &HashSet<String>,
    ) -> Result<Vec<Change>, ResolveError> {
        // Mark phase: resolve the install set *without* ordering it — apt's
        // `MarkInstall` does not sort; the dpkg order is computed later in
        // the order phase ([`ChangeSet::into_transaction`]).
        //
        // Each mark is the candidate entry the caller resolved before
        // marking, so its root pins that exact version (apt's
        // `SetCandidateVersion`); a mark without a version leaves the version
        // to the resolver.
        let roots: Vec<(&str, AptVersionSet)> = roots
            .iter()
            .map(|e| {
                let vs = e.version.as_deref().map_or(AptVersionSet::Any, |v| {
                    AptVersionSet::Constraint(VersionConstraint::Equal, v.to_string())
                });
                (e.package.as_str(), vs)
            })
            .collect();
        let plan = resolve_plan(self.index, Some(self.dpkg), &roots, self.options)?;

        // Everything the resolver selects that wasn't an explicit root was
        // pulled in automatically as a dependency — apt's `Auto-Installed`
        // flag, which the executor records after a successful install.
        let root_names: HashSet<&str> = roots.iter().map(|(name, _)| *name).collect();

        // Removals: installed packages the plan conflicts with or breaks,
        // then grown with installed reverse hard-dependents (apt's
        // `MarkDelete` marks dependents for removal too, recursively).
        // Essential/protected packages are protected: apt refuses to remove
        // them, so the plan is not achievable and the mark fails. This only
        // borrows the plan, so it runs before the plan is consumed below.
        let mut to_remove = self.plan_removal_targets(&plan);
        for installed in &to_remove {
            self.require_removable(installed)?;
        }

        // Reverse hard-dependency closure: installed packages that depend on
        // a removal target are removed too (recursively), still refusing to
        // remove essential packages. The reverse index is built once — the
        // naive per-(target, package) check would re-parse every installed
        // package's dependencies for each target.
        if !to_remove.is_empty() {
            let dependents = self.installed_dependents();
            let plan_names: HashSet<&str> = plan.iter().map(|item| item.name.as_str()).collect();
            let mut queue: Vec<String> = to_remove.iter().cloned().collect();
            while let Some(name) = queue.pop() {
                let Some(deps) = dependents.get(&name) else {
                    continue;
                };
                for other in deps {
                    if plan_names.contains(other.as_str()) || to_remove.contains(other) {
                        continue;
                    }
                    self.require_removable(other)?;
                    to_remove.insert(other.clone());
                    queue.push(other.clone());
                }
            }
        }

        // Install side: everything the resolver selects that wasn't an
        // explicit root was pulled in automatically as a dependency — apt's
        // `Auto-Installed` flag, which the executor records after a
        // successful install. Consuming the plan moves each item's fields
        // into the change instead of cloning them.
        let mut changes = Vec::new();
        for item in plan {
            let auto_installed = !root_names.contains(item.name.as_str());
            let Some(installed) = self.dpkg.installed_version(&item.name) else {
                changes.push(Change {
                    kind: ChangeKind::Install,
                    package: item.name,
                    from_version: None,
                    to_version: Some(item.version),
                    old_size: None,
                    new_size: item.installed_size,
                    download_size: item.download_size,
                    depends_on: item.depends_on,
                    auto_installed,
                });
                continue;
            };
            if installed == item.version {
                if self.dpkg.needs_reinstall(&item.name) || reinstall.contains(&item.name) {
                    let old_size = size_fields(self.index, &item.name, installed).0;
                    changes.push(Change {
                        kind: ChangeKind::Reinstall,
                        package: item.name,
                        from_version: Some(installed.to_string()),
                        to_version: Some(item.version),
                        old_size,
                        new_size: item.installed_size,
                        download_size: item.download_size,
                        depends_on: item.depends_on,
                        auto_installed,
                    });
                }
                continue; // already present at the selected version
            }
            let kind = match (
                Version::parse_lenient(installed).ok(),
                Version::parse_lenient(&item.version).ok(),
            ) {
                (Some(old), Some(new)) if old > new => ChangeKind::Downgrade,
                _ => ChangeKind::Upgrade,
            };
            let old_size = size_fields(self.index, &item.name, installed).0;
            changes.push(Change {
                kind,
                package: item.name,
                from_version: Some(installed.to_string()),
                to_version: Some(item.version),
                old_size,
                new_size: item.installed_size,
                download_size: item.download_size,
                depends_on: item.depends_on,
                auto_installed,
            });
        }

        for installed in to_remove {
            let old_size = self
                .dpkg
                .installed_version(&installed)
                .and_then(|v| size_fields(self.index, &installed, v).0);
            changes.push(Change {
                kind: ChangeKind::Remove,
                from_version: self.dpkg.installed_version(&installed).map(str::to_string),
                to_version: None,
                old_size,
                new_size: None,
                download_size: None,
                depends_on: Vec::new(),
                package: installed,
                auto_installed: false,
            });
        }

        Ok(changes)
    }

    /// Remove side of [`Self::resolve`]: installed packages that must go with
    /// `targets` — the reverse hard-dependency closure over
    /// `Pre-Depends`/`Depends` — as `Remove` changes.
    ///
    /// The explicit `targets` are user-requested, so like apt's `FromUser`
    /// they bypass the essential/protected/held protection; reverse
    /// hard-dependents pulled in by the closure are not user-requested, so
    /// they are refused when protected ([`Self::require_removable`], apt's
    /// `IsModeChangeOk` with `FromUser=false`).
    fn resolve_remove(&self, mut targets: Vec<PackageEntry>) -> Result<Vec<Change>, ResolveError> {
        // Reverse hard-dependency closure. `to_remove` doubles as the visited
        // set. The explicit targets are popped directly as the work list (they
        // are owned and unused afterwards — no separate queue to seed);
        // reverse hard-dependents found along the way are appended as names.
        // The reverse index is built once — the naive per-(target, package)
        // check would re-parse every installed package's dependencies for
        // each target.
        let explicit: HashSet<String> = targets.iter().map(|t| t.package.clone()).collect();
        let dependents = self.installed_dependents();
        let mut to_remove: HashSet<String> = HashSet::new();
        let mut pending: Vec<String> = Vec::new();

        while let Some(name) = targets.pop().map(|t| t.package).or_else(|| pending.pop()) {
            if !to_remove.insert(name.clone()) {
                continue;
            }
            let Some(deps) = dependents.get(&name) else {
                continue;
            };
            for other in deps {
                // Explicit targets are never refused (user-requested); only
                // closure-found reverse-dependents hit the protection.
                if to_remove.contains(other) || explicit.contains(other) {
                    continue;
                }
                self.require_removable(other)?;
                let other = other.clone();
                if to_remove.insert(other.clone()) {
                    pending.push(other);
                }
            }
        }

        let mut changes: Vec<Change> = to_remove
            .into_iter()
            .map(|package| {
                let old_size = self
                    .dpkg
                    .installed_version(&package)
                    .and_then(|v| size_fields(self.index, &package, v).0);
                Change {
                    kind: ChangeKind::Remove,
                    from_version: self.dpkg.installed_version(&package).map(str::to_string),
                    to_version: None,
                    old_size,
                    new_size: None,
                    download_size: None,
                    depends_on: Vec::new(),
                    package,
                    auto_installed: false,
                }
            })
            .collect();
        changes.sort_by(|a, b| a.package.cmp(&b.package));

        Ok(changes)
    }

    /// Installed reverse hard-dependency index: package name → the installed
    /// packages that hard-depend on it (name equality on any
    /// `Pre-Depends`/`Depends` alternative, checked against each package's
    /// *installed* version entry). Built once per removal pass, so the
    /// reverse-dependency closure never re-parses every installed package's
    /// dependencies for each removal target.
    fn installed_dependents(&self) -> HashMap<String, Vec<String>> {
        let mut dependents: HashMap<String, Vec<String>> = HashMap::new();
        for other in self.dpkg.installed_packages() {
            let Some(version) = self.dpkg.installed_version(other) else {
                continue;
            };
            let Some(deps) = self.index.deps_of(other, version) else {
                continue;
            };
            for group in deps.pre_depends.iter().chain(deps.depends.iter()) {
                for alt in group {
                    dependents
                        .entry(alt.name.clone())
                        .or_default()
                        .push(other.to_string());
                }
            }
        }
        dependents
    }

    /// Protection levels, like apt's `IsModeChangeOk`: essential
    /// (`Essential: yes`), protected (`Protected: yes`, dpkg 1.19+ — apt's
    /// `Flag::Important`) and held (`Status: hold`) packages can never be
    /// removed by a plan — apt blocks non-user changes to them. Returns an
    /// error if `name` would need to be removed.
    fn require_removable(&self, name: &str) -> Result<(), ResolveError> {
        if self.dpkg.is_essential(name) {
            return Err(ResolveError::Essential(format!(
                "would remove essential package {name}"
            )));
        }
        if self.dpkg.is_protected(name) {
            return Err(ResolveError::Protected(format!(
                "would remove protected package {name}"
            )));
        }
        if self.dpkg.is_held(name) {
            return Err(ResolveError::Held(format!(
                "would remove held package {name}"
            )));
        }
        Ok(())
    }
}

/// Parse an entry's `Conflicts`/`Breaks` fields into dep groups (empty when
/// the entry has neither).
fn conflict_groups(deps: &ParsedDeps) -> Vec<Vec<debian_control::lossy::Relation>> {
    deps.conflicts
        .iter()
        .chain(deps.breaks.iter())
        .cloned()
        .collect()
}

/// Whether a `Conflicts`/`Breaks` alternative targets `name` at `version`.
fn dep_matches(dep: &debian_control::lossy::Relation, name: &str, version: &str) -> bool {
    dep.name == name && dep_version_matches(dep, version)
}

/// Whether `version` satisfies `dep`'s version constraint (ignoring its
/// name). No constraint is always satisfied.
fn dep_version_matches(dep: &debian_control::lossy::Relation, version: &str) -> bool {
    let Some((relation, want)) = &dep.version else {
        return true; // no version constraint
    };
    let Some(actual) = Version::parse_lenient(version).ok() else {
        return false;
    };
    match relation {
        VersionConstraint::LessThan => actual < *want,
        VersionConstraint::LessThanEqual => actual <= *want,
        VersionConstraint::Equal => actual == *want,
        VersionConstraint::GreaterThanEqual => actual >= *want,
        VersionConstraint::GreaterThan => actual > *want,
    }
}
