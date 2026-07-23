//! Full-text package search with `indicium`.
//!
//! Builds a search index from parsed APT list entries and dpkg status,
//! without depending on the C++ `oma-apt` binding.

use std::collections::HashMap;
use std::str::FromStr;

use ahash::RandomState;
use glob_match::glob_match;
pub use indicium::simple::SearchType;
use indicium::simple::{Indexable, SearchIndex, SearchIndexBuilder};
use memchr::memmem;
use serde::{Deserialize, Serialize};
use spdlog::debug;

use crate::{AptDb, DpkgState};

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
#[derive(Clone, Serialize, Deserialize)]
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

impl IndiciumSearch {
    /// Build a new search index from an `AptDb` (package entries) and `DpkgState`.
    ///
    /// * `apt_db` — Parsed and cached apt package data.
    /// * `dpkg` — Fresh dpkg status.
    /// * `search_type` — The indicium search type to use.
    /// * `progress` — A callback invoked with the current index position during building.
    pub fn new(
        apt_db: &AptDb,
        dpkg: &DpkgState,
        search_type: SearchType,
        progress: impl Fn(usize),
    ) -> Self {
        let mut pkg_map: IndexMap<String, SearchEntry> = IndexMap::with_hasher(RandomState::new());

        for (i, entry) in apt_db.entries.iter().enumerate() {
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

            let status = if dpkg.is_installed(name) {
                if is_upgradable(
                    entry.version.as_ref(),
                    dpkg.installed_versions.get(name.as_str()),
                ) {
                    PackageStatus::Upgrade
                } else {
                    PackageStatus::Installed
                }
            } else {
                PackageStatus::Avail
            };

            let (old_version, new_version) =
                extract_versions(status, &dpkg.installed_versions, name, &entry.version);

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

            let has_dbg = apt_db.has_package(&format!("{name}-dbg"));

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

    /// Refresh the status and version information of all entries from fresh
    /// dpkg status data, without rebuilding the full-text search index.
    pub fn refresh_status(&mut self, dpkg: &DpkgState) {
        for entry in self.pkg_map.values_mut() {
            let (status, old_ver, new_ver) = if dpkg.installed.contains(entry.name.as_str()) {
                let inst_ver = dpkg.installed_versions.get(&entry.name).cloned();
                if is_upgradable(
                    Some(&entry.new_version),
                    dpkg.installed_versions.get(&entry.name),
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

impl IndiciumSearch {
    /// Build a search index from explicit paths, handling cache automatically.
    ///
    /// Cache hierarchy (fastest first):
    /// 1. `search_cache_path` — serialized `pkg_map`; avoids rebuilding `SearchIndex`.
    /// 2. `apt_cache_path` — serialized `Vec<PackageEntry>`; avoids re-parsing deb822.
    /// 3. Cold path — parse from scratch.
    ///
    /// After loading (any tier) the search index is refreshed with current dpkg
    /// status via `refresh_status`.
    pub fn from_paths(
        lists_dir: impl AsRef<std::path::Path>,
        dpkg_status_path: impl AsRef<std::path::Path>,
        apt_cache_path: impl AsRef<std::path::Path>,
        search_cache_path: impl AsRef<std::path::Path>,
        search_type: SearchType,
        progress: impl Fn(usize),
    ) -> Result<Self, crate::error::Error> {
        // Tier 1: try search cache (fastest)
        if Self::search_cache_valid(&search_cache_path, &lists_dir) {
            let dpkg = DpkgState::from_file(&dpkg_status_path)?;
            if let Some(mut searcher) =
                Self::load_search_cache(&search_cache_path, search_type.clone())
            {
                debug!("Search cache hit");
                searcher.refresh_status(&dpkg);
                return Ok(searcher);
            }
        }

        debug!("Search cache miss, trying AptDb cache");
        // Tier 2: try apt DB cache
        let dpkg = DpkgState::from_file(&dpkg_status_path)?;
        let apt_db = AptDb::load_or_build(&apt_cache_path, &lists_dir)?;

        let mut searcher = Self::new(&apt_db, &dpkg, search_type, progress);
        searcher.refresh_status(&dpkg);

        // Persist search cache for next time
        if let Err(e) = searcher.save_search_cache(&search_cache_path) {
            debug!("Failed to save search cache: {e}");
        } else {
            debug!("Search cache saved");
        }

        Ok(searcher)
    }

    /// Check whether the search cache is still valid by comparing mtimes with
    /// the `*_Packages` source files.
    fn search_cache_valid(
        cache_path: impl AsRef<std::path::Path>,
        lists_dir: impl AsRef<std::path::Path>,
    ) -> bool {
        use std::fs;

        let cache_mtime = match fs::metadata(&cache_path).and_then(|m| m.modified()) {
            Ok(t) => t,
            Err(_) => return false,
        };

        let dir = match fs::read_dir(lists_dir.as_ref()) {
            Ok(d) => d,
            Err(_) => return false,
        };

        for entry in dir {
            let entry = match entry {
                Ok(e) => e,
                Err(_) => continue,
            };
            let name = entry.file_name();
            let name = name.to_string_lossy();
            if !name.ends_with("_Packages") {
                continue;
            }
            let src_mtime = match entry.metadata().and_then(|m| m.modified()) {
                Ok(t) => t,
                Err(_) => continue,
            };
            if src_mtime > cache_mtime {
                return false;
            }
        }
        true
    }

    /// Try to load a previously saved search index from its binary cache.
    fn load_search_cache(
        path: impl AsRef<std::path::Path>,
        search_type: SearchType,
    ) -> Option<Self> {
        use std::fs;
        use std::io::Read;

        let mut file = fs::File::open(path.as_ref()).ok()?;
        let mut buf = Vec::new();
        file.read_to_end(&mut buf).ok()?;

        let pkg_map: IndexMap<String, SearchEntry> =
            bincode::serde::decode_from_slice(&buf, bincode::config::standard())
                .ok()?
                .0;

        let mut search_index: SearchIndex<String> = SearchIndexBuilder::default()
            .search_type(search_type)
            .exclude_keywords(None)
            .build();
        pkg_map.iter().for_each(|(key, value)| {
            search_index.insert(key, value);
        });

        Some(Self {
            pkg_map,
            index: search_index,
        })
    }

    /// Save the search index (pkg_map) to a binary cache file.
    fn save_search_cache(&self, path: impl AsRef<std::path::Path>) -> std::io::Result<()> {
        use std::fs;
        use std::io::Write;

        if let Some(parent) = path.as_ref().parent() {
            fs::create_dir_all(parent)?;
        }

        let encoded = bincode::serde::encode_to_vec(&self.pkg_map, bincode::config::standard())
            .map_err(std::io::Error::other)?;

        let mut file = fs::File::create(path.as_ref())?;
        file.write_all(&encoded)?;
        Ok(())
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

/// String-similarity search, results sorted by `strsim::jaro_winkler` score.
pub struct StrSimSearch<'a> {
    apt_db: &'a AptDb,
    dpkg: &'a DpkgState,
}

impl OmaSearch for StrSimSearch<'_> {
    fn search(&self, query: &str) -> OmaSearchResult<Vec<SearchResult>> {
        let mut scored: Vec<(String, u16, bool, bool)> = Vec::new(); // (name, score, installed, upgradable)
        let query_lower = query.to_lowercase();

        for entry in &self.apt_db.entries {
            let name = &entry.package;
            if name.ends_with("-dbg") {
                continue;
            }

            let name_match = memmem::find(name.as_bytes(), query.as_bytes()).is_some();
            let desc_match = entry
                .description
                .as_deref()
                .is_some_and(|d| memmem::find(d.as_bytes(), query.as_bytes()).is_some());

            if !name_match && !desc_match {
                continue;
            }

            if scored.iter().any(|(n, _, _, _)| n == name) {
                continue;
            }

            let installed = self.dpkg.is_installed(name);
            let upgradable = installed
                && is_upgradable(
                    entry.version.as_ref(),
                    self.dpkg.installed_versions.get(name.as_str()),
                );
            let score = (strsim::jaro_winkler(name, query_lower.as_str()) * 1000.0) as u16;

            scored.push((name.clone(), score, installed, upgradable));
        }

        scored.sort_unstable_by(|a, b| b.1.cmp(&a.1).then_with(|| a.0.cmp(&b.0)));

        let mut results: Vec<SearchResult> = scored
            .into_iter()
            .map(|(name, _, installed, upgradable)| {
                let entry = self.apt_db.get(&name);
                let (old_version, new_version) = if let Some(e) = entry {
                    extract_versions(
                        if upgradable {
                            PackageStatus::Upgrade
                        } else if installed {
                            PackageStatus::Installed
                        } else {
                            PackageStatus::Avail
                        },
                        &self.dpkg.installed_versions,
                        &name,
                        &e.version,
                    )
                } else {
                    (None, "Unknown".to_string())
                };

                let desc = entry
                    .and_then(|e| {
                        e.description
                            .as_deref()
                            .map(|d| d.lines().next().unwrap_or(d).to_string())
                    })
                    .unwrap_or_else(|| "No description".to_string());

                let has_dbg =
                    entry.is_some_and(|_| self.apt_db.has_package(&format!("{name}-dbg")));

                SearchResult {
                    name: name.clone(),
                    desc,
                    old_version,
                    new_version,
                    full_match: query == name,
                    dbg_package: has_dbg,
                    status: if upgradable {
                        PackageStatus::Upgrade
                    } else if installed {
                        PackageStatus::Installed
                    } else {
                        PackageStatus::Avail
                    },
                    is_base: name.ends_with("-base"),
                }
            })
            .collect();

        results.sort_by_key(|b| std::cmp::Reverse(b.status));
        for i in 0..results.len() {
            if results[i].full_match {
                let r = results.remove(i);
                results.insert(0, r);
            }
        }

        Ok(results)
    }
}

impl<'a> StrSimSearch<'a> {
    pub fn new(apt_db: &'a AptDb, dpkg: &'a DpkgState) -> Self {
        Self { apt_db, dpkg }
    }
}

/// Text / glob match search based on `memmem`.
pub struct TextSearch<'a> {
    apt_db: &'a AptDb,
    dpkg: &'a DpkgState,
}

impl<'a> TextSearch<'a> {
    pub fn new(apt_db: &'a AptDb, dpkg: &'a DpkgState) -> Self {
        Self { apt_db, dpkg }
    }
}

impl OmaSearch for TextSearch<'_> {
    fn search(&self, query: &str) -> OmaSearchResult<Vec<SearchResult>> {
        let mut results = vec![];

        for entry in &self.apt_db.entries {
            let name = &entry.package;
            if name.ends_with("-dbg") {
                continue;
            }

            if !memmem::find(name.as_bytes(), query.as_bytes()).is_some()
                && !glob_match(query, name)
            {
                continue;
            }

            let installed = self.dpkg.is_installed(name);
            let upgradable = installed
                && is_upgradable(
                    entry.version.as_ref(),
                    self.dpkg.installed_versions.get(name.as_str()),
                );

            let (old_version, new_version) = extract_versions(
                if upgradable {
                    PackageStatus::Upgrade
                } else if installed {
                    PackageStatus::Installed
                } else {
                    PackageStatus::Avail
                },
                &self.dpkg.installed_versions,
                name,
                &entry.version,
            );

            let desc = entry
                .description
                .as_deref()
                .map(|d| d.lines().next().unwrap_or(d).to_string())
                .unwrap_or_else(|| "No description".to_string());

            let has_dbg = self.apt_db.has_package(&format!("{name}-dbg"));

            results.push(SearchResult {
                name: name.clone(),
                desc,
                old_version,
                new_version,
                full_match: query == name,
                dbg_package: has_dbg,
                status: if upgradable {
                    PackageStatus::Upgrade
                } else if installed {
                    PackageStatus::Installed
                } else {
                    PackageStatus::Avail
                },
                is_base: name.ends_with("-base"),
            });
        }

        results.sort_by_key(|b| std::cmp::Reverse(b.status));
        for i in 0..results.len() {
            if results[i].full_match {
                let r = results.remove(i);
                results.insert(0, r);
            }
        }

        Ok(results)
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_provides_simple() {
        assert_eq!(
            parse_provides("fish, fisher, fisherman"),
            vec!["fish", "fisher", "fisherman"]
        );
    }

    #[test]
    fn test_parse_provides_with_version_constraint() {
        let result = parse_provides("foo (= 1.0), bar (>= 2.0), baz");
        assert_eq!(result, vec!["foo", "bar", "baz"]);
    }

    #[test]
    fn test_parse_provides_empty() {
        let result: Vec<String> = parse_provides("");
        assert!(result.is_empty());
    }

    #[test]
    fn test_parse_provides_single() {
        assert_eq!(parse_provides("sh"), vec!["sh"]);
    }

    #[test]
    fn test_is_upgradable_newer_candidate() {
        // 4.8.1 > 4.7.1 → upgradable
        assert!(is_upgradable(
            Some(&"4.8.1".to_string()),
            Some(&"4.7.1".to_string()),
        ));
    }

    #[test]
    fn test_is_upgradable_same_version() {
        assert!(!is_upgradable(
            Some(&"4.8.1".to_string()),
            Some(&"4.8.1".to_string()),
        ));
    }

    #[test]
    fn test_is_upgradable_installed_newer() {
        // installed 1:3.3-1~pre > candidate 1:3.3 → not upgradable
        assert!(!is_upgradable(
            Some(&"1:3.3".to_string()),
            Some(&"1:3.3-1~pre20250407T092541Z".to_string()),
        ));
    }

    #[test]
    fn test_is_upgradable_tilde_handling() {
        // 2.0~rc1 < 2.0 → not upgradable
        assert!(!is_upgradable(
            Some(&"2.0~rc1".to_string()),
            Some(&"2.0".to_string()),
        ));
        // 2.0 > 2.0~rc1 → upgradable
        assert!(is_upgradable(
            Some(&"2.0".to_string()),
            Some(&"2.0~rc1".to_string()),
        ));
    }

    #[test]
    fn test_is_upgradable_epoch() {
        // 2:1.0 > 1:2.0 (epoch takes precedence)
        assert!(is_upgradable(
            Some(&"2:1.0".to_string()),
            Some(&"1:2.0".to_string()),
        ));
    }

    #[test]
    fn test_is_upgradable_not_installed() {
        assert!(!is_upgradable(Some(&"1.0".to_string()), None));
    }

    #[test]
    fn test_is_upgradable_no_candidate() {
        assert!(!is_upgradable(None, Some(&"1.0".to_string())));
    }

    #[test]
    fn test_build_available_names() {
        use crate::apt_lists::PackageEntry;

        let entries = vec![
            PackageEntry {
                package: "foo".into(),
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
            },
            PackageEntry {
                package: "foo-dbg".into(),
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
            },
        ];
        let db = AptDb::from_entries(entries);
        assert!(db.has_package("foo"));
        assert!(db.has_package("foo-dbg"));
    }
}
