use indicium::simple::{Indexable, SearchIndex};
use oma_apt::{
    cache::{Cache, PackageSort},
    package::Package,
    raw::package::RawPackage,
};
use std::collections::HashMap;

use crate::query::has_dbg;

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

struct SearchEntry {
    pkgname: String,
    description: String,
    status: PackageStatus,
    provide: Option<String>,
    has_dbg: bool,
    raw_pkg: RawPackage,
    section_is_base: bool,
}

impl Indexable for SearchEntry {
    fn strings(&self) -> Vec<String> {
        vec![self.pkgname.clone(), self.description.clone()]
    }
}

#[derive(Debug, thiserror::Error)]
pub enum OmaSearchError {
    #[error(transparent)]
    RustApt(#[from] oma_apt::util::Exception),
    #[error("No result found: {0}")]
    NoResult(String),
    #[error("Failed to get candidate version: {0}")]
    FailedGetCandidate(String),
}

pub type OmaSearchResult<T> = Result<T, OmaSearchError>;

#[derive(Debug)]
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

pub fn search_pkgs(cache: &Cache, input: &str) -> OmaSearchResult<Vec<SearchResult>> {
    let mut search_res = vec![];
    let input = input.to_lowercase();
    let sort = PackageSort::default().include_virtual();
    let packages = cache.packages(&sort)?;

    let mut pkg_map = HashMap::new();

    for pkg in packages {
        if pkg.name().contains("-dbg") {
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
            pkg_map.insert(
                pkg.name().to_string(),
                SearchEntry {
                    pkgname: pkg.name().to_string(),
                    description: cand.description().unwrap_or("".to_string()),
                    status,
                    provide: None,
                    has_dbg: has_dbg(cache, &pkg, &cand),
                    raw_pkg: pkg.unique(),
                    section_is_base: cand.section().map(|x| x == "Bases").unwrap_or(false),
                },
            );
            continue;
        }

        let real_pkgs = pkg
            .provides()
            .map(|x| (x.name().to_string(), x.target_pkg()));

        for (provide, i) in real_pkgs {
            let pkg = Package::new(cache, i.unique());

            let status = if pkg.is_upgradable() {
                PackageStatus::Upgrade
            } else if pkg.is_installed() {
                PackageStatus::Installed
            } else {
                PackageStatus::Avail
            };

            if let Some(cand) = pkg.candidate() {
                pkg_map.insert(
                    i.name().to_string(),
                    SearchEntry {
                        pkgname: pkg.name().to_string(),
                        description: cand.description().unwrap_or("".to_string()),
                        status,
                        provide: Some(provide.to_string()),
                        has_dbg: has_dbg(cache, &pkg, &cand),
                        raw_pkg: pkg.unique(),
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

    let res = search_index.search(&input);

    if res.is_empty() {
        let input = input.to_string();
        return Err(OmaSearchError::NoResult(input));
    }

    for i in res {
        // res 的源确保是存在的，所以直接 unwrap
        let entry = pkg_map.get(i).unwrap();

        let name = entry.pkgname.clone();
        let desc = entry.description.clone();

        let status = entry.status.clone();
        let has_dbg = entry.has_dbg;
        let pkg = entry.raw_pkg.unique();
        let pkg = Package::new(cache, pkg);
        let full_match = name == input || entry.provide == Some(input.to_string());

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

        search_res.push(SearchResult {
            name,
            desc,
            old_version,
            new_version,
            full_match,
            dbg_package: has_dbg,
            status,
            is_base,
        });
    }

    search_res.sort_by(|a, b| b.status.cmp(&a.status));

    let index = search_res.iter().position(|x| x.full_match);

    if let Some(index) = index {
        let i = search_res.remove(index);
        search_res.insert(0, i);
    }

    Ok(search_res)
}
