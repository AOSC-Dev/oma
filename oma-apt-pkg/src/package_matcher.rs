//! Package matching — resolve patterns (exact / glob / version / branch) to
//! package entries

use std::borrow::Cow;

use glob_match::glob_match;

use crate::apt_db::AptDb;
use crate::apt_lists::{AptListsError, PackageVersion};

/// Errors produced by [`PackageMatcher`].
#[derive(Debug, thiserror::Error)]
pub enum MatcherError {
    #[error("Can not find package {0} from database")]
    NoPackage(String),
    #[error("Pkg {0} has no version {1}")]
    NoVersion(String, String),
    #[error(transparent)]
    AptLists(#[from] AptListsError),
}

pub type MatcherResult<T> = Result<T, MatcherError>;

/// A group of matched versions — one per query keyword.
///
/// A group holds the (possibly filtered) versions of one package, each
/// carrying the sources it is available from. Versions are [`Cow`]s:
/// borrowed from the index when no filtering needed them owned, owned
/// otherwise.
pub type MatchGroup<'a> = Vec<Cow<'a, PackageVersion>>;

/// Resolves user-supplied keywords into matched packages.
///
/// # Example
///
/// ```ignore
/// let matcher = PackageMatcher::new(&apt_db);
/// let (matched, no_result) =
///     matcher.match_pkgs_and_versions(["fish", "apt*", "apt=2.5.4"].into_iter())?;
/// ```
pub struct PackageMatcher<'a> {
    index: &'a AptDb,
}

impl<'a> PackageMatcher<'a> {
    /// Create a matcher over the given package database.
    pub fn new(index: &'a AptDb) -> Self {
        Self { index }
    }

    /// Match each keyword against the index.
    ///
    /// An architecture-qualified name like `apt:amd64` is treated as a
    /// package name (see [`has_package`](Self::has_package)); the rest
    /// dispatches:
    /// - contains `=` → [`match_from_version`](Self::match_from_version)
    /// - contains `/` → [`match_from_branch`](Self::match_from_branch)
    /// - otherwise → [`match_pkgs_and_versions_from_glob`](Self::match_pkgs_and_versions_from_glob)
    ///
    /// Returns the matched version groups and the unmatched keywords. The
    /// returned groups borrow the index; the keyword borrow is only used for
    /// `no_result`.
    pub fn match_pkgs_and_versions<'k>(
        &self,
        keywords: impl IntoIterator<Item = &'k str>,
    ) -> MatcherResult<(Vec<MatchGroup<'a>>, Vec<&'k str>)> {
        let mut pkgs = Vec::new();
        let mut no_result = Vec::new();

        for keyword in keywords {
            let res = if let Some((name, version)) = keyword.split_once('=') {
                self.match_from_version(name, version)?
            } else if let Some((name, branch)) = keyword.split_once('/') {
                self.match_from_branch(name, branch)?
            } else {
                self.match_pkgs_and_versions_from_glob(keyword)?
            };

            if res.is_empty() {
                no_result.push(keyword);
            } else {
                pkgs.extend(res);
            }
        }

        Ok((pkgs, no_result))
    }

    /// Whether `name` — possibly an architecture-qualified name like
    /// `apt:amd64` — is available in the index.
    ///
    /// An arch-qualified name resolves to the package of that name, filtered
    /// by its `Architecture` (see [`arch_matches`]).
    fn has_package(&self, name: &str) -> bool {
        let (pkg, arch) = split_arch(name);
        self.index.has_package(pkg)
            && arch.is_none_or(|arch| {
                self.index
                    .versions(pkg)
                    .iter()
                    .any(|version| arch_matches(&version.entry.architecture, arch))
            })
    }

    /// All versions of `name` — possibly an architecture-qualified name like
    /// `apt:amd64` — lazily, as borrowed-or-owned [`Cow`]s: borrowed storage
    /// is handed out zero-copy, owned storage is moved out. Callers apply
    /// arch/version/branch filtering and collect exactly once.
    fn versions_of(&self, name: &str) -> Box<dyn Iterator<Item = Cow<'a, PackageVersion>> + 'a> {
        match self.index.versions(name) {
            Cow::Borrowed(slice) => Box::new(slice.iter().map(Cow::Borrowed)),
            Cow::Owned(vec) => Box::new(vec.into_iter().map(Cow::Owned)),
        }
    }

    /// Match packages from a glob pattern (like `apt*`). A pattern without
    /// wildcards falls back to exact name matching; an architecture-qualified
    /// name like `apt:amd64` matches that build directly.
    pub fn match_pkgs_and_versions_from_glob<'k>(
        &self,
        glob: &'k str,
    ) -> MatcherResult<Vec<MatchGroup<'a>>> {
        let mut res = Vec::new();
        if self.has_package(glob) {
            let (pkg, arch) = split_arch(glob);
            res.push(
                self.versions_of(pkg)
                    .filter(|v| arch_matches_or(v, arch))
                    .collect(),
            );
        } else {
            for name in self.index.packages().filter(|p| glob_match(glob, p)) {
                res.push(self.versions_of(name).collect());
            }
        }

        Ok(res)
    }

    /// Match a package against an exact version (like `apt=2.5.4` or
    /// `apt:amd64=2.5.4`).
    ///
    /// Takes the already-split name and version: the dispatcher parses the
    /// `name=version` pattern, and local `.deb`s hand over their control
    /// file's `(name, version)` directly instead of round-tripping through a
    /// pattern string.
    pub fn match_from_version(
        &self,
        name: &str,
        version: &str,
    ) -> MatcherResult<Vec<MatchGroup<'a>>> {
        if !self.has_package(name) {
            return Err(MatcherError::NoPackage(name.to_string()));
        }

        let (pkg, arch) = split_arch(name);
        let versions: Vec<Cow<'a, PackageVersion>> = self
            .versions_of(pkg)
            .filter(|v| {
                arch_matches_or(v, arch) && v.entry.version.as_deref().is_some_and(|v| v == version)
            })
            .collect();

        if versions.is_empty() {
            return Err(MatcherError::NoVersion(
                name.to_string(),
                version.to_string(),
            ));
        }

        Ok(vec![versions])
    }

    /// Match a package against a branch (like `apt/stable` or
    /// `apt:amd64/stable`).
    ///
    /// Takes the already-split name and branch, like
    /// [`match_from_version`](Self::match_from_version). A package is
    /// matched by the suite of the source it came from, i.e. its recorded
    /// [`IndexSource::suite`] equals the branch.
    pub fn match_from_branch(
        &self,
        name: &str,
        branch: &str,
    ) -> MatcherResult<Vec<MatchGroup<'a>>> {
        if !self.has_package(name) {
            return Err(MatcherError::NoPackage(name.to_string()));
        }

        // Keep only the versions available from the branch, trimming their
        // sources to the branch itself. Only the survivors are cloned.
        let (pkg, arch) = split_arch(name);
        let versions: Vec<Cow<'a, PackageVersion>> = self
            .versions_of(pkg)
            .filter_map(|v| {
                if !arch_matches_or(&v, arch) || !v.sources.iter().any(|s| s.suite == branch) {
                    return None;
                }
                let mut owned = v.into_owned();
                owned.sources.retain(|s| s.suite == branch);
                Some(Cow::Owned(owned))
            })
            .collect();

        if versions.is_empty() {
            return Ok(Vec::new());
        }

        Ok(vec![versions])
    }
}

/// Split an arch-qualified name into (package, arch qualifier).
fn split_arch(name: &str) -> (&str, Option<&str>) {
    match name.split_once(':') {
        Some((pkg, arch)) => (pkg, Some(arch)),
        None => (name, None),
    }
}

/// Whether `v` satisfies an `:arch` qualifier (`None` = any arch).
fn arch_matches_or(v: &PackageVersion, arch: Option<&str>) -> bool {
    arch.is_none_or(|a| arch_matches(&v.entry.architecture, a))
}

/// Whether an entry's `Architecture` satisfies an `:arch` qualifier.
fn arch_matches(entry_arch: &Option<String>, arch: &str) -> bool {
    match arch {
        // `any`/`native` are satisfied by any build.
        "any" | "native" => true,
        "all" => entry_arch.as_deref().is_some_and(|a| a == "all"),
        specific => entry_arch
            .as_deref()
            .is_some_and(|a| a == specific || a == "all"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use crate::apt_db::AptDb;
    use crate::apt_lists::IndexSource;

    fn entry(name: &str, version: &str, arch: Option<&str>) -> crate::PackageEntry {
        crate::PackageEntry {
            package: name.to_string(),
            version: Some(version.to_string()),
            architecture: arch.map(str::to_string),
            ..crate::PackageEntry {
                package: String::new(),
                version: None,
                architecture: None,
                description: None,
                description_md5: None,
                maintainer: None,
                installed_size: None,
                depends: None,
                pre_depends: None,
                recommends: None,
                suggests: None,
                breaks: None,
                conflicts: None,
                replaces: None,
                provides: None,
                section: None,
                priority: None,
                homepage: None,
                multi_arch: None,
                filename: None,
                size: None,
                sha256: None,
                essential: None,
                protected: None,
            }
        }
    }

    fn source(base: &str, suite: &str, component: &str, arch: &str) -> IndexSource {
        IndexSource {
            base_url: base.to_string(),
            suite: suite.to_string(),
            component: Some(component.to_string()),
            arch: Some(arch.to_string()),
        }
    }

    fn matcher() -> PackageMatcher<'static> {
        let stable = source("http://repo", "stable", "main", "amd64");
        let preview = source("http://repo", "preview", "main", "amd64");
        let db = AptDb::from_entries_with_sources(
            "",
            vec![
                entry("fish", "4.5.0", Some("amd64")),
                entry("apt", "2.5.4", Some("amd64")),
                entry("bash", "5.2.3", Some("amd64")),
                entry("fish", "4.8.1", Some("amd64")),
                entry("firefox", "130.0", Some("amd64")),
            ],
            vec![
                stable.clone(),
                stable.clone(),
                stable.clone(),
                preview.clone(),
                preview.clone(),
            ],
        );
        // Leak the database to get a 'static reference for testing.
        let db: &'static AptDb = Box::leak(Box::new(db));
        let matcher = PackageMatcher::new(db);
        matcher
    }

    fn names<'a>(groups: &'a [MatchGroup<'a>]) -> Vec<&'a str> {
        groups.iter().map(|g| g[0].entry.package.as_str()).collect()
    }

    #[test]
    fn test_exact_name() {
        let m = matcher();
        let res = m.match_pkgs_and_versions_from_glob("fish").unwrap();
        assert_eq!(names(&res), vec!["fish"]);
        // Both versions of fish (one per source repo)
        assert_eq!(res[0].len(), 2);
    }

    #[test]
    fn test_glob_match() {
        let m = matcher();
        let res = m.match_pkgs_and_versions_from_glob("fi*").unwrap();
        let mut n = names(&res);
        n.sort();
        assert_eq!(n, vec!["firefox", "fish"]);
    }

    #[test]
    fn test_no_match() {
        let m = matcher();
        let res = m.match_pkgs_and_versions_from_glob("zzz*").unwrap();
        assert!(res.is_empty());
    }

    #[test]
    fn test_match_from_version() {
        let m = matcher();
        let res = m.match_from_version("apt", "2.5.4").unwrap();
        assert_eq!(names(&res), vec!["apt"]);
        assert_eq!(res[0].len(), 1);
        assert_eq!(res[0][0].entry.version.as_deref(), Some("2.5.4"));
    }

    #[test]
    fn test_match_from_version_not_found() {
        let m = matcher();
        assert!(matches!(
            m.match_from_version("apt", "9.9.9"),
            Err(MatcherError::NoVersion(_, _))
        ));
        assert!(matches!(
            m.match_from_version("nosuchpkg", "1.0"),
            Err(MatcherError::NoPackage(_))
        ));
    }

    #[test]
    fn test_match_from_branch() {
        let m = matcher();
        let res = m.match_from_branch("fish", "stable").unwrap();
        assert_eq!(names(&res), vec!["fish"]);
        assert_eq!(res[0].len(), 1);
        assert_eq!(res[0][0].entry.version.as_deref(), Some("4.5.0"));

        let res = m.match_from_branch("fish", "preview").unwrap();
        assert_eq!(res[0][0].entry.version.as_deref(), Some("4.8.1"));
    }

    #[test]
    fn test_match_pkgs_and_versions_dispatch() {
        let m = matcher();
        let (pkgs, no_result) = m
            .match_pkgs_and_versions(["fish", "apt=2.5.4", "bash/stable", "missing"].into_iter())
            .unwrap();
        assert_eq!(no_result, vec!["missing"]);
        let mut n = names(&pkgs);
        n.sort();
        assert_eq!(n, vec!["apt", "bash", "fish"]);
    }

    #[test]
    fn test_match_pkgs_and_versions_arch_dispatch() {
        let m = matcher();
        let (pkgs, no_result) = m
            .match_pkgs_and_versions(["apt:amd64", "missing"].into_iter())
            .unwrap();
        assert_eq!(no_result, vec!["missing"]);
        // Arch-qualified lookup resolves the package and returns only the
        // amd64 build.
        assert_eq!(pkgs[0][0].entry.package, "apt");
        assert!(pkgs[0].iter().all(|v| {
            v.entry
                .architecture
                .as_deref()
                .is_some_and(|a| a == "amd64")
        }));
    }

    #[test]
    fn test_match_pkgs_and_versions_combined_qualifiers() {
        let m = matcher();
        let (pkgs, no_result) = m
            .match_pkgs_and_versions(["apt:amd64=2.5.4", "fish:amd64/stable"].into_iter())
            .unwrap();
        assert!(no_result.is_empty());

        // arch + version: `apt:amd64=2.5.4`
        let apt = pkgs.iter().find(|g| g[0].entry.package == "apt").unwrap();
        assert_eq!(apt.len(), 1);
        assert_eq!(apt[0].entry.version.as_deref(), Some("2.5.4"));
        assert_eq!(apt[0].entry.architecture.as_deref(), Some("amd64"));

        // arch + branch: `fish:amd64/stable` → the stable build
        let fish = pkgs.iter().find(|g| g[0].entry.package == "fish").unwrap();
        assert_eq!(fish.len(), 1);
        assert_eq!(fish[0].entry.version.as_deref(), Some("4.5.0"));
    }
}
