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

/// A keyword as the matcher reads it: a package name or glob, optionally
/// with an `=version` and a `/branch` select — `pkg`, `pkg=1.0`,
/// `pkg/suite`, `pkg:arch=1.0`.
///
/// The one place keywords are parsed, so the index and a source that holds
/// entries the index does not (the dpkg status file) read them alike.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Keyword<'a> {
    name: &'a str,
    version: Option<&'a str>,
    branch: Option<&'a str>,
}

impl<'a> Keyword<'a> {
    pub(crate) fn parse(keyword: &'a str) -> Self {
        let (name, version) = match keyword.split_once('=') {
            Some((name, version)) => (name, Some(version)),
            None => (keyword, None),
        };
        let (name, branch) = match name.split_once('/') {
            Some((name, branch)) => (name, Some(branch)),
            None => (name, None),
        };

        Self {
            name,
            version,
            branch,
        }
    }

    /// The package name, architecture qualifier included — what
    /// [`PackageMatcher::match_from_version`] and
    /// [`PackageMatcher::match_from_branch`] take.
    pub(crate) fn name(&self) -> &'a str {
        self.name
    }

    pub(crate) fn version(&self) -> Option<&'a str> {
        self.version
    }

    pub(crate) fn branch(&self) -> Option<&'a str> {
        self.branch
    }

    /// The package pattern the keyword names — the name or glob, without the
    /// `:arch` qualifier, which only filters versions.
    pub(crate) fn pattern(&self) -> &'a str {
        split_arch(self.name).0
    }

    /// The `:arch` qualifier the keyword carries, which selects the builds
    /// to show.
    pub(crate) fn arch(&self) -> Option<&'a str> {
        split_arch(self.name).1
    }
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
            let res = self.match_keyword(keyword)?;

            if res.is_empty() {
                no_result.push(keyword);
            } else {
                pkgs.extend(res);
            }
        }

        Ok((pkgs, no_result))
    }

    /// Match one keyword — a name or glob, optionally with `=version`,
    /// `/branch` and `:arch` — against the index.
    ///
    /// [`match_pkgs_and_versions`](Self::match_pkgs_and_versions) and
    /// [`AptDb::resolve_queries`] share this dispatch, so a keyword is read
    /// the same way wherever it is resolved.
    pub(crate) fn match_keyword(&self, keyword: &str) -> MatcherResult<Vec<MatchGroup<'a>>> {
        let parsed = Keyword::parse(keyword);
        if let Some(version) = parsed.version() {
            self.match_from_version(parsed.name(), version)
        } else if let Some(branch) = parsed.branch() {
            self.match_from_branch(parsed.name(), branch)
        } else {
            self.match_pkgs_and_versions_from_glob(parsed.name())
        }
    }

    /// The records `name` selects, with an optional `:arch` qualifier, the
    /// way apt resolves the package: no qualifier and `:any` take this
    /// system's package when it has one and fall back to whatever
    /// architecture the name has; `:all` and `:native` name this system's
    /// package only; a foreign architecture selects that build only.
    fn view_for<'q>(&self, name: &str, qualifier: Option<&'q str>) -> Option<ArchView<'q>> {
        ArchView::for_versions(
            &self.index.versions(name),
            self.index.native_arch(),
            qualifier,
        )
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

    /// The versions of `name` that `view` selects.
    fn versions_in_view(&self, name: &str, view: ArchView<'_>) -> Vec<Cow<'a, PackageVersion>> {
        let native_arch = self.index.native_arch();
        self.versions_of(name)
            .filter(|version| view.matches(version, native_arch))
            .collect()
    }

    /// Match packages from a glob pattern (like `apt*`). A pattern without
    /// wildcards falls back to exact name matching; an architecture-qualified
    /// name like `apt:amd64` matches what apt calls that package.
    ///
    /// An `:arch` qualifier also applies to a glob (`vendor-*:i386`): it
    /// selects the records apt would show, whether the pattern names one
    /// package or several.
    pub fn match_pkgs_and_versions_from_glob<'k>(
        &self,
        glob: &'k str,
    ) -> MatcherResult<Vec<MatchGroup<'a>>> {
        let (pattern, qualifier) = split_arch(glob);
        let mut res = Vec::new();

        if self.index.has_package(pattern) {
            if let Some(view) = self.view_for(pattern, qualifier) {
                let versions = self.versions_in_view(pattern, view);
                if !versions.is_empty() {
                    res.push(versions);
                }
            }
        } else {
            for pkg in self
                .index
                .packages_iter()
                .filter(|p| glob_match(pattern, p.name()))
            {
                let Some(view) = self.view_for(pkg.name(), qualifier) else {
                    continue;
                };
                let versions = self.versions_in_view(pkg.name(), view);
                if !versions.is_empty() {
                    res.push(versions);
                }
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
        let (pkg, qualifier) = split_arch(name);
        let Some(view) = self.view_for(pkg, qualifier) else {
            return Err(MatcherError::NoPackage(name.to_string()));
        };

        let versions: Vec<Cow<'a, PackageVersion>> = self
            .versions_in_view(pkg, view)
            .into_iter()
            .filter(|v| v.entry.version.as_deref().is_some_and(|v| v == version))
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
        let (pkg, qualifier) = split_arch(name);
        let Some(view) = self.view_for(pkg, qualifier) else {
            return Err(MatcherError::NoPackage(name.to_string()));
        };

        // Keep only the versions available from the branch, trimming their
        // sources to the branch itself. Only the survivors are cloned.
        let versions: Vec<Cow<'a, PackageVersion>> = self
            .versions_in_view(pkg, view)
            .into_iter()
            .filter_map(|v| {
                if !v.sources.iter().any(|s| s.suite == branch) {
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

/// The package records a query selects, the way apt resolves an `:arch`
/// qualifier.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ArchView<'a> {
    /// This system's package: the native build and `Architecture: all`.
    Native,
    /// One foreign architecture's package.
    Foreign(&'a str),
    /// Whatever architectures the name has — apt's fallback for a plain or
    /// `:any` query when the package has no native-view build.
    Any,
}

impl<'a> ArchView<'a> {
    /// The view `qualifier` selects over `versions`, or `None` when the
    /// package has nothing apt would show for it.
    ///
    /// `:all` and `:native` name this system's package: an `Architecture:
    /// all` build belongs to it (apt keeps it next to the native build), it
    /// is not a filter for `all` records only. A plain query and `:any`
    /// prefer that package and otherwise fall back to the architectures the
    /// name has; a foreign qualifier selects that build only, never `all`.
    pub(crate) fn for_versions(
        versions: &[PackageVersion],
        native_arch: &str,
        qualifier: Option<&'a str>,
    ) -> Option<Self> {
        if versions.is_empty() {
            return None;
        }

        let has_native = versions
            .iter()
            .any(|version| native_view(version.entry.architecture.as_deref(), native_arch));

        match qualifier {
            Some("all" | "native") => has_native.then_some(Self::Native),
            Some(arch) if arch == native_arch => has_native.then_some(Self::Native),
            Some("any") | None => Some(if has_native { Self::Native } else { Self::Any }),
            Some(arch) => versions
                .iter()
                .any(|version| version.entry.architecture.as_deref() == Some(arch))
                .then_some(Self::Foreign(arch)),
        }
    }

    /// Whether `version` belongs to this view.
    pub(crate) fn matches(&self, version: &PackageVersion, native_arch: &str) -> bool {
        let entry_arch = version.entry.architecture.as_deref();
        match self {
            Self::Native => native_view(entry_arch, native_arch),
            Self::Foreign(arch) => entry_arch == Some(*arch),
            Self::Any => true,
        }
    }
}

/// Whether a record belongs to this system's package: the native build, an
/// `Architecture: all` build, or one whose field is missing.
fn native_view(entry_arch: Option<&str>, native_arch: &str) -> bool {
    matches!(entry_arch, None | Some("all")) || entry_arch == Some(native_arch)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn keyword_parse() {
        let parse = |keyword| Keyword::parse(keyword);

        let plain = parse("fish");
        assert_eq!(plain.name(), "fish");
        assert_eq!(plain.pattern(), "fish");
        assert_eq!(plain.arch(), None);
        assert_eq!(plain.version(), None);
        assert_eq!(plain.branch(), None);

        let version = parse("fish:amd64=3.7");
        assert_eq!(version.name(), "fish:amd64");
        assert_eq!(version.pattern(), "fish");
        assert_eq!(version.arch(), Some("amd64"));
        assert_eq!(version.version(), Some("3.7"));
        assert_eq!(version.branch(), None);

        let branch = parse("fish/stable");
        assert_eq!(branch.name(), "fish");
        assert_eq!(branch.pattern(), "fish");
        assert_eq!(branch.arch(), None);
        assert_eq!(branch.version(), None);
        assert_eq!(branch.branch(), Some("stable"));

        // A glob keeps its wildcards; only a qualifier is stripped.
        let glob = parse("fish-*:i386");
        assert_eq!(glob.pattern(), "fish-*");
        assert_eq!(glob.arch(), Some("i386"));
    }

    /// An `:arch` qualifier filters a glob the same way it filters an exact
    /// name — the build the query asks for, not every build that matches.
    #[test]
    fn arch_qualified_glob_filters_builds() {
        let db = AptDb::from_entries(
            "amd64",
            vec![
                entry("vendor-tool", "7", Some("amd64")),
                entry("vendor-tool", "7", Some("i386")),
                entry("vendor-helper", "1", Some("i386")),
            ],
        );
        let matcher = PackageMatcher::new(&db);

        let groups = matcher
            .match_pkgs_and_versions_from_glob("vendor-*:i386")
            .unwrap();
        let mut found: Vec<_> = groups
            .iter()
            .map(|group| {
                assert_eq!(group.len(), 1);
                group[0].entry.package.clone()
            })
            .collect();
        found.sort_unstable();
        assert_eq!(found, vec!["vendor-helper", "vendor-tool"]);

        let exact = matcher
            .match_pkgs_and_versions_from_glob("vendor-tool:i386")
            .unwrap();
        assert_eq!(exact.len(), 1);
        assert_eq!(
            exact[0][0].entry.architecture.as_deref(),
            Some("i386"),
            "the i386 build is the one the query asks for"
        );

        assert!(
            matcher
                .match_pkgs_and_versions_from_glob("vendor-*:arm64")
                .unwrap()
                .is_empty()
        );
    }

    /// The architecture views apt resolves for `show`:
    /// - a plain query and `:any` take this system's package and fall back
    ///   to a foreign build only when there is no native-view record;
    /// - `:all` and `:native` name this system's package (`Architecture:
    ///   all` included) and do not fall back;
    /// - a foreign qualifier selects that build only, never `all`.
    #[test]
    fn architecture_views_follow_apt() {
        let db = AptDb::from_entries(
            "amd64",
            vec![
                entry("mixed", "1", Some("all")),
                entry("mixed", "2", Some("i386")),
                entry("native", "1", Some("all")),
                entry("native", "2", Some("amd64")),
                entry("foreign", "1", Some("i386")),
            ],
        );
        let matcher = PackageMatcher::new(&db);

        let versions = |query: &str| {
            matcher
                .match_pkgs_and_versions_from_glob(query)
                .unwrap()
                .into_iter()
                .flatten()
                .map(|version| {
                    (
                        version.entry.version.clone().unwrap(),
                        version.entry.architecture.clone().unwrap(),
                    )
                })
                .collect::<Vec<_>>()
        };

        // A plain query is the native package: `all` plus native, no i386.
        assert_eq!(versions("mixed"), vec![("1".into(), "all".into())]);
        assert_eq!(
            versions("native"),
            vec![("1".into(), "all".into()), ("2".into(), "amd64".into())]
        );
        // `:any` resolves like the plain query.
        assert_eq!(versions("mixed:any"), vec![("1".into(), "all".into())]);
        // `:all` and `:native` are the native package too, not an `all`-only
        // filter; with no native-view build they show nothing.
        assert_eq!(
            versions("native:all"),
            vec![("1".into(), "all".into()), ("2".into(), "amd64".into())]
        );
        assert!(versions("foreign:all").is_empty());
        assert!(versions("foreign:native").is_empty());
        // A foreign qualifier selects that build only, never `all`.
        assert_eq!(versions("mixed:i386"), vec![("2".into(), "i386".into())]);
        assert!(versions("native:i386").is_empty());
        // A plain query falls back to the only package the name has.
        assert_eq!(versions("foreign"), vec![("1".into(), "i386".into())]);
    }

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

    fn matcher() -> PackageMatcher<'static> {
        let stable = source("http://repo", "stable", "main", "amd64");
        let preview = source("http://repo", "preview", "main", "amd64");
        let db = AptDb::from_entries_with_sources(
            "amd64",
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

        PackageMatcher::new(db)
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
            .match_pkgs_and_versions(["fish", "apt=2.5.4", "bash/stable", "missing"])
            .unwrap();
        assert_eq!(no_result, vec!["missing"]);
        let mut n = names(&pkgs);
        n.sort();
        assert_eq!(n, vec!["apt", "bash", "fish"]);
    }

    #[test]
    fn test_match_pkgs_and_versions_arch_dispatch() {
        let m = matcher();
        let (pkgs, no_result) = m.match_pkgs_and_versions(["apt:amd64", "missing"]).unwrap();
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
            .match_pkgs_and_versions(["apt:amd64=2.5.4", "fish:amd64/stable"])
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
