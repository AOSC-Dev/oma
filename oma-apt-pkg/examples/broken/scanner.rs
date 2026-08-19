//! Broken-package scanner — moved out of the library.
#![allow(dead_code)] // shared by several examples, each using only part of it
//!
//! The library used to ship `BrokenScanner` / `find_broken` / `package_is_broken`
//! as public API, but nothing in the production code path consumed them (they
//! were only used by examples and tests). They now live here, as a module
//! shared by the diagnostic examples (`broken`, `broken_compare`,
//! `resolver_check`), keeping the library's public surface minimal.
//!
//! Two notions of "broken":
//! - the full solver notion: a package is broken when no *consistent* version
//!   set satisfies it (transitive, version- and conflict-aware). Implemented
//!   on the library's public [`SharedSolver`], which builds the intern pool
//!   once and reuses it across every check.
//! - shallow checks (≈ `apt-cache unmet`): a package whose *direct*
//!   dependencies have no satisfying candidate, without any SAT solving.

use std::collections::{HashMap, HashSet};

use debian_control::lossy::Relation;
use debian_control::relations::VersionConstraint;
use debversion::Version;
use oma_apt_pkg::{AptDb, AptVersionSet, ParsedDeps, ResolveOptions, SharedSolver, solve_packages};

/// Reverse provides map: name → the real packages that provide it.
///
/// Mirrors the resolver's internal map (real names, the `X:any` aliases, the
/// `Architecture: all` expansion, and virtual names from `Provides:`),
/// rebuilt here from the public [`AptDb`] API so this tool needs no
/// library internals.
struct ProvidesMap {
    providers: HashMap<String, Vec<String>>,
}

impl ProvidesMap {
    fn build(index: &AptDb) -> Self {
        // Real names first, so the alias pass can ask "does `X:all` exist?"
        // while registering arch aliases.
        let real_names: Vec<&str> = index
            .packages()
            .filter(|name| !index.versions(name).is_empty())
            .collect();
        let mut providers: HashMap<String, Vec<String>> = HashMap::new();

        for name in &real_names {
            providers
                .entry(name.to_string())
                .or_default()
                .push(name.to_string());
        }

        // Multi-arch aliases:
        // - `X:any` matches every architecture of X;
        // - a specific `X:arch` reference is also satisfied by the
        //   `Architecture: all` package `X:all`, which serves any arch.
        let real_set: HashSet<&str> = real_names.iter().copied().collect();
        for name in &real_names {
            let Some((base, arch)) = name.rsplit_once(':') else {
                continue;
            };
            let any_name = format!("{base}:any");
            providers
                .entry(any_name.clone())
                .or_default()
                .push(name.to_string());
            if arch != "all" {
                let all_name = format!("{base}:all");
                if real_set.contains(all_name.as_str()) {
                    let all_providers = providers.get(&all_name).cloned().unwrap_or_default();
                    providers
                        .entry(name.to_string())
                        .or_default()
                        .extend(all_providers.iter().cloned());
                    providers.entry(any_name).or_default().extend(all_providers);
                }
            }
        }

        // Virtual names from `Provides:` fields, registered bare (cross-arch).
        for name in index.packages() {
            let entries = index.versions(name);
            for version in entries.iter() {
                let Some(version_str) = version.entry.version.as_deref() else {
                    continue;
                };
                let Some(deps) = index.deps_of(name, version_str) else {
                    continue;
                };
                for dep in &deps.provides {
                    providers
                        .entry(dep.name.clone())
                        .or_default()
                        .push(name.to_string());
                    providers
                        .entry(format!("{}:any", dep.name))
                        .or_default()
                        .push(name.to_string());
                }
            }
        }

        Self { providers }
    }

    /// Whether `name` is provided by a *different* package (a provider could
    /// stand in for it — e.g. a transitional package).
    fn provided_by_other(&self, name: &str) -> bool {
        self.providers
            .get(name)
            .is_some_and(|ps| ps.iter().any(|p| p != name))
    }

    fn contains(&self, name: &str) -> bool {
        self.providers.contains_key(name)
    }

    fn providers_of(&self, name: &str) -> Option<&Vec<String>> {
        self.providers.get(name)
    }
}

/// Reusable scanner for broken packages.
///
/// Builds a [`SharedSolver`] (intern pool + provides/solvables reverse
/// indexes) and the provides map once, then checks many package names
/// against them — a whole-archive scan reuses the pool instead of
/// re-interning the universe per package. The
/// [`BrokenScanner::direct_deps_unsatisfiable`] prefilter short-circuits
/// the obvious cases without a SAT solve.
pub struct BrokenScanner<'a> {
    index: &'a AptDb,
    solver: SharedSolver<'a>,
    provides: ProvidesMap,
}

impl<'a> BrokenScanner<'a> {
    /// Build a scanner over `index`.
    pub fn new(index: &'a AptDb) -> Self {
        Self {
            index,
            solver: SharedSolver::new(index, None, ResolveOptions::default()),
            provides: ProvidesMap::build(index),
        }
    }

    /// Whether `name` cannot be part of any consistent version set.
    pub fn is_broken(&mut self, name: &str) -> bool {
        self.check(name).is_err()
    }

    /// Solve `name` alone.
    ///
    /// On success returns the names of every package in the resolved closure.
    /// Since they form one consistent version set, all of them are provably
    /// *not* broken — callers can use this to skip re-solving them.
    #[allow(clippy::result_unit_err)]
    pub fn check(&mut self, name: &str) -> Result<HashSet<String>, ()> {
        match self.solver.solve(&[(name, AptVersionSet::Any)]) {
            Ok(solution) => Ok(solution.into_iter().map(|(n, _)| n).collect()),
            Err(_) => Err(()),
        }
    }

    /// Cheap prefilter: is `name` plainly broken by its direct dependencies?
    ///
    /// Returns true only when `name` is *not* provided by any other package
    /// (a provider could stand in for it — e.g. a transitional package) and
    /// *every* version entry of `name` has an unsatisfiable
    /// `Pre-Depends`/`Depends`/`Recommends` OR-group (no alternative's name
    /// exists or is provided by anything). This is a strict subset of the
    /// solver's notion of broken — it never produces false positives, it just
    /// catches the obvious cases without a SAT solve.
    pub fn direct_deps_unsatisfiable(&self, name: &str) -> bool {
        // If another package provides `name`, it may be chosen instead of
        // `name`'s own (possibly broken) candidate, so `name`'s entries don't
        // prove it broken — defer to the full solver.
        if self.provides.provided_by_other(name) {
            return false;
        }

        let entries = self.index.versions(name);
        if entries.is_empty() {
            return false;
        }
        entries.iter().all(|version| {
            let Some(version_str) = version.entry.version.as_deref() else {
                return false;
            };
            let Some(deps) = self.index.deps_of(name, version_str) else {
                return false;
            };
            deps.pre_depends
                .iter()
                .chain(deps.depends.iter())
                .chain(deps.recommends.iter())
                .any(|group| !group.iter().any(|alt| self.alt_name_provided(&alt.name)))
        })
    }

    /// Whether a dependency name is satisfiable by something: it is either a
    /// real package or provided by one (the provides map).
    fn alt_name_provided(&self, alt_name: &str) -> bool {
        self.provides.contains(alt_name)
    }

    /// Fast shallow check (≈ `apt-cache unmet` semantics): is the *candidate*
    /// of `name` broken by its direct dependencies?
    ///
    /// This does no SAT solving — no transitive closure, no version
    /// consistency — so it runs at roughly apt's speed. It therefore does
    /// *not* detect transitive breakage: a package whose direct dependencies
    /// all have a satisfiable candidate passes even if its transitive closure
    /// is broken (e.g. `blender` depending on a broken package). Use
    /// [`BrokenScanner::check`] for the full, transitive answer.
    pub fn shallow_is_broken(&self, name: &str) -> bool {
        let Some(candidate) = self.index.candidate_version(name) else {
            return false;
        };
        let Some(version) = candidate.entry.version.as_deref() else {
            return false;
        };
        let Some(deps) = self.index.deps_of(name, version) else {
            return false;
        };
        self.entry_direct_unsatisfiable(&deps)
    }

    /// Shallow check over *all* versions: true when **any** version entry of
    /// `name` is broken by its direct dependencies.
    ///
    /// This matches `apt-cache unmet`'s notion — it reports a package when
    /// *some* version has an unmet dependency — so it is the comparable
    /// question for side-by-side comparison. Note it is the *opposite* of
    /// installability: a package with one broken old version and a fine newer
    /// one is still installable (the full solver says so), yet flagged here.
    pub fn shallow_is_broken_any_version(&self, name: &str) -> bool {
        !self.shallow_broken_versions(name).is_empty()
    }

    /// The versions of `name` that are broken by their direct dependencies
    /// (empty when none). See [`BrokenScanner::shallow_is_broken_any_version`].
    pub fn shallow_broken_versions(&self, name: &str) -> Vec<String> {
        let index = self.index;
        let entries = index.versions(name);
        let mut versions: Vec<String> = entries
            .iter()
            .filter(|version| {
                version
                    .entry
                    .version
                    .as_deref()
                    .and_then(|v| index.deps_of(name, v))
                    .is_some_and(|deps| self.entry_direct_unsatisfiable(&deps))
            })
            .filter_map(|version| version.entry.version.clone())
            .collect();
        versions.sort();
        versions.dedup();
        versions
    }

    /// Whether a single package entry has an unsatisfiable direct
    /// `Pre-Depends`/`Depends`/`Recommends` OR-group.
    fn entry_direct_unsatisfiable(&self, deps: &ParsedDeps) -> bool {
        for group in deps
            .pre_depends
            .iter()
            .chain(deps.depends.iter())
            .chain(deps.recommends.iter())
        {
            if !group.iter().any(|alt| self.alt_satisfiable(alt)) {
                return true;
            }
        }

        false
    }

    /// Whether a single dependency alternative is satisfiable by some
    /// candidate (of the package itself or of a provider).
    fn alt_satisfiable(&self, alt: &Relation) -> bool {
        let Some(providers) = self.provides.providers_of(&alt.name) else {
            return false;
        };
        let Some((relation, want)) = &alt.version else {
            // No version constraint: satisfied as long as the name exists.
            return true;
        };
        providers.iter().any(|provider_name| {
            let Some(candidate) = self.index.candidate_version(provider_name) else {
                return false;
            };
            let Some(cand_ver) = candidate
                .entry
                .version
                .as_deref()
                .and_then(|v| Version::parse_lenient(v).ok())
            else {
                return false;
            };
            match relation {
                VersionConstraint::LessThan => cand_ver < *want,
                VersionConstraint::LessThanEqual => cand_ver <= *want,
                VersionConstraint::Equal => cand_ver == *want,
                VersionConstraint::GreaterThanEqual => cand_ver >= *want,
                VersionConstraint::GreaterThan => cand_ver > *want,
            }
        })
    }
}

/// Report which of `names` cannot be part of any consistent version set.
///
/// The provides map is built once and reused across all checks. Two
/// optimizations keep this fast:
/// - a cheap direct-dependency prefilter short-circuits the obvious cases
/// - a successful solve proves its whole closure is not broken, so those
///   packages are skipped instead of being re-solved
pub fn find_broken(index: &AptDb, names: &[&str]) -> Vec<String> {
    let mut scanner = BrokenScanner::new(index);
    let mut known_ok: HashSet<String> = HashSet::new();
    let mut broken = Vec::new();
    for name in names {
        if known_ok.contains(*name) {
            continue;
        }
        if scanner.direct_deps_unsatisfiable(name) {
            broken.push(name.to_string());
            continue;
        }
        match scanner.check(name) {
            Ok(closure) => known_ok.extend(closure),
            Err(()) => broken.push(name.to_string()),
        }
    }
    broken
}

/// Whether a single package is broken — it cannot be part of any consistent
/// version set.
pub fn package_is_broken(index: &AptDb, name: &str) -> bool {
    solve_packages(index, &[name]).is_err()
}

#[cfg(test)]
mod tests {
    use super::*;
    use oma_apt_pkg::{AptDb, PackageEntry};

    fn base_entry() -> PackageEntry {
        PackageEntry {
            architecture: Some("amd64".to_string()),
            ..PackageEntry::default()
        }
    }

    fn entry(
        name: &str,
        version: &str,
        depends: Option<&str>,
        provides: Option<&str>,
        conflicts: Option<&str>,
        breaks: Option<&str>,
    ) -> PackageEntry {
        PackageEntry {
            package: name.to_string(),
            version: Some(version.to_string()),
            depends: depends.map(str::to_string),
            provides: provides.map(str::to_string),
            conflicts: conflicts.map(str::to_string),
            breaks: breaks.map(str::to_string),
            ..base_entry()
        }
    }

    fn db(entries: Vec<PackageEntry>) -> AptDb {
        AptDb::from_entries("", entries)
    }

    #[test]
    fn test_package_is_broken() {
        let index = db(vec![
            entry("app", "1.0", Some("missing-pkg"), None, None, None),
            entry("ok", "1.0", None, None, None, None),
        ]);
        assert!(package_is_broken(&index, "app"));
        assert!(!package_is_broken(&index, "ok"));
    }

    #[test]
    fn test_find_broken() {
        let index = db(vec![
            entry("app", "1.0", Some("missing-pkg"), None, None, None),
            entry("ok", "1.0", None, None, None, None),
        ]);
        let broken = find_broken(&index, &["app", "ok", "nonexistent"]);
        // "app" is broken; "ok" is fine; "nonexistent" has no candidates so
        // its requirement is trivially unsolvable too.
        assert_eq!(broken, vec!["app".to_string(), "nonexistent".to_string()]);
    }

    #[test]
    fn test_find_broken_memoization_matches_naive() {
        // A satisfiable chain app → liba → libb plus a broken package `bad`
        // (missing dep) and a transitive user of it.
        let index = db(vec![
            entry("app", "1.0", Some("liba"), None, None, None),
            entry("liba", "1.0", Some("libb"), None, None, None),
            entry("libb", "1.0", None, None, None, None),
            entry("bad", "1.0", Some("missing-pkg"), None, None, None),
            entry("user", "1.0", Some("bad"), None, None, None),
        ]);
        let names = ["app", "liba", "libb", "bad", "user"];
        let mut naive = Vec::new();
        {
            let mut scanner = BrokenScanner::new(&index);
            for name in names {
                if scanner.is_broken(name) {
                    naive.push(name.to_string());
                }
            }
        }
        let memoized = find_broken(&index, &names);
        // Both must agree, and the satisfiable chain must not be flagged.
        assert_eq!(memoized, naive);
        assert_eq!(memoized, vec!["bad".to_string(), "user".to_string()]);
    }

    #[test]
    fn test_find_broken_provider_shadow_no_false_positive() {
        // `shadow` is a real package with an unsatisfiable direct dependency,
        // but `provider` Provides `shadow` and is itself fine — so requiring
        // `shadow` can be satisfied by installing `provider` instead. The
        // direct-deps prefilter must not claim `shadow` is definitely broken.
        let index = db(vec![
            entry("shadow", "1.0", Some("missing-pkg"), None, None, None),
            entry("provider", "1.0", None, Some("shadow"), None, None),
        ]);
        let broken = find_broken(&index, &["shadow", "provider"]);
        assert_eq!(broken, Vec::<String>::new());
    }
}
