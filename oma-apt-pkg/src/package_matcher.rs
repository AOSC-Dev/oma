//! Package matching — resolve patterns (exact / glob / version / branch) to
//! package entries

use std::borrow::Cow;

use glob_match::glob_match;

use crate::apt_lists::{AptListsError, IndexSource, PackageEntry, PackageIndex};

/// Errors produced by [`PackageMatcher`].
#[derive(Debug, thiserror::Error)]
pub enum MatcherError {
    #[error("Invalid pattern: {0}")]
    InvalidPattern(String),
    #[error("Can not find package {0} from database")]
    NoPackage(String),
    #[error("Pkg {0} has no version {1}")]
    NoVersion(String, String),
    #[error(transparent)]
    AptLists(#[from] AptListsError),
}

pub type MatcherResult<T> = Result<T, MatcherError>;

/// A matched package together with its (possibly filtered) entries and their
/// sources.
#[derive(Debug)]
pub struct MatchedPackage<'a> {
    /// The package name.
    pub name: Cow<'a, str>,
    /// The entries for this package (all, or filtered by version/branch),
    /// each paired with its [`IndexSource`].
    pub entries: Vec<(Cow<'a, PackageEntry>, IndexSource)>,
}

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
    index: &'a dyn PackageIndex,
}

impl<'a> PackageMatcher<'a> {
    /// Create a matcher over the given package index.
    pub fn new(index: &'a dyn PackageIndex) -> Self {
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
    /// Returns the matched packages and the unmatched keywords. The returned
    /// entries borrow the index; the keyword borrow is only used for
    /// `no_result`.
    pub fn match_pkgs_and_versions<'k>(
        &self,
        keywords: impl IntoIterator<Item = &'k str>,
    ) -> MatcherResult<(Vec<MatchedPackage<'a>>, Vec<&'k str>)> {
        let mut pkgs = Vec::new();
        let mut no_result = Vec::new();

        for keyword in keywords {
            let res = match keyword {
                x if x.split_once('=').is_some() => self.match_from_version(x)?,
                x if x.split_once('/').is_some() => self.match_from_branch(x)?,
                x => self.match_pkgs_and_versions_from_glob(x)?,
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
        match name.split_once(':') {
            Some((pkg, arch)) => {
                self.index.has_package(pkg)
                    && self
                        .index
                        .get_all(pkg)
                        .iter()
                        .any(|version| arch_matches(&version.entry.architecture, arch))
            }
            None => self.index.has_package(name),
        }
    }

    /// Entries for `name` — possibly an architecture-qualified name like
    /// `apt:amd64` — together with their sources.
    fn entries_of(&self, name: &str) -> Vec<(Cow<'a, PackageEntry>, IndexSource)> {
        match name.split_once(':') {
            Some((pkg, arch)) => self
                .index
                .get_with_source(pkg)
                .filter(|(entry, _)| arch_matches(&entry.architecture, arch))
                .collect(),
            None => self.index.get_with_source(name).collect(),
        }
    }

    /// Match packages from a glob pattern (like `apt*`). A pattern without
    /// wildcards falls back to exact name matching; an architecture-qualified
    /// name like `apt:amd64` matches that build directly.
    pub fn match_pkgs_and_versions_from_glob<'k>(
        &self,
        glob: &'k str,
    ) -> MatcherResult<Vec<MatchedPackage<'a>>> {
        let mut res = Vec::new();
        if self.has_package(glob) {
            res.push(MatchedPackage {
                name: Cow::Owned(glob.to_string()),
                entries: self.entries_of(glob),
            });
        } else {
            for name in self.index.packages().filter(|p| glob_match(glob, p)) {
                res.push(MatchedPackage {
                    name: Cow::Borrowed(name),
                    entries: self.entries_of(name),
                });
            }
        }

        Ok(res)
    }

    /// Match package from a version pattern (like `apt=2.5.4` or
    /// `apt:amd64=2.5.4`).
    pub fn match_from_version<'k>(&self, pat: &'k str) -> MatcherResult<Vec<MatchedPackage<'a>>> {
        let (pkgname, version_str) = pat
            .split_once('=')
            .ok_or_else(|| MatcherError::InvalidPattern(pat.to_string()))?;

        if !self.has_package(pkgname) {
            return Err(MatcherError::NoPackage(pat.to_string()));
        }

        let entries: Vec<(Cow<'a, PackageEntry>, IndexSource)> = self
            .entries_of(pkgname)
            .into_iter()
            .filter(|(entry, _)| entry.version.as_deref() == Some(version_str))
            .collect();

        if entries.is_empty() {
            return Err(MatcherError::NoVersion(
                pkgname.to_string(),
                version_str.to_string(),
            ));
        }

        Ok(vec![MatchedPackage {
            name: Cow::Owned(pkgname.to_string()),
            entries,
        }])
    }

    /// Match package from a branch pattern (like `apt/stable` or
    /// `apt:amd64/stable`).
    ///
    /// A package is matched by the suite of the source it came from, i.e.
    /// its recorded [`IndexSource::suite`] equals the branch.
    pub fn match_from_branch<'k>(&self, pat: &'k str) -> MatcherResult<Vec<MatchedPackage<'a>>> {
        let (pkgname, branch) = pat
            .split_once('/')
            .ok_or_else(|| MatcherError::InvalidPattern(pat.to_string()))?;

        if !self.has_package(pkgname) {
            return Err(MatcherError::NoPackage(pat.to_string()));
        }

        let entries: Vec<(Cow<'a, PackageEntry>, IndexSource)> = self
            .entries_of(pkgname)
            .into_iter()
            .filter(|(_, source)| source.suite == branch)
            .collect();

        if entries.is_empty() {
            return Ok(Vec::new());
        }

        Ok(vec![MatchedPackage {
            name: Cow::Owned(pkgname.to_string()),
            entries,
        }])
    }
}

/// Whether an entry's `Architecture` satisfies an `:arch` qualifier.
fn arch_matches(entry_arch: &Option<String>, arch: &str) -> bool {
    match arch {
        // `any`/`native` are satisfied by any build.
        "any" | "native" => true,
        "all" => entry_arch.as_deref() == Some("all"),
        specific => entry_arch.as_deref() == Some(specific) || entry_arch.as_deref() == Some("all"),
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

    fn matcher() -> ((), PackageMatcher<'static>) {
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
        ((), matcher)
    }

    fn names<'a>(pkgs: &'a [MatchedPackage<'a>]) -> Vec<Cow<'a, str>> {
        pkgs.iter().map(|p| p.name.clone()).collect()
    }

    #[test]
    fn test_exact_name() {
        let (_d, m) = matcher();
        let res = m.match_pkgs_and_versions_from_glob("fish").unwrap();
        assert_eq!(names(&res), vec!["fish"]);
        // Both repos' entries for fish
        assert_eq!(res[0].entries.len(), 2);
    }

    #[test]
    fn test_glob_match() {
        let (_d, m) = matcher();
        let res = m.match_pkgs_and_versions_from_glob("fi*").unwrap();
        let mut n = names(&res);
        n.sort();
        assert_eq!(n, vec!["firefox", "fish"]);
    }

    #[test]
    fn test_no_match() {
        let (_d, m) = matcher();
        let res = m.match_pkgs_and_versions_from_glob("zzz*").unwrap();
        assert!(res.is_empty());
    }

    #[test]
    fn test_match_from_version() {
        let (_d, m) = matcher();
        let res = m.match_from_version("apt=2.5.4").unwrap();
        assert_eq!(names(&res), vec!["apt"]);
        assert_eq!(res[0].entries.len(), 1);
        assert_eq!(res[0].entries[0].0.version.as_deref(), Some("2.5.4"));
    }

    #[test]
    fn test_match_from_version_not_found() {
        let (_d, m) = matcher();
        assert!(matches!(
            m.match_from_version("apt=9.9.9"),
            Err(MatcherError::NoVersion(_, _))
        ));
        assert!(matches!(
            m.match_from_version("nosuchpkg=1.0"),
            Err(MatcherError::NoPackage(_))
        ));
    }

    #[test]
    fn test_match_from_branch() {
        let (_d, m) = matcher();
        let res = m.match_from_branch("fish/stable").unwrap();
        assert_eq!(names(&res), vec!["fish"]);
        assert_eq!(res[0].entries.len(), 1);
        assert_eq!(res[0].entries[0].0.version.as_deref(), Some("4.5.0"));

        let res = m.match_from_branch("fish/preview").unwrap();
        assert_eq!(res[0].entries[0].0.version.as_deref(), Some("4.8.1"));
    }

    #[test]
    fn test_match_pkgs_and_versions_dispatch() {
        let (_d, m) = matcher();
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
        let (_d, m) = matcher();
        let (pkgs, no_result) = m
            .match_pkgs_and_versions(["apt:amd64", "missing"].into_iter())
            .unwrap();
        assert_eq!(no_result, vec!["missing"]);
        assert_eq!(names(&pkgs), vec!["apt:amd64"]);
        // Arch-qualified lookup returns only the amd64 build.
        assert!(
            pkgs[0]
                .entries
                .iter()
                .all(|(e, _)| e.architecture.as_deref() == Some("amd64"))
        );
    }

    #[test]
    fn test_match_pkgs_and_versions_combined_qualifiers() {
        let (_d, m) = matcher();
        let (pkgs, no_result) = m
            .match_pkgs_and_versions(["apt:amd64=2.5.4", "fish:amd64/stable"].into_iter())
            .unwrap();
        assert!(no_result.is_empty());

        // arch + version: `apt:amd64=2.5.4`
        let apt = pkgs.iter().find(|p| p.name == "apt:amd64").unwrap();
        assert_eq!(apt.entries.len(), 1);
        assert_eq!(apt.entries[0].0.version.as_deref(), Some("2.5.4"));
        assert_eq!(apt.entries[0].0.architecture.as_deref(), Some("amd64"));

        // arch + branch: `fish:amd64/stable` → the stable build
        let fish = pkgs.iter().find(|p| p.name == "fish:amd64").unwrap();
        assert_eq!(fish.entries.len(), 1);
        assert_eq!(fish.entries[0].0.version.as_deref(), Some("4.5.0"));
    }
}
