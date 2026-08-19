use resolvo::{Requirement, utils::Pool};

use crate::{AptDb, DpkgState, RelationExt};

use super::{AptProvider, AptVersionSet, ResolveError, ResolveOptions};

/// Convert an OR-group into a resolvo `Requirement`.
///
/// A group `a | b | c` becomes a `Union` of version sets (one per
/// alternative); a single alternative becomes a plain `Single`.
pub(crate) fn group_to_requirement(
    pool: &Pool<AptVersionSet, String>,
    group: &[debian_control::lossy::Relation],
) -> Requirement {
    if group.len() == 1 {
        let name_id = pool.intern_package_name(group[0].qualified_name());
        let vs = dep_version_set(&group[0]);

        return Requirement::Single(pool.intern_version_set(name_id, vs));
    }

    // OR group: intern each alternative's version set, then a Union over the
    // first plus the rest (no intermediate Vec needed).
    let mut iter = group.iter();
    let first_dep = iter.next().expect("OR group is non-empty");
    let first_name = pool.intern_package_name(first_dep.qualified_name());
    let first_vs = dep_version_set(first_dep);
    let first = pool.intern_version_set(first_name, first_vs);
    let others = iter.map(|dep| {
        let nid = pool.intern_package_name(dep.qualified_name());
        let vs = dep_version_set(dep);
        pool.intern_version_set(nid, vs)
    });

    Requirement::Union(pool.intern_version_set_union(first, others))
}

/// Build the `AptVersionSet` for a single dependency alternative.
pub(crate) fn dep_version_set(dep: &debian_control::lossy::Relation) -> AptVersionSet {
    match &dep.version {
        Some((rel, ver)) => AptVersionSet::Constraint(rel.clone(), ver.to_string()),
        None => AptVersionSet::Any,
    }
}

/// Solve for a consistent set of versions satisfying `roots` (name plus an
/// optional version set) and all their transitive dependencies.
///
/// Returns the selected `(package, version)` pairs in solution order, or a
/// human-readable error describing why no solution exists.
pub fn solve_requirements(
    index: &AptDb,
    roots: &[(&str, AptVersionSet)],
) -> Result<Vec<(String, String)>, ResolveError> {
    solve_requirements_with(index, roots, ResolveOptions::default())
}

/// Like [`solve_requirements`], with explicit resolution options.
pub fn solve_requirements_with(
    index: &AptDb,
    roots: &[(&str, AptVersionSet)],
    options: ResolveOptions,
) -> Result<Vec<(String, String)>, ResolveError> {
    solve_with(index, None, options, |provider| {
        roots
            .iter()
            .map(|(name, vs)| {
                let name_id = provider.pool.pool.intern_package_name(*name);
                let vs_id = provider.pool.pool.intern_version_set(name_id, vs.clone());
                resolvo::ConditionalRequirement::from(vs_id)
            })
            .collect()
    })
}

/// Convenience wrapper around [`solve_requirements`] that requires any version
/// of each root package.
pub fn solve_packages(
    index: &AptDb,
    root_names: &[&str],
) -> Result<Vec<(String, String)>, ResolveError> {
    solve_with(index, None, ResolveOptions::default(), |provider| {
        root_names
            .iter()
            .map(|name| {
                let name_id = provider.pool.pool.intern_package_name(*name);
                let vs_id = provider
                    .pool
                    .pool
                    .intern_version_set(name_id, AptVersionSet::Any);
                resolvo::ConditionalRequirement::from(vs_id)
            })
            .collect()
    })
}

/// Run the solver with requirements interned against `index`'s pool.
///
/// `dpkg` optionally provides the installed state so
/// [`ResolveOptions::prefer_installed`] (apt semantics) can take effect.
///
/// `build_requirements` runs while the provider (and its intern pool) is
/// alive; the provider is then moved into the solver.
pub(crate) fn solve_with(
    index: &AptDb,
    dpkg: Option<&DpkgState>,
    options: ResolveOptions,
    build_requirements: impl FnOnce(&AptProvider<'_>) -> Vec<resolvo::ConditionalRequirement>,
) -> Result<Vec<(String, String)>, ResolveError> {
    let provider = AptProvider::with_options_and_dpkg(index, dpkg, options);
    let requirements = build_requirements(&provider);

    let mut solver = resolvo::Solver::new(provider);

    let problem = resolvo::Problem::new().requirements(requirements);
    let solution = match solver.solve(problem) {
        Ok(solution) => solution,
        Err(resolvo::UnsolvableOrCancelled::Unsolvable(conflict)) => {
            return Err(ResolveError::Unsolvable(
                conflict.display_user_friendly(&solver).to_string(),
            ));
        }
        Err(e) => return Err(ResolveError::Cancelled(format!("{e:?}"))),
    };

    let provider = solver.provider();

    Ok(solution
        .into_iter()
        .map(|id| {
            let solvable = provider.pool.pool.resolve_solvable(id);
            let name = provider.pool.pool.resolve_package_name(solvable.name);
            (name.clone(), solvable.record.clone())
        })
        .collect())
}

/// A solver that builds its [`AptProvider`] (the intern pool and the
/// provides/solvables/group reverse indexes) **once** and reuses it across
/// several solves — apt reuses its single dependency graph the same way.
///
/// Each [`SharedSolver::solve`] is an independent run:
/// [`resolvo::Solver::solve`] resets its state per call, so solving many
/// small root sets (e.g. one per outdated package during upgrade marking)
/// shares the expensive provider build instead of re-interning the whole
/// universe per solve.
///
/// This is the reusable entry point for tools that solve many small root
/// sets against one index (the `broken` example, upgrade marking, ...).
pub struct SharedSolver<'a> {
    solver: resolvo::Solver<AptProvider<'a>>,
}

impl<'a> SharedSolver<'a> {
    /// Build a solver whose intern pool is built once over `index`.
    ///
    /// `dpkg` optionally provides the installed state so
    /// [`ResolveOptions::prefer_installed`] (apt semantics) can take effect.
    pub fn new(index: &'a AptDb, dpkg: Option<&'a DpkgState>, options: ResolveOptions) -> Self {
        let provider = AptProvider::with_options_and_dpkg(index, dpkg, options);
        Self {
            solver: resolvo::Solver::new(provider),
        }
    }

    /// Solve for `roots`, returning the selected `(package, version)` pairs
    /// in solution order, or a human-readable error when unsolvable.
    ///
    /// Each call is an independent run — [`resolvo::Solver::solve`] resets
    /// its state — but every call shares the pool built in [`SharedSolver::new`].
    pub fn solve(
        &mut self,
        roots: &[(&str, AptVersionSet)],
    ) -> Result<Vec<(String, String)>, ResolveError> {
        // Intern the roots against the shared provider; the borrow of
        // `self.solver` ends before `solve` takes it mutably.
        let requirements = {
            let provider = self.solver.provider();
            roots
                .iter()
                .map(|(name, vs)| {
                    let name_id = provider.pool.pool.intern_package_name(*name);
                    let vs_id = provider.pool.pool.intern_version_set(name_id, vs.clone());
                    resolvo::ConditionalRequirement::from(vs_id)
                })
                .collect::<Vec<_>>()
        };
        let problem = resolvo::Problem::new().requirements(requirements);
        let solution = match self.solver.solve(problem) {
            Ok(solution) => solution,
            Err(resolvo::UnsolvableOrCancelled::Unsolvable(conflict)) => {
                return Err(ResolveError::Unsolvable(
                    conflict.display_user_friendly(&self.solver).to_string(),
                ));
            }
            Err(e) => return Err(ResolveError::Cancelled(format!("{e:?}"))),
        };
        let provider = self.solver.provider();
        Ok(solution
            .into_iter()
            .map(|id| {
                let solvable = provider.pool.pool.resolve_solvable(id);
                let name = provider.pool.pool.resolve_package_name(solvable.name);
                (name.clone(), solvable.record.clone())
            })
            .collect())
    }
}
