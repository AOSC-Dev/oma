//! Full-text package search with `indicium`.
//!
//! Builds a search index from parsed APT list entries and dpkg status,
//! without depending on the C++ `oma-apt` binding.

use std::collections::{HashMap, HashSet};
use std::str::FromStr;

use ahash::RandomState;
pub use indicium::simple::SearchType;
use indicium::simple::{Indexable, SearchIndex, SearchIndexBuilder};
use serde::{Deserialize, Serialize};

use crate::apt_lists::PackageEntry;

type IndexSet<T> = indexmap::IndexSet<T, RandomState>;
type IndexMap<K, V> = indexmap::IndexMap<K, V, RandomState>;

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

/// A single entry in the search index.
#[derive(Clone)]
pub struct SearchEntry {
    /// The name of the package
    pub name: String,
    /// The description of the package
    pub description: String,
    /// The status of the package. See [`PackageStatus`]
    pub status: PackageStatus,
    /// Virtual packages this package provides.
    pub provides: IndexSet<String>,
    /// Whether the package provides a matching package for debug symbols.
    pub has_dbg: bool,
    /// Whether the package is an AOSC OS metapackage (-base package).
    pub section_is_base: bool,
    /// Old (installed) version, if any.
    pub old_version: Option<String>,
    /// New (candidate) version.
    pub new_version: String,
}

impl std::fmt::Debug for SearchEntry {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SearchEntry")
            .field("pkgname", &self.name)
            .field("description", &self.description)
            .field("status", &self.status)
            .field("provides", &self.provides)
            .field("has_dbg", &self.has_dbg)
            .field("old_version", &self.old_version)
            .field("new_version", &self.new_version)
            .field("section_is_base", &self.section_is_base)
            .finish()
    }
}

impl Indexable for SearchEntry {
    fn strings(&self) -> Vec<String> {
        let mut v = vec![self.name.clone(), self.description.clone()];
        v.extend(self.provides.clone());
        v
    }
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

/// Index search based on `indicium`.
pub struct IndiciumSearch {
    /// Map contains package names and their corresponding search entries.
    pub pkg_map: IndexMap<String, SearchEntry>,
    /// Index used to perform search operations.
    pub index: SearchIndex<String>,
}

pub trait OmaSearch {
    fn search(&self, query: &str) -> OmaSearchResult<Vec<SearchResult>>;
}

impl OmaSearch for IndiciumSearch {
    fn search(&self, query: &str) -> OmaSearchResult<Vec<SearchResult>> {
        let mut search_res = vec![];
        let query = query.to_lowercase();
        let res = self.index.search(&query);

        if res.is_empty() {
            return Err(OmaSearchError::NoResult(query));
        }

        for i in res {
            let entry = self
                .pkg_map
                .get(i)
                .ok_or_else(|| OmaSearchError::NoResult(i.to_string()))?;

            let full_match = query == entry.name || entry.provides.iter().any(|x| *x == query);

            search_res.push(SearchResult {
                name: entry.name.clone(),
                desc: entry.description.clone(),
                old_version: entry.old_version.clone(),
                new_version: entry.new_version.clone(),
                full_match,
                dbg_package: entry.has_dbg,
                status: entry.status,
                is_base: entry.section_is_base,
            });
        }

        search_res.sort_by_key(|b| std::cmp::Reverse(b.status));

        for i in 0..search_res.len() {
            if search_res[i].full_match {
                let i = search_res.remove(i);
                search_res.insert(0, i);
            }
        }

        Ok(search_res)
    }
}

/// Parse a `Provides` field value into individual virtual package names.
/// Format: `"foo, bar (= 1.0), baz"` → `["foo", "bar", "baz"]`
fn parse_provides(value: &str) -> Vec<String> {
    value
        .split(',')
        .map(|s| s.trim())
        .filter_map(|s| s.split_whitespace().next())
        .map(|s| s.to_string())
        .collect()
}

/// Build a set of all available package names from the entries list.
fn build_available_names(entries: &[PackageEntry]) -> HashSet<String> {
    entries.iter().map(|e| e.package.clone()).collect()
}

impl IndiciumSearch {
    /// Build a new search index from parsed APT list entries and dpkg status.
    ///
    /// * `entries` — All package entries from all `*_Packages` files.
    /// * `installed` — Set of currently installed package names (from dpkg status).
    /// * `installed_versions` — Map from installed package name to its version.
    /// * `search_type` — The indicium search type to use.
    /// * `progress` — A callback invoked with the current index position during building.
    pub fn new(
        entries: &[PackageEntry],
        installed: &HashSet<String>,
        installed_versions: &HashMap<String, String>,
        search_type: SearchType,
        progress: impl Fn(usize),
    ) -> Self {
        let available_names = build_available_names(entries);
        let mut pkg_map: IndexMap<String, SearchEntry> = IndexMap::with_hasher(RandomState::new());

        for (i, entry) in entries.iter().enumerate() {
            progress(i);
            let name = &entry.package;

            if name.ends_with("-dbg") {
                continue;
            }

            // Duplicates across different repos/components: keep the one with
            // the highest version (handles `~`, epochs, etc.).
            let should_replace = pkg_map.get(name).is_some_and(|existing: &SearchEntry| {
                match (&entry.version, Some(&existing.new_version)) {
                    (Some(new_ver), Some(old_ver)) => {
                        let nv = debversion::Version::from_str(new_ver);
                        let ov = debversion::Version::from_str(old_ver);
                        match (nv, ov) {
                            (Ok(n), Ok(o)) => n > o,
                            _ => new_ver != old_ver,
                        }
                    }
                    _ => false,
                }
            });

            if pkg_map.contains_key(name) && !should_replace {
                continue;
            }

            let status = if installed.contains(name.as_str()) {
                // Check if a newer version is available in the repo
                if is_upgradable(
                    entry.version.as_ref(),
                    installed_versions.get(name.as_str()),
                ) {
                    PackageStatus::Upgrade
                } else {
                    PackageStatus::Installed
                }
            } else {
                PackageStatus::Avail
            };

            let (old_version, new_version) =
                extract_versions(status, installed_versions, name, &entry.version);

            let description = entry
                .description
                .as_deref()
                .map(|d| d.lines().next().unwrap_or(d).to_string())
                .unwrap_or_else(|| "No description".to_string());

            let provides = entry
                .provides
                .as_deref()
                .map(parse_provides)
                .unwrap_or_default();

            let has_dbg = available_names.contains(&format!("{name}-dbg"));

            let section_is_base = entry.section.as_deref().is_some_and(|s| s == "Bases");

            pkg_map.insert(
                name.clone(),
                SearchEntry {
                    name: name.clone(),
                    description,
                    status,
                    provides: provides.into_iter().collect(),
                    has_dbg,
                    section_is_base,
                    old_version,
                    new_version,
                },
            );
        }

        let mut search_index: SearchIndex<String> = SearchIndexBuilder::default()
            .search_type(search_type)
            .exclude_keywords(None)
            .build();
        pkg_map.iter().for_each(|(key, value)| {
            search_index.insert(key, value);
        });

        Self {
            pkg_map,
            index: search_index,
        }
    }

    /// Refresh the status and version information of all entries from fresh dpkg
    /// status data, without rebuilding the full-text search index.
    /// This is much cheaper than creating a new `IndiciumSearch` from scratch.
    ///
    /// * `installed` — Re-parsed set of installed package names.
    /// * `installed_versions` — Re-parsed map from installed package name to its version.
    pub fn refresh_status(
        &mut self,
        installed: &HashSet<String>,
        installed_versions: &HashMap<String, String>,
    ) {
        for entry in self.pkg_map.values_mut() {
            let (status, old_ver, new_ver) = if installed.contains(entry.name.as_str()) {
                let inst_ver = installed_versions.get(&entry.name).cloned();
                if is_upgradable(
                    Some(&entry.new_version),
                    installed_versions.get(&entry.name),
                ) {
                    (PackageStatus::Upgrade, inst_ver, entry.new_version.clone())
                } else {
                    (
                        PackageStatus::Installed,
                        inst_ver,
                        entry.new_version.clone(),
                    )
                }
            } else {
                (PackageStatus::Avail, None, entry.new_version.clone())
            };

            entry.status = status;
            entry.old_version = old_ver;
            entry.new_version = new_ver;
        }
    }
}

/// Determine if a package is upgradable: a newer version is available in the
/// repo than what is currently installed.
/// Uses proper Debian version comparison (handles `~`, epochs, etc.).
fn is_upgradable(candidate_version: Option<&String>, installed_version: Option<&String>) -> bool {
    match (candidate_version, installed_version) {
        (Some(cand), Some(inst)) => {
            let cand_ver = debversion::Version::from_str(cand);
            let inst_ver = debversion::Version::from_str(inst);
            match (cand_ver, inst_ver) {
                (Ok(cv), Ok(iv)) => cv > iv,
                // Fall back to string comparison if parsing fails
                _ => cand != inst,
            }
        }
        (Some(_), None) => false, // not installed
        (None, _) => false,       // no candidate available
    }
}

fn extract_versions(
    status: PackageStatus,
    installed_versions: &HashMap<String, String>,
    name: &str,
    candidate_version: &Option<String>,
) -> (Option<String>, String) {
    let new = candidate_version
        .clone()
        .unwrap_or_else(|| "Unknown".to_string());

    // Keep the installed version for all installed packages, not just upgrades.
    let old = if status == PackageStatus::Installed || status == PackageStatus::Upgrade {
        installed_versions.get(name).cloned()
    } else {
        None
    };

    (old, new)
}
