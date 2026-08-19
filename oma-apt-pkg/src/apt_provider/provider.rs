use std::collections::{HashMap, HashSet};
use std::fmt;

use debversion::Version;
use resolvo::{
    Candidates, Condition, Dependencies, DependencyProvider, Interner, KnownDependencies,
    VersionSetId, utils::Pool,
};

use crate::{AptDb, DpkgState, RelationExt};
use debian_control::relations::VersionConstraint;

use super::{AptVersionSet, ResolveOptions, dep_version_set, group_to_requirement};

/// A pool of all packages, with resolvo intern tables.
///
/// `Pool<AptVersionSet, String>` interning package names, solvables (each
/// carrying a version string) and version sets.
pub(crate) struct AptPool {
    pub(crate) pool: Pool<AptVersionSet, String>,
    /// name_id → the underlying package names that provide it (for virtuals).
    /// For a real package this is `vec![name]`.
    pub(crate) providers: HashMap<resolvo::NameId, Vec<resolvo::NameId>>,
    /// name_id → its solvables (reverse index, built once in [`AptProvider::with_options_and_dpkg`]).
    pub(crate) solvables: HashMap<resolvo::NameId, Vec<resolvo::SolvableId>>,
    /// base name → every arch-qualified name in the same multi-arch group.
    /// Drives apt's implicit cross-arch exclusion (`AddImplicitDepends` in
    /// `pkgcachegen.cc`): group members of different architectures are
    /// mutually exclusive unless `Multi-Arch: same` (which co-installs at
    /// the same version). `Architecture: all` members are excluded — they
    /// are not arch-bound.
    pub(crate) group: HashMap<String, Vec<resolvo::NameId>>,
}

/// A resolvo `DependencyProvider` over an [`AptDb`].
pub(crate) struct AptProvider<'a> {
    /// The package data.
    pub(crate) index: &'a AptDb,
    /// Interned package names and solvables.
    pub(crate) pool: AptPool,
    /// Resolution policy.
    pub(crate) options: ResolveOptions,
    /// Installed state, when known (enables preferring installed versions).
    pub(crate) dpkg: Option<&'a DpkgState>,
}

impl<'a> AptProvider<'a> {
    /// Build a provider by interning every package's candidate versions and
    /// building a provides map.
    ///
    /// `dpkg` optionally provides the installed state so
    /// [`ResolveOptions::prefer_installed`] (apt semantics) can take effect.
    pub(crate) fn with_options_and_dpkg(
        index: &'a AptDb,
        dpkg: Option<&'a DpkgState>,
        options: ResolveOptions,
    ) -> Self {
        let pool = Pool::new();

        // First pass: intern package names and all solvables.
        // `packages()` may include virtual names too; we only intern real
        // names that have entries.
        let mut provides_map: HashMap<resolvo::NameId, Vec<resolvo::NameId>> = HashMap::new();
        let mut solvables_map: HashMap<resolvo::NameId, Vec<resolvo::SolvableId>> = HashMap::new();

        // Collect the real package names first, so the alias pass below can
        // ask "does `X:all` exist?" while registering arch aliases.
        let real_names: Vec<&str> = index
            .packages()
            .filter(|name| !index.versions(name).is_empty())
            .collect();

        for &name in &real_names {
            let name_id = pool.intern_package_name(name);
            let solvables = index
                .versions(name)
                .iter()
                .filter_map(|v| v.entry.version.as_deref().map(|v| (name_id, v.to_string())))
                .map(|(nid, v)| pool.intern_solvable(nid, v))
                .collect::<Vec<_>>();
            solvables_map.insert(name_id, solvables);
            provides_map.entry(name_id).or_default().push(name_id);
        }

        // Multi-arch aliases, registered once so candidate lookup and
        // conflict checks (constraint_for_dep) share a single source of
        // truth:
        // - `X:any` matches every architecture of X;
        // - a specific `X:arch` reference is also satisfied by the
        //   `Architecture: all` package `X:all`, which serves any arch.
        // Bare (unqualified) names — single-arch indexes — get no alias.
        let real_set: HashSet<&str> = real_names.iter().copied().collect();
        let mut group: HashMap<String, Vec<resolvo::NameId>> = HashMap::new();
        for &name in &real_names {
            let Some((base, arch)) = name.rsplit_once(':') else {
                continue;
            };
            let name_id = pool.intern_package_name(name);
            let any_id = pool.intern_package_name(format!("{base}:any"));
            provides_map.entry(any_id).or_default().push(name_id);
            if arch != "all" {
                let all_name = format!("{base}:all");
                if real_set.contains(all_name.as_str()) {
                    let all_id = pool.intern_package_name(&all_name);
                    if let Some(all_providers) = provides_map.get(&all_id).cloned() {
                        provides_map
                            .entry(name_id)
                            .or_default()
                            .extend(all_providers.iter().copied());
                        provides_map
                            .entry(any_id)
                            .or_default()
                            .extend(all_providers.iter().copied());
                    }
                }
                // Same multi-arch group: the arch-qualified names of `base`.
                group.entry(base.to_string()).or_default().push(name_id);
            }
        }

        // Second pass: record `Provides:` virtual names.
        // Each version may declare virtual packages it provides; those
        // virtual names map to the providing package name.
        for name in index.packages() {
            let entries = index.versions(name);
            for version in entries.iter() {
                let Some(version_str) = version.entry.version.as_deref() else {
                    continue;
                };
                let Some(deps) = index.deps_of(name, version_str) else {
                    continue;
                };
                if deps.provides.is_empty() {
                    continue;
                }
                let provider_id = pool.intern_package_name(name);
                for dep in &deps.provides {
                    // Virtual names are registered bare (cross-arch): an
                    // `i386` package's dependency on `virt` is satisfied by
                    // an `amd64` provider.
                    let virtual_id = pool.intern_package_name(&dep.name);
                    provides_map
                        .entry(virtual_id)
                        .or_default()
                        .push(provider_id);
                    // `X:any` references (bare Conflicts/Breaks targets) also
                    // match virtual names, so register the `:any` alias.
                    let any_id = pool.intern_package_name(format!("{}:any", dep.name));
                    provides_map.entry(any_id).or_default().push(provider_id);
                }
            }
        }

        Self {
            index,
            pool: AptPool {
                pool,
                providers: provides_map,
                solvables: solvables_map,
                group,
            },
            options,
            dpkg,
        }
    }
}

impl Interner for AptProvider<'_> {
    type NameId = resolvo::NameId;
    type SolvableId = resolvo::SolvableId;

    fn display_solvable(&self, solvable: Self::SolvableId) -> impl fmt::Display + '_ {
        let solvable = self.pool.pool.resolve_solvable(solvable);
        let name = self.pool.pool.resolve_package_name(solvable.name);
        format!("{name}-{}", solvable.record)
    }

    fn display_name(&self, name: Self::NameId) -> impl fmt::Display + '_ {
        self.pool.pool.resolve_package_name(name)
    }

    fn display_version_set(&self, version_set: VersionSetId) -> impl fmt::Display + '_ {
        let vs = self.pool.pool.resolve_version_set(version_set);
        format!("{vs}")
    }

    fn display_string(&self, string_id: resolvo::StringId) -> impl fmt::Display + '_ {
        self.pool.pool.resolve_string(string_id)
    }

    fn version_set_name(&self, version_set: VersionSetId) -> Self::NameId {
        self.pool.pool.resolve_version_set_package_name(version_set)
    }

    fn solvable_name(&self, solvable: Self::SolvableId) -> Self::NameId {
        self.pool.pool.resolve_solvable(solvable).name
    }

    fn version_sets_in_union(
        &self,
        version_set_union: resolvo::VersionSetUnionId,
    ) -> impl Iterator<Item = VersionSetId> {
        self.pool.pool.resolve_version_set_union(version_set_union)
    }

    fn resolve_condition(&self, condition: resolvo::ConditionId) -> Condition {
        self.pool.pool.resolve_condition(condition).clone()
    }
}

impl DependencyProvider for AptProvider<'_> {
    async fn filter_candidates(
        &self,
        candidates: &[Self::SolvableId],
        version_set: VersionSetId,
        inverse: bool,
    ) -> Vec<Self::SolvableId> {
        let vs = self.pool.pool.resolve_version_set(version_set);

        candidates
            .iter()
            .copied()
            .filter(|&id| {
                let version = &self.pool.pool.resolve_solvable(id).record;
                let matched = vs.matches(version);
                if inverse { !matched } else { matched }
            })
            .collect()
    }

    async fn get_candidates(&self, name: Self::NameId) -> Option<Candidates<Self::SolvableId>> {
        // The provides map is the single source of truth: real packages, the
        // arch aliases (`X:any`, and the `:all` expansion registered at build
        // time) and virtual names all live here.
        let mut provider_names = self.pool.providers.get(&name).cloned().unwrap_or_default();

        // Fall back to the bare name when no arch-qualified flavour exists:
        // virtual packages are registered bare (a provider of `virt` lives
        // under `virt`, so a `virt:amd64` or `virt:any` reference resolves to
        // it), and single-arch indexes carry no arch suffix at all.
        if provider_names.is_empty() {
            let ref_name = self.pool.pool.resolve_package_name(name);
            if let Some((base, _)) = ref_name.rsplit_once(':') {
                let base_id = self.pool.pool.intern_package_name(base);
                if let Some(base_providers) = self.pool.providers.get(&base_id) {
                    provider_names.extend(base_providers.iter().copied());
                }
            }
        }

        let mut seen = HashSet::new();
        let mut candidates = Vec::new();
        for provider in provider_names {
            if let Some(solvables) = self.pool.solvables.get(&provider) {
                for &s in solvables {
                    // The same solvable can be reached via several names
                    // (e.g. the `:all` fallback); keep one copy per solvable.
                    if seen.insert(s) {
                        candidates.push(s);
                    }
                }
            }
        }

        // apt semantics: the installed version is always a candidate — apt
        // reads it from dpkg status even when it isn't in the repos — so the
        // resolver can keep it instead of downgrading (sort_candidates moves
        // it to the front when prefer_installed is set).
        if self.options.prefer_installed
            && let Some(dpkg) = self.dpkg
        {
            let package_name = self.pool.pool.resolve_package_name(name);
            if let Some(installed) = dpkg.installed_version(package_name)
                && !candidates
                    .iter()
                    .any(|&s| self.pool.pool.resolve_solvable(s).record == installed)
            {
                let nid = self.pool.pool.intern_package_name(package_name);
                candidates.push(self.pool.pool.intern_solvable(nid, installed.to_string()));
            }
        }

        if candidates.is_empty() {
            return None;
        }
        Some(Candidates {
            candidates,
            ..Default::default()
        })
    }

    async fn sort_candidates(
        &self,
        _solver: &resolvo::SolverCache<Self>,
        solvables: &mut [Self::SolvableId],
    ) {
        // Sort by version descending so the solver tries the newest first.
        solvables.sort_by(|&a, &b| {
            let a = &self.pool.pool.resolve_solvable(a).record;
            let b = &self.pool.pool.resolve_solvable(b).record;
            let av = Version::parse_lenient(a).ok();
            let bv = Version::parse_lenient(b).ok();
            bv.cmp(&av)
        });

        // apt semantics: prefer keeping the currently installed version when
        // it is among the candidates, instead of spuriously upgrading or
        // downgrading. If the installed version fails the constraints, the
        // solver simply tries the next candidate.
        if self.options.prefer_installed
            && let Some(dpkg) = self.dpkg
            && let Some(first) = solvables.first()
        {
            let name_id = self.pool.pool.resolve_solvable(*first).name;
            let package_name = self.pool.pool.resolve_package_name(name_id);
            if let Some(installed) = dpkg.installed_version(package_name)
                && let Some(pos) = solvables
                    .iter()
                    .position(|&s| self.pool.pool.resolve_solvable(s).record == installed)
            {
                solvables.swap(0, pos);
            }
        }
    }

    async fn get_dependencies(&self, solvable: Self::SolvableId) -> Dependencies {
        let solvable_entry = self.pool.pool.resolve_solvable(solvable);
        let name_id = solvable_entry.name;
        let package_name = self.pool.pool.resolve_package_name(name_id);

        // Use the pre-parsed dependencies for this solvable's *exact*
        // version, since dependency sets (and Breaks/Conflicts) can differ
        // between versions of the same package. Fall back to the candidate
        // when the exact version has no entry (e.g. a synthetic solvable).
        let deps = self
            .index
            .deps_of(package_name, solvable_entry.record.as_str())
            .or_else(|| {
                let candidate = self.index.candidate_version(package_name)?;
                let version = candidate.entry.version.as_deref()?;
                self.index.deps_of(package_name, version)
            });
        let Some(deps) = deps else {
            return Dependencies::Unknown(
                self.pool
                    .pool
                    .intern_string(format!("package {package_name} has no entry")),
            );
        };

        let mut requirements = Vec::new();
        let mut constrains = Vec::new();

        // apt's `AddImplicitDepends` (pkgcachegen.cc): at most one member of
        // a multi-arch group per architecture is installable. Encoded like a
        // Conflicts constraint — selecting this solvable requires every
        // other group member to match this version (`Multi-Arch: same`,
        // co-installable) or nothing at all (everything else, mutually
        // exclusive).
        // An `Architecture: all` holder is arch-independent — it does not
        // exclude the other group members.
        if let Some((base, arch)) = package_name.rsplit_once(':')
            && arch != "all"
            && let Some(members) = self.pool.group.get(base)
        {
            let own_version = solvable_entry.record.as_str();
            let same = self
                .index
                .get_version(package_name, own_version)
                .and_then(|c| c.entry.multi_arch.as_deref().map(str::to_string))
                == Some("same".to_string());
            let vs = if same {
                AptVersionSet::Constraint(VersionConstraint::Equal, own_version.to_string())
            } else {
                AptVersionSet::Empty
            };
            for &member in members {
                if member == name_id {
                    continue;
                }
                constrains.push(self.pool.pool.intern_version_set(member, vs.clone()));
            }
        }

        // Hard dependencies: Pre-Depends + Depends (both are must-satisfy for
        // version selection). Unconditional requirements use `None` condition.
        for group in deps.pre_depends.iter().chain(deps.depends.iter()) {
            let req = group_to_requirement(&self.pool.pool, group);
            requirements.push(req.into());
        }

        // Recommends: install by default → also required (unless disabled).
        if self.options.install_recommends {
            for group in &deps.recommends {
                let req = group_to_requirement(&self.pool.pool, group);
                requirements.push(req.into());
            }
        }

        // Suggests: only required if enabled (APT::Install-Suggests).
        if self.options.install_suggests {
            for group in &deps.suggests {
                let req = group_to_requirement(&self.pool.pool, group);
                requirements.push(req.into());
            }
        }

        // Breaks: the broken package cannot be selected in the excluded range.
        // `constrains` says "if selected, must match this set", so we push the
        // *complement* of the forbidden range.
        for dep in deps.breaks.iter().flatten() {
            if let Some(c) = self.constraint_for_dep(solvable, dep) {
                constrains.push(c);
            }
        }

        // Conflicts: two-way exclusion — the conflicting package cannot be
        // installed alongside us. Encode on our side (the other side is
        // handled when that package's dependencies are queried). Like Breaks,
        // encode the complement of the forbidden range.
        for dep in deps.conflicts.iter().flatten() {
            if let Some(c) = self.constraint_for_dep(solvable, dep) {
                constrains.push(c);
            }
        }

        Dependencies::Known(KnownDependencies {
            requirements,
            constrains,
        })
    }
}

/// Build a constraint (exclusion) for a `Breaks`/`Conflicts` alternative.
///
/// Returns `None` when the constraint would exclude the holding solvable
/// itself — a package never conflicts with itself (apt semantics), and
/// resolvo's clause encoding cannot represent `(¬x ∨ ¬x)`. Self-conflicts
/// on the same package or on a virtual package the holder itself provides
/// are therefore dropped.
impl<'a> AptProvider<'a> {
    fn constraint_for_dep(
        &self,
        holder: resolvo::SolvableId,
        dep: &debian_control::lossy::Relation,
    ) -> Option<VersionSetId> {
        if self.dep_excludes_solvable(holder, dep) {
            return None;
        }
        let name_id = self.pool.pool.intern_package_name(dep.qualified_name());
        let vs = dep_version_set(dep).complement();

        Some(self.pool.pool.intern_version_set(name_id, vs))
    }

    /// Whether `holder` is among the solvables excluded by this
    /// `Breaks`/`Conflicts` alternative — i.e. the holder's own name provides
    /// the target (directly or as a virtual) and its version falls in the
    /// forbidden range.
    fn dep_excludes_solvable(
        &self,
        holder: resolvo::SolvableId,
        dep: &debian_control::lossy::Relation,
    ) -> bool {
        let forbidden = dep_version_set(dep);
        let dep_name_id = self.pool.pool.intern_package_name(dep.qualified_name());
        let Some(provider_names) = self.pool.providers.get(&dep_name_id) else {
            return false;
        };
        let holder_solvable = self.pool.pool.resolve_solvable(holder);
        if !provider_names.contains(&holder_solvable.name) {
            return false;
        }

        forbidden.matches(&holder_solvable.record)
    }
}
