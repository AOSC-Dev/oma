//! Package search API.
//!
//! Provides legacy search backends (`StrSimSearch`, `TextSearch`)
//! that build on the C++ `oma_apt::Cache`, and defines common search
//! types used throughout the codebase.

use ahash::AHashMap;
use glob_match::glob_match;
use memchr::memmem;
use oma_apt::{
    Package,
    cache::{Cache, PackageSort},
    raw::IntoRawIter,
};
use serde::{Deserialize, Serialize};

use crate::{
    matches::has_dbg,
    pkginfo::OmaPackage,
};

// ---------------------------------------------------------------------------
// Shared search types
// ---------------------------------------------------------------------------

/// Status of the package.
#[derive(PartialEq, Eq, Debug, Clone, Copy, Serialize, Deserialize)]
pub enum PackageStatus {
    Avail,
    Installed,
    Upgrade,
}

impl PartialOrd for PackageStatus {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for PackageStatus {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        match self {
            PackageStatus::Avail => match other {
                PackageStatus::Avail => std::cmp::Ordering::Equal,
                PackageStatus::Installed => std::cmp::Ordering::Greater,
                PackageStatus::Upgrade => std::cmp::Ordering::Less,
            },
            PackageStatus::Installed => match other {
                PackageStatus::Avail => std::cmp::Ordering::Less,
                PackageStatus::Installed => std::cmp::Ordering::Equal,
                PackageStatus::Upgrade => std::cmp::Ordering::Less,
            },
            PackageStatus::Upgrade => match other {
                PackageStatus::Avail => std::cmp::Ordering::Greater,
                PackageStatus::Installed => std::cmp::Ordering::Greater,
                PackageStatus::Upgrade => std::cmp::Ordering::Equal,
            },
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
/// Result of a search process.
pub struct SearchResult {
    /// String contains the name of a package to search for.
    pub name: String,
    /// String contains the description of a package.
    pub desc: String,
    /// Optional string contains the old_version(s) of a package
    pub old_version: Option<String>,
    /// String contains the new_version(s) of a package
    pub new_version: String,
    /// Boolean indicating whether this result is a full match or not.
    pub full_match: bool,
    /// Boolean indicating whether this result has a matching package for debug symbols.
    pub dbg_package: bool,
    /// `PackageStatus` instance which reports the status of the package.
    pub status: PackageStatus,
    /// Boolean indicating whether the package is an AOSC OS metapackage (-base package).
    pub is_base: bool,
}

#[derive(Debug, thiserror::Error)]
pub enum OmaSearchError {
    #[error("No result found: {0}")]
    NoResult(String),
    #[error("Failed to get candidate version: {0}")]
    FailedGetCandidate(String),
    #[error("Null pointer in apt cache")]
    PtrIsNone,
}

pub type OmaSearchResult<T> = Result<T, OmaSearchError>;

/// Common trait for all search backends.
pub trait OmaSearch {
    fn search(&self, query: &str) -> OmaSearchResult<Vec<SearchResult>>;
}

// ---------------------------------------------------------------------------
// Legacy backends — kept here because they rely on `oma_apt::Cache`.
// ---------------------------------------------------------------------------

/// strsim: Sort search results based on string matching similarity (score).
pub struct StrSimSearch<'a> {
    /// Locally cached index.
    cache: &'a Cache,
}

impl OmaSearch for StrSimSearch<'_> {
    fn search(&self, query: &str) -> OmaSearchResult<Vec<SearchResult>> {
        let sort = PackageSort::default().include_virtual();
        let pkgs = self.cache.packages(&sort);

        let mut res = AHashMap::new();

        for pkg in pkgs {
            let name = pkg.fullname(true);
            if let Some(cand) = pkg.candidate() {
                if memmem::find(name.as_bytes(), query.as_bytes()).is_some()
                    && !name.ends_with("-dbg")
                    && !res.contains_key(&name)
                {
                    let oma_pkg = OmaPackage::new(&cand, &pkg)
                        .map_err(|_| OmaSearchError::PtrIsNone)?;
                    res.insert(
                        name.clone(),
                        (oma_pkg, cand.is_installed(), pkg.is_upgradable(), false),
                    );
                }

                if cand
                    .description()
                    .is_some_and(|x| memmem::find(x.as_bytes(), query.as_bytes()).is_some())
                    && !res.contains_key(&name)
                    && !name.ends_with("-dbg")
                {
                    let oma_pkg = OmaPackage::new(&cand, &pkg)
                        .map_err(|_| OmaSearchError::PtrIsNone)?;
                    res.insert(
                        name.clone(),
                        (oma_pkg, cand.is_installed(), pkg.is_upgradable(), false),
                    );
                }
            } else if name == query && pkg.has_provides() {
                let real_pkgs = pkg.provides().flat_map(|x| {
                    unsafe { x.target_pkg() }
                        .make_safe()
                        .ok_or(OmaSearchError::PtrIsNone)
                });
                for pkg in real_pkgs {
                    let pkg = Package::new(self.cache, pkg);
                    if let Some(cand) = pkg.candidate() {
                        let oma_pkg = OmaPackage::new(&cand, &pkg)
                            .map_err(|_| OmaSearchError::PtrIsNone)?;

                        res.insert(
                            name.clone(),
                            (oma_pkg, cand.is_installed(), pkg.is_upgradable(), true),
                        );
                    }
                }
            }
        }

        let mut res = res.into_values().collect::<Vec<_>>();

        res.sort_unstable_by(|x, y| {
            let x_score = Self::pkg_score(query, &x.0, x.3);
            let y_score = Self::pkg_score(query, &y.0, y.3);

            let c = y_score.cmp(&x_score);

            if c == std::cmp::Ordering::Equal {
                y.0.raw_pkg.fullname(true).cmp(&x.0.raw_pkg.fullname(true))
            } else {
                c
            }
        });

        let mut v = vec![];

        for (pkginfo, install, upgrade, _) in res {
            let pkg = Package::new(self.cache, pkginfo.raw_pkg);
            let cand = pkg
                .candidate()
                .ok_or_else(|| OmaSearchError::FailedGetCandidate(pkg.fullname(true)))?;

            let name = pkg.fullname(true);
            let is_base = name.ends_with("-base");
            let full_match = query == name;

            v.push(SearchResult {
                name,
                desc: cand
                    .summary()
                    .unwrap_or_else(|| "No description".to_string()),
                old_version: {
                    if !upgrade {
                        None
                    } else {
                        pkg.installed().map(|x| x.version().to_string())
                    }
                },
                new_version: cand.version().to_string(),
                full_match,
                dbg_package: has_dbg(self.cache, &pkg, &cand),
                status: if upgrade {
                    PackageStatus::Upgrade
                } else if install {
                    PackageStatus::Installed
                } else {
                    PackageStatus::Avail
                },
                is_base,
            });
        }

        v.sort_by_key(|b| std::cmp::Reverse(b.status));

        for i in 0..v.len() {
            if v[i].full_match {
                let i = v.remove(i);
                v.insert(0, i);
            }
        }

        Ok(v)
    }
}

impl<'a> StrSimSearch<'a> {
    pub fn new(cache: &'a Cache) -> Self {
        Self { cache }
    }
    /// return the string similarity score.
    fn pkg_score(input: &str, pkginfo: &OmaPackage, is_provide: bool) -> u16 {
        if is_provide {
            return 1000;
        }

        (strsim::jaro_winkler(&pkginfo.raw_pkg.fullname(true), input) * 1000.0) as u16
    }
}

/// Text match search based on `memmem`
pub struct TextSearch<'a> {
    cache: &'a Cache,
}

impl<'a> TextSearch<'a> {
    pub fn new(cache: &'a Cache) -> Self {
        Self { cache }
    }
}

impl OmaSearch for TextSearch<'_> {
    fn search(&self, query: &str) -> OmaSearchResult<Vec<SearchResult>> {
        let mut res = vec![];

        let sort = PackageSort::default();
        let pkgs = self.cache.packages(&sort);

        for pkg in pkgs {
            let name = pkg.fullname(true);
            let cand = pkg.candidate();

            if (memmem::find(name.as_bytes(), query.as_bytes()).is_some()
                || glob_match(query, &name))
                && !name.ends_with("-dbg")
            {
                let full_match = query == name;
                let is_base = name.ends_with("-base");
                let upgrade = pkg.is_upgradable();
                let installed = pkg.is_installed();
                if let Some(cand) = cand {
                    res.push(SearchResult {
                        name,
                        desc: cand
                            .summary()
                            .unwrap_or_else(|| "No description".to_string()),
                        old_version: {
                            if !pkg.is_upgradable() {
                                None
                            } else {
                                pkg.installed().map(|x| x.version().to_string())
                            }
                        },
                        new_version: cand.version().to_string(),
                        full_match,
                        dbg_package: has_dbg(self.cache, &pkg, &cand),
                        status: if upgrade {
                            PackageStatus::Upgrade
                        } else if installed {
                            PackageStatus::Installed
                        } else {
                            PackageStatus::Avail
                        },
                        is_base,
                    })
                }
            }
        }

        res.sort_by_key(|b| std::cmp::Reverse(b.status));

        for i in 0..res.len() {
            if res[i].full_match {
                let i = res.remove(i);
                res.insert(0, i);
            }
        }

        Ok(res)
    }
}
