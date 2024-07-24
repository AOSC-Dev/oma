use cxx::UniquePtr;
use indicium::simple::{Indexable, SearchIndex};
use oma_apt::{
    cache::{Cache, PackageSort},
    error::{AptError, AptErrors},
    raw::{IntoRawIter, PkgIterator},
    Package,
};
use std::{collections::HashMap, fmt::Debug};

use crate::{format_description, pkginfo::PtrIsNone, query::has_dbg};

#[derive(PartialEq, Eq, Debug, Clone)]
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

pub struct SearchEntry {
    pkgname: String,
    description: String,
    status: PackageStatus,
    provide: Option<String>,
    has_dbg: bool,
    raw_pkg: UniquePtr<PkgIterator>,
    section_is_base: bool,
}

impl Debug for SearchEntry {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SearchEntry")
            .field("pkgname", &self.pkgname)
            .field("description", &self.description)
            .field("status", &self.status)
            .field("provide", &self.provide)
            .field("has_dbg", &self.has_dbg)
            .field("raw_pkg", &self.raw_pkg.name())
            .field("section_is_base", &self.section_is_base)
            .finish()
    }
}

impl Indexable for SearchEntry {
    fn strings(&self) -> Vec<String> {
        vec![self.pkgname.clone(), self.description.clone()]
    }
}

#[derive(Debug, thiserror::Error)]
pub enum OmaSearchError {
    #[error(transparent)]
    AptErrors(#[from] AptErrors),
    #[error(transparent)]
    AptError(#[from] AptError),
    #[error(transparent)]
    AptCxxException(#[from] cxx::Exception),
    #[error("No result found: {0}")]
    NoResult(String),
    #[error("Failed to get candidate version: {0}")]
    FailedGetCandidate(String),
    #[error(transparent)]
    PtrIsNone(#[from] PtrIsNone),
}

pub type OmaSearchResult<T> = Result<T, OmaSearchError>;

#[derive(Debug, Clone)]
pub struct SearchResult {
    pub name: String,
    pub desc: String,
    pub old_version: Option<String>,
    pub new_version: String,
    pub full_match: bool,
    pub dbg_package: bool,
    pub status: PackageStatus,
    pub is_base: bool,
}

pub struct OmaSearch<'a> {
    cache: &'a Cache,
    pub pkg_map: HashMap<String, SearchEntry>,
    index: SearchIndex<String>,
}

impl<'a> OmaSearch<'a> {
    pub fn new(cache: &'a Cache) -> OmaSearchResult<Self> {
        let sort = PackageSort::default().include_virtual();
        let packages = cache.packages(&sort);

        let mut pkg_map = HashMap::new();

        for pkg in packages {
            if pkg.fullname(true).contains("-dbg") {
                continue;
            }

            let status = if pkg.is_upgradable() {
                PackageStatus::Upgrade
            } else if pkg.is_installed() {
                PackageStatus::Installed
            } else {
                PackageStatus::Avail
            };

            if let Some(cand) = pkg.candidate() {
                if !pkg_map.contains_key(&pkg.fullname(true)) {
                    pkg_map.insert(
                        pkg.fullname(true),
                        SearchEntry {
                            pkgname: pkg.fullname(true),
                            description: format_description(
                                &cand.description().unwrap_or("".to_string()),
                            )
                            .0
                            .to_string(),
                            status,
                            provide: pkg.provides().next().map(|x| x.name().to_string()),
                            has_dbg: has_dbg(cache, &pkg, &cand),
                            raw_pkg: unsafe { pkg.unique() }
                                .make_safe()
                                .ok_or(OmaSearchError::PtrIsNone(PtrIsNone))?,
                            section_is_base: cand.section().map(|x| x == "Bases").unwrap_or(false),
                        },
                    );
                    continue;
                }
            }

            let mut real_pkgs = vec![];
            for i in pkg.provides() {
                real_pkgs.push((
                    i.name().to_string(),
                    unsafe { i.target_pkg() }
                        .make_safe()
                        .ok_or(OmaSearchError::PtrIsNone(PtrIsNone))?,
                ));
            }

            for (provide, i) in real_pkgs {
                let pkg = Package::new(cache, i);

                let status = if pkg.is_upgradable() {
                    PackageStatus::Upgrade
                } else if pkg.is_installed() {
                    PackageStatus::Installed
                } else {
                    PackageStatus::Avail
                };

                if let Some(cand) = pkg.candidate() {
                    pkg_map.insert(
                        pkg.fullname(true),
                        SearchEntry {
                            pkgname: pkg.fullname(true),
                            description: format_description(
                                &cand.description().unwrap_or("".to_string()),
                            )
                            .0
                            .to_string(),
                            status,
                            provide: Some(provide.to_string()),
                            has_dbg: has_dbg(cache, &pkg, &cand),
                            raw_pkg: unsafe { pkg.unique() }
                                .make_safe()
                                .ok_or(OmaSearchError::PtrIsNone(PtrIsNone))?,
                            section_is_base: cand.section().map(|x| x == "Bases").unwrap_or(false),
                        },
                    );
                }
            }
        }

        let mut search_index: SearchIndex<String> = SearchIndex::default();

        pkg_map
            .iter()
            .for_each(|(key, value)| search_index.insert(key, value));

        Ok(Self {
            cache,
            pkg_map,
            index: search_index,
        })
    }

    pub fn search(&self, query: &str) -> OmaSearchResult<Vec<SearchResult>> {
        let mut search_res = vec![];
        let query = query.to_lowercase();
        let res = self.index.search(&query);

        if res.is_empty() {
            return Err(OmaSearchError::NoResult(query));
        }

        for i in res {
            let entry = self.search_result(i, Some(&query))?;
            search_res.push(entry);
        }

        search_res.sort_by(|a, b| b.status.cmp(&a.status));

        for i in 0..search_res.len() {
            if search_res[i].full_match {
                let i = search_res.remove(i);
                search_res.insert(0, i);
            }
        }

        Ok(search_res)
    }

    pub fn search_result(
        &self,
        i: &str,
        query: Option<&str>,
    ) -> Result<SearchResult, OmaSearchError> {
        let entry = self.pkg_map.get(i).unwrap();
        let name = entry.pkgname.clone();
        let desc = entry.description.clone();
        let status = entry.status.clone();
        let has_dbg = entry.has_dbg;
        let pkg = unsafe { entry.raw_pkg.unique() }
            .make_safe()
            .ok_or(OmaSearchError::PtrIsNone(PtrIsNone))?;
        let pkg = Package::new(self.cache, pkg);

        let full_match = if let Some(query) = query {
            query == name || entry.provide.as_ref().map(|x| x == query).unwrap_or(false)
        } else {
            false
        };

        let old_version = if status != PackageStatus::Upgrade {
            None
        } else {
            pkg.installed().map(|x| x.version().to_string())
        };

        let new_version = pkg
            .candidate()
            .map(|x| x.version().to_string())
            .ok_or_else(|| OmaSearchError::FailedGetCandidate(pkg.name().to_string()))?;

        let is_base = entry.section_is_base;

        Ok(SearchResult {
            name,
            desc,
            old_version,
            new_version,
            full_match,
            dbg_package: has_dbg,
            status,
            is_base,
        })
    }
}
