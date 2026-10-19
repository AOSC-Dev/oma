use super::arch::{Arch, PackageKey};
use super::model::Version;

/// One resolved query: the selected package plus its filtered versions.
#[derive(Debug)]
pub struct ResolvedPackage {
    /// The selected package identity.
    pub key: PackageKey,
    /// The versions this query selects, after version/branch filtering.
    pub versions: Vec<Version>,
    /// The architecture selector the query carried, if any.
    pub arch: Option<Arch>,
}

/// Result of resolving package queries.
#[derive(Debug)]
pub struct QueryResolution {
    /// Resolved queries, in query order.
    pub resolved: Vec<ResolvedPackage>,
    /// Queries that matched no package.
    pub no_match: Vec<String>,
}

/// A parsed package query: `name[:arch][=version|/branch]`.
#[derive(Debug, Clone, Copy)]
pub(crate) struct Query<'a> {
    pub(crate) name: &'a str,
    pub(crate) arch: Option<&'a str>,
    pub(crate) version: Option<&'a str>,
    pub(crate) branch: Option<&'a str>,
}

impl<'a> Query<'a> {
    pub(crate) fn parse(query: &'a str) -> Self {
        let (name, version) = match query.split_once('=') {
            Some((name, version)) => (name, Some(version)),
            None => (query, None),
        };
        let (name, branch) = match name.split_once('/') {
            Some((name, branch)) => (name, Some(branch)),
            None => (name, None),
        };
        let (name, arch) = match name.rsplit_once(':') {
            Some((name, arch)) => (name, Some(arch)),
            None => (name, None),
        };
        Self {
            name,
            arch,
            version,
            branch,
        }
    }
}

pub(crate) fn matches_version(
    version: &Version,
    exact: Option<&str>,
    branch: Option<&str>,
) -> bool {
    if let Some(exact) = exact {
        return version.version() == Some(exact);
    }
    if let Some(branch) = branch {
        return version.sources.iter().any(|source| source.suite == branch);
    }
    true
}
