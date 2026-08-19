use std::collections::HashMap;

use crate::{AptDb, DpkgState, RelationExt};

use super::{AptVersionSet, ResolveError, ResolveOptions, solve_with};

/// A single package in an install plan.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InstallItem {
    /// Package name.
    pub name: String,
    /// Selected version.
    pub version: String,
    /// Other packages in the plan that must be installed first (its hard
    /// dependencies: `Pre-Depends` + `Depends`, resolved to concrete plan
    /// entries — including virtual names mapped to their providers).
    pub depends_on: Vec<String>,
    /// `Installed-Size` of the selected version, when known.
    pub installed_size: Option<u64>,
    /// Download `Size` of the selected version, when known.
    pub download_size: Option<u64>,
}

/// Resolve `roots` and their transitive dependencies into an install plan.
///
/// The solver picks a consistent set of versions; this then topologically
/// sorts them so every package's hard dependencies (`Pre-Depends`, `Depends`)
/// come before it. Returns the plan in install order (dependencies first), or
/// an error if the roots have no consistent solution.
///
/// `Recommends`/`Suggests` are not ordering constraints; they may be installed
/// before or after, so they are not used as edges.
pub fn resolve_install_order(
    index: &AptDb,
    roots: &[&str],
) -> Result<Vec<InstallItem>, ResolveError> {
    resolve_install_order_with(index, roots, ResolveOptions::default())
}

/// Like [`resolve_install_order`], with explicit resolution options.
///
/// With [`ResolveOptions::install_recommends`] set to false, the plan covers
/// only the hard dependency closure (`Pre-Depends` + `Depends`), comparable
/// with `apt-cache depends --recurse --important`.
pub fn resolve_install_order_with(
    index: &AptDb,
    roots: &[&str],
    options: ResolveOptions,
) -> Result<Vec<InstallItem>, ResolveError> {
    resolve_install_order_impl(index, None, roots, options)
}

fn resolve_install_order_impl(
    index: &AptDb,
    dpkg: Option<&DpkgState>,
    roots: &[&str],
    options: ResolveOptions,
) -> Result<Vec<InstallItem>, ResolveError> {
    // The public `resolve_install_order*` API marks roots by name only — any
    // version satisfies a root.
    let roots: Vec<(&str, AptVersionSet)> = roots
        .iter()
        .map(|name| (*name, AptVersionSet::Any))
        .collect();
    let items = resolve_plan(index, dpkg, &roots, options)?;
    // Order phase: sort into install order (dependencies first). The public
    // `resolve_install_order*` API promises install order; `mark_install`
    // deliberately uses [`resolve_plan`] and defers ordering to the dpkg-plan
    // stage, mirroring apt (marks are unordered; `pkgOrderList` orders only
    // when the execution plan is built).
    Ok(order_by_deps(items))
}

/// The resolved solution before ordering: the selected packages with their
/// hard-dependency edges, ready for the *mark* phase.
///
/// Mirroring apt, the mark phase
/// ([`crate::apt_provider::TransactionPlanner::mark_install`]) must not sort —
/// ordering is a separate, later stage (apt's `pkgOrderList`), performed when
/// the dpkg plan is built.
pub(crate) fn resolve_plan(
    index: &AptDb,
    dpkg: Option<&DpkgState>,
    roots: &[(&str, AptVersionSet)],
    options: ResolveOptions,
) -> Result<Vec<InstallItem>, ResolveError> {
    let solution = solve_with(index, dpkg, options, |provider| {
        roots
            .iter()
            .map(|(name, vs)| {
                let name_id = provider.pool.pool.intern_package_name(*name);
                let vs_id = provider.pool.pool.intern_version_set(name_id, vs.clone());
                resolvo::ConditionalRequirement::from(vs_id)
            })
            .collect()
    })?;

    // name → version for the selected set.
    let version_by_name: HashMap<&str, &str> = solution
        .iter()
        .map(|(name, version)| (name.as_str(), version.as_str()))
        .collect();

    // Map every provided (virtual) name in the plan to the package that
    // provides it, so a dependency that resolves to a virtual name can be
    // mapped back to a concrete plan entry.
    let mut provider_by_name: HashMap<String, String> = HashMap::new();
    for (name, version) in &solution {
        let Some(deps) = index.deps_of(name, version) else {
            continue;
        };
        for dep in &deps.provides {
            provider_by_name
                .entry(dep.name.clone())
                .or_insert_with(|| name.clone());
        }
    }

    // Hard dependency edges: package → the plan entries it needs first.
    // Each alternative resolves to a concrete plan entry (see
    // [`resolve_dep_target`]) — the bare name alone cannot match, because
    // the plan holds arch-qualified names (`bar:amd64`, and possibly
    // `bar:i386` for a `Multi-Arch: foreign` dependency).
    let mut depends_on: HashMap<String, Vec<String>> = HashMap::new();
    for (name, version) in &solution {
        let mut deps = Vec::new();
        if let Some(parsed) = index.deps_of(name, version) {
            for group in parsed.pre_depends.iter().chain(parsed.depends.iter()) {
                for alt in group {
                    let dep = resolve_dep_target(alt, &version_by_name, &provider_by_name);
                    // Skip self-edges: a package that satisfies one of
                    // its own dependencies (e.g. a transitional package
                    // providing a name it depends on) needs nothing
                    // installed before it.
                    if let Some(dep) = dep.filter(|dep| dep.as_str() != name.as_str()) {
                        deps.push(dep);
                    }
                }
            }
        }
        deps.sort();
        deps.dedup();
        depends_on.insert(name.clone(), deps);
    }

    Ok(solution
        .into_iter()
        .map(|(name, version)| {
            // Sizes come from the exact selected version (no candidate
            // fallback: that could return a different version's sizes).
            let entry = index.get_version(&name, &version);
            InstallItem {
                // Move the dependency edges out of the map instead of
                // cloning them.
                depends_on: depends_on.remove(&name).unwrap_or_default(),
                name,
                version,
                installed_size: entry.as_deref().and_then(|e| e.entry.installed_size),
                download_size: entry.as_deref().and_then(|e| e.entry.size),
            }
        })
        .collect())
}

/// Resolve one dependency alternative to the concrete plan entry satisfying
/// it, if any:
///
/// - the exact arch-qualified name first (also covers bare names in
///   single-arch indexes, where the relation's qualified name is the bare
///   name);
/// - for `X:any` (or a bare name that got qualified), any architecture of X
///   actually selected into the plan — a `Multi-Arch: foreign` package can
///   satisfy a bare dependency across architectures;
/// - a plan package that provides the virtual name.
fn resolve_dep_target(
    alt: &debian_control::lossy::Relation,
    version_by_name: &HashMap<&str, &str>,
    provider_by_name: &HashMap<String, String>,
) -> Option<String> {
    let qualified = alt.qualified_name();
    if version_by_name.contains_key(qualified.as_str()) {
        return Some(qualified);
    }
    if matches!(alt.archqual.as_deref(), Some("any") | None)
        && let Some(found) = version_by_name
            .keys()
            .find(|n| n.rsplit_once(':').is_some_and(|(base, _)| base == alt.name))
    {
        return Some((*found).to_string());
    }
    provider_by_name.get(alt.name.as_str()).cloned()
}

/// An item with a name and hard-dependency edges, orderable by
/// [`order_by_deps`]. Implemented by the resolver's [`InstallItem`] and by the
/// mark phase's [`Change`] (`crate::apt_provider::Change`).
pub(crate) trait Orderable {
    /// The item's package name.
    fn order_name(&self) -> &str;
    /// Hard dependencies within the same set (already concrete plan entries).
    fn order_deps(&self) -> &[String];
}

impl Orderable for InstallItem {
    fn order_name(&self) -> &str {
        &self.name
    }

    fn order_deps(&self) -> &[String] {
        &self.depends_on
    }
}

/// Reorder `items`: consume the `Vec` and return it with hard dependencies
/// before their dependents — the *order* phase, mirroring apt's `pkgOrderList`
/// (which runs at execution-plan time, not at mark time).
///
/// A DFS over the hard-dependency edges assigns each item a *finish* time; a
/// dependency finishes before its dependent, so sorting by finish ascending
/// yields a topological order. Real repositories contain dependency cycles
/// (e.g. glibc ↔ libxcrypt), so back-edges (a dependency still on the DFS
/// stack) are skipped rather than recursed into — every item is still ordered,
/// with dependencies first everywhere an acyclic order exists. Dependencies
/// not present in `items` (e.g. already installed, so not part of the change
/// set) impose no ordering edge.
pub(crate) fn order_by_deps<T: Orderable>(mut items: Vec<T>) -> Vec<T> {
    // name → index. Owned keys so the map can be read while `items` is
    // mutably borrowed by the sort below.
    let by_name: HashMap<String, usize> = items
        .iter()
        .enumerate()
        .map(|(idx, item)| (item.order_name().to_string(), idx))
        .collect();

    let mut finish = vec![0usize; items.len()];
    let mut done = vec![false; items.len()];
    let mut on_stack = vec![false; items.len()];
    let mut timer = 0usize;

    // Deterministic DFS roots (like the old index-based orderer).
    let mut roots: Vec<&str> = items.iter().map(|item| item.order_name()).collect();
    roots.sort_unstable();

    for root in roots {
        let Some(&root_idx) = by_name.get(root) else {
            continue;
        };

        if done[root_idx] {
            continue;
        }

        let mut stack: Vec<(usize, usize)> = vec![(root_idx, 0)];

        on_stack[root_idx] = true;

        while let Some(&(cur, next_dep)) = stack.last() {
            let deps = items[cur].order_deps();

            if next_dep >= deps.len() {
                stack.pop();
                on_stack[cur] = false;
                done[cur] = true;
                finish[cur] = timer;
                timer += 1;
                continue;
            }

            let dep = deps[next_dep].as_str();
            stack.last_mut().expect("non-empty stack").1 += 1;

            let Some(&dep_idx) = by_name.get(dep) else {
                continue;
            };

            if done[dep_idx] || on_stack[dep_idx] {
                continue;
            }

            on_stack[dep_idx] = true;
            stack.push((dep_idx, 0));
        }
    }

    items.sort_by_key(|item| finish[by_name[item.order_name()]]);

    items
}
