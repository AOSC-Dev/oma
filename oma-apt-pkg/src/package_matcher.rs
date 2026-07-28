//! Package matching — resolve patterns (exact / glob / version / branch) to
//! package entries

use std::borrow::Cow;

use glob_match::glob_match;

use crate::apt_lists::{AptListsError, PackageEntry, PackageIndex};

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

/// A matched package together with its (possibly filtered) entries.
#[derive(Debug)]
pub struct MatchedPackage<'a> {
    /// The package name.
    pub name: &'a str,
    /// The entries for this package (all, or filtered by version/branch).
    pub entries: Vec<Cow<'a, PackageEntry>>,
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
    /// Dispatch rules:
    /// - contains `=` → [`match_from_version`](Self::match_from_version)
    /// - contains `/` → [`match_from_branch`](Self::match_from_branch)
    /// - otherwise → [`match_pkgs_and_versions_from_glob`](Self::match_pkgs_and_versions_from_glob)
    ///
    /// Returns the matched packages and the unmatched keywords.
    pub fn match_pkgs_and_versions(
        &self,
        keywords: impl IntoIterator<Item = &'a str>,
    ) -> MatcherResult<(Vec<MatchedPackage<'a>>, Vec<&'a str>)> {
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

    /// Match packages from a glob pattern (like `apt*`). A pattern without
    /// wildcards falls back to exact name matching.
    pub fn match_pkgs_and_versions_from_glob(
        &self,
        glob: &'a str,
    ) -> MatcherResult<Vec<MatchedPackage<'a>>> {
        let names: Vec<&'a str> = if self.index.has_package(glob) {
            vec![glob]
        } else {
            self.index
                .packages()
                .filter(|p| glob_match(glob, p))
                .collect()
        };

        let mut res = Vec::with_capacity(names.len());
        for name in names {
            res.push(MatchedPackage {
                name,
                entries: self.entries_of(name)?,
            });
        }
        Ok(res)
    }

    /// Match package from a version pattern (like `apt=2.5.4`).
    pub fn match_from_version(&self, pat: &'a str) -> MatcherResult<Vec<MatchedPackage<'a>>> {
        let (pkgname, version_str) = pat
            .split_once('=')
            .ok_or_else(|| MatcherError::InvalidPattern(pat.to_string()))?;

        if !self.index.has_package(pkgname) {
            return Err(MatcherError::NoPackage(pat.to_string()));
        }

        let entries = self
            .entries_of(pkgname)?
            .into_iter()
            .filter(|e| e.version.as_deref() == Some(version_str))
            .collect::<Vec<_>>();

        if entries.is_empty() {
            return Err(MatcherError::NoVersion(
                pkgname.to_string(),
                version_str.to_string(),
            ));
        }

        Ok(vec![MatchedPackage {
            name: pkgname,
            entries,
        }])
    }

    /// Match package from a branch pattern (like `apt/stable`).
    ///
    /// The package `filename` field (`pool/stable/main/f/foo/foo_1.0_amd64.deb`)
    /// has the branch as its second path segment.
    pub fn match_from_branch(&self, pat: &'a str) -> MatcherResult<Vec<MatchedPackage<'a>>> {
        let (pkgname, branch) = pat
            .split_once('/')
            .ok_or_else(|| MatcherError::InvalidPattern(pat.to_string()))?;

        if !self.index.has_package(pkgname) {
            return Err(MatcherError::NoPackage(pat.to_string()));
        }

        let entries = self
            .entries_of(pkgname)?
            .into_iter()
            .filter(|e| {
                e.filename
                    .as_deref()
                    .is_some_and(|f| f.split('/').nth(1) == Some(branch))
            })
            .collect::<Vec<_>>();

        if entries.is_empty() {
            return Ok(Vec::new());
        }

        Ok(vec![MatchedPackage {
            name: pkgname,
            entries,
        }])
    }

    fn entries_of(&self, name: &str) -> MatcherResult<Vec<Cow<'a, PackageEntry>>> {
        Ok(match self.index.get_all(name)? {
            Cow::Borrowed(slice) => slice.iter().map(Cow::Borrowed).collect(),
            Cow::Owned(vec) => vec.into_iter().map(Cow::Owned).collect(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::apt_lists_reader::AptListsReader;
    use std::fs;
    use std::path::Path;

    fn write_packages(dir: &Path, filename: &str, content: &str) {
        fs::write(dir.join(filename), content).unwrap();
    }

    fn matcher() -> (tempfile::TempDir, PackageMatcher<'static>) {
        let dir = tempfile::tempdir().unwrap();
        write_packages(
            dir.path(),
            "repo_stable_main_binary-amd64_Packages",
            r#"Package: fish
Version: 4.5.0
Filename: pool/stable/main/f/fish/fish_4.5.0_amd64.deb

Package: apt
Version: 2.5.4
Filename: pool/stable/main/a/apt/apt_2.5.4_amd64.deb

Package: bash
Version: 5.2.3
Filename: pool/stable/main/b/bash/bash_5.2.3_amd64.deb

"#,
        );
        write_packages(
            dir.path(),
            "repo_preview_main_binary-amd64_Packages",
            r#"Package: fish
Version: 4.8.1
Filename: pool/preview/main/f/fish/fish_4.8.1_amd64.deb

Package: firefox
Version: 130.0
Filename: pool/preview/main/f/firefox/firefox_130.0_amd64.deb

"#,
        );

        let reader = AptListsReader::build(dir.path()).unwrap();
        // Leak the reader to get a 'static reference for testing.
        let reader: &'static AptListsReader = Box::leak(Box::new(reader));
        let matcher = PackageMatcher::new(reader);
        (dir, matcher)
    }

    fn names<'a>(pkgs: &'a [MatchedPackage<'a>]) -> Vec<&'a str> {
        pkgs.iter().map(|p| p.name).collect()
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
        assert_eq!(res[0].entries[0].version.as_deref(), Some("2.5.4"));
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
        assert_eq!(res[0].entries[0].version.as_deref(), Some("4.5.0"));

        let res = m.match_from_branch("fish/preview").unwrap();
        assert_eq!(res[0].entries[0].version.as_deref(), Some("4.8.1"));
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
}
