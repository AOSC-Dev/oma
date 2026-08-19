//! Full-text package search with `indicium`.
//!
//! Builds a search index from parsed APT list entries and dpkg status,
//! without depending on the C++ `oma-apt` binding.

use std::collections::HashSet;
use std::fs;
use std::io::Read;
use std::io::Write;
use std::path::Path;
use std::str::FromStr;

use ahash::RandomState;
#[cfg(any(feature = "search-strsim", feature = "search-text"))]
use glob_match::glob_match;
use serde::{Deserialize, Serialize};
use spdlog::debug;

use crate::apt_sources::SourceLookup;
use crate::cache::{self, CacheFile};
use crate::{AptConfig, AptDb, DpkgState};

#[cfg(feature = "search-indicium")]
pub use indicium::simple::SearchType;
#[cfg(feature = "search-indicium")]
use indicium::simple::{Indexable, SearchIndex, SearchIndexBuilder};
#[cfg(any(feature = "search-strsim", feature = "search-text"))]
use memchr::memmem;

type IndexSet<T> = indexmap::IndexSet<T, RandomState>;
type IndexMap<K, V> = indexmap::IndexMap<K, V, RandomState>;

/// Status of the package.
#[derive(PartialEq, Eq, Debug, Clone, Copy, Serialize, Deserialize)]
#[cfg_attr(
    feature = "apt-lists",
    derive(rkyv::Archive, rkyv::Serialize, rkyv::Deserialize)
)]
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
#[cfg_attr(
    feature = "apt-lists",
    derive(rkyv::Archive, rkyv::Serialize, rkyv::Deserialize)
)]
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

#[cfg(feature = "search-indicium")]
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
#[cfg_attr(
    feature = "apt-lists",
    derive(rkyv::Archive, rkyv::Serialize, rkyv::Deserialize)
)]
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
#[cfg(feature = "search-indicium")]
pub struct IndiciumSearch {
    /// Map contains package names and their corresponding search entries.
    pub pkg_map: IndexMap<String, SearchEntry>,
    /// Index used to perform search operations.
    pub index: SearchIndex<String>,
    /// The lists files this index was built from (filename + size + mtime),
    /// mirroring apt's PackageFile IMS records. Checked by
    /// [`crate::cache::valid`] on cache load.
    pub(crate) files: Vec<CacheFile>,
}

/// Magic for the search-index cache file (`Dir::Cache::oma-search`); the rest
/// of the header layout is shared via [`crate::cache`].
const SEARCH_CACHE_MAGIC: &[u8; 8] = b"OMASCH\x00\x00";

/// On-disk form of the search cache: the package map plus the lists files it
/// was built from.
#[cfg_attr(
    feature = "apt-lists",
    derive(rkyv::Archive, rkyv::Serialize, rkyv::Deserialize)
)]
struct SearchCache {
    pkg_map: IndexMap<String, SearchEntry>,
    files: Vec<CacheFile>,
}

pub trait OmaSearch {
    fn search(&self, query: &str) -> OmaSearchResult<Vec<SearchResult>>;
}

#[cfg(feature = "search-indicium")]
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

        sort_and_promote(&mut search_res);

        Ok(search_res)
    }
}

#[cfg(feature = "search-indicium")]
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

        // Names whose `-dbg` companion is present, so `has_dbg` below is a
        // set lookup instead of allocating `format!("{name}-dbg")` per
        // package.
        let dbg_bases: HashSet<String> = apt_db
            .entries()
            .filter_map(|e| e.package.strip_suffix("-dbg").map(str::to_owned))
            .collect();

        for (i, entry) in apt_db.entries().enumerate() {
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
                if is_upgradable(entry.version.as_deref(), dpkg.installed_version(name)) {
                    PackageStatus::Upgrade
                } else {
                    PackageStatus::Installed
                }
            } else {
                PackageStatus::Avail
            };

            let (old_version, new_version) = extract_versions(status, dpkg, name, &entry.version);

            let description = entry
                .description
                .as_deref()
                .map(|d| d.lines().next().unwrap_or(d).to_string())
                .unwrap_or_else(|| "No description".to_string());

            let provides: IndexSet<String> = entry
                .version
                .as_deref()
                .and_then(|v| apt_db.deps_of(&entry.package, v))
                .map(|deps| deps.provides.iter().map(|d| d.name.clone()).collect())
                .unwrap_or_default();

            let has_dbg = dbg_bases.contains(name.as_str());

            let section_is_base = entry.section.as_deref().is_some_and(|s| s == "Bases");

            pkg_map.insert(
                name.clone(),
                SearchEntry {
                    name: name.clone(),
                    description,
                    status,
                    provides,
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
            files: Vec::new(),
        }
    }

    /// Refreshes the package status of existing entries and adds
    /// new entries that appear in `apt_db` but have not yet been included
    /// in the search index (e.g packages added to the software
    /// sources since the search cache was created).
    pub fn refresh_from(&mut self, apt_db: &AptDb, dpkg: &DpkgState) {
        // Names whose `-dbg` companion is present (see `IndiciumSearch::new`).
        let dbg_bases: HashSet<String> = apt_db
            .entries()
            .filter_map(|e| e.package.strip_suffix("-dbg").map(str::to_owned))
            .collect();

        for entry in apt_db.entries() {
            let name = &entry.package;
            if name.ends_with("-dbg") {
                continue;
            }

            let is_new = !self.pkg_map.contains_key(name);

            if is_new {
                // 源里的新包
                let has_dbg = dbg_bases.contains(name.as_str());

                let provides: IndexSet<String> = entry
                    .version
                    .as_deref()
                    .and_then(|v| apt_db.deps_of(&entry.package, v))
                    .map(|deps| deps.provides.iter().map(|d| d.name.clone()).collect())
                    .unwrap_or_default();

                let section_is_base = entry.section.as_deref().is_some_and(|s| s == "Bases");
                let description = entry
                    .description
                    .as_deref()
                    .map(|d| d.lines().next().unwrap_or(d).to_string())
                    .unwrap_or_else(|| "No description".to_string());

                self.pkg_map.insert(
                    name.clone(),
                    SearchEntry {
                        name: name.clone(),
                        description,
                        status: PackageStatus::Avail,
                        provides,
                        has_dbg,
                        section_is_base,
                        old_version: None,
                        new_version: entry.version.clone().unwrap_or_default(),
                    },
                );

                self.index.insert(name, &self.pkg_map[name]);
            }

            let pkg_entry = self.pkg_map.get_mut(name).unwrap();

            // 更新已安装的包的状态
            if dpkg.is_installed(name) {
                let inst_ver = dpkg.installed_version(name).map(str::to_string);
                if is_upgradable(entry.version.as_deref(), dpkg.installed_version(name)) {
                    pkg_entry.status = PackageStatus::Upgrade;
                    pkg_entry.old_version = inst_ver;
                    if let Some(ref v) = entry.version {
                        pkg_entry.new_version.clone_from(v);
                    }
                } else {
                    pkg_entry.status = PackageStatus::Installed;
                    pkg_entry.old_version = inst_ver;
                }
            } else {
                // 未安装的包
                pkg_entry.status = PackageStatus::Avail;
                pkg_entry.old_version = None;
                if let Some(ref v) = entry.version {
                    pkg_entry.new_version.clone_from(v);
                }
            }
        }

        // 移除源里已删除的包
        let current: HashSet<String> = apt_db
            .entries()
            .filter(|e| !e.package.ends_with("-dbg"))
            .map(|e| e.package.clone())
            .collect();

        self.pkg_map.retain(|name, entry| {
            if current.contains(name) {
                true
            } else {
                self.index.remove(name, entry);
                false
            }
        });
    }
}

#[cfg(feature = "search-indicium")]
impl IndiciumSearch {
    /// Build a search index, optionally loading from search cache when valid.
    ///
    /// * If `search_cache_path` is valid the index is loaded from cache
    ///   (fastest path) and the status is refreshed from `dpkg`.
    /// * Otherwise the index is built from `apt_db` + `dpkg` and persisted
    ///   to the search cache for next time.
    pub fn new_with_cache(
        apt_db: &AptDb,
        dpkg: &DpkgState,
        apt_cfg: &AptConfig,
        search_type: SearchType,
        progress: impl Fn(usize),
    ) -> Result<Self, crate::error::Error> {
        let lists_dir = apt_cfg.get_dir("Dir::State::lists", "var/lib/apt/lists");
        let search_cache_path =
            apt_cfg.get_file("Dir::Cache::oma-search", "var/cache/apt/oma-search.bincode");
        let archs = apt_cfg.architectures();
        let lookup = SourceLookup::build(apt_cfg);

        // Tier 1: try search cache (fastest)
        if let Some(cache) = Self::load_search_cache(&search_cache_path)
            && cache::valid(
                &search_cache_path,
                &lists_dir,
                &lookup,
                &archs,
                &cache.files,
            )
        {
            debug!("Search cache hit");
            let mut searcher = Self::from_cache(cache, search_type);
            searcher.refresh_from(apt_db, dpkg);
            return Ok(searcher);
        }

        debug!("Search cache miss, building index ...");

        let mut searcher = Self::new(apt_db, dpkg, search_type, progress);
        searcher.files = cache::collect(&lists_dir, &lookup, &archs);

        // Persist search cache for next time
        if let Err(e) = searcher.save_search_cache(&search_cache_path) {
            debug!("Failed to save search cache: {e}");
        } else {
            debug!("Search cache saved");
        }

        Ok(searcher)
    }

    /// Rebuild the in-memory index from a loaded cache.
    fn from_cache(cache: SearchCache, search_type: SearchType) -> Self {
        let mut search_index: SearchIndex<String> = SearchIndexBuilder::default()
            .search_type(search_type)
            .exclude_keywords(None)
            .build();
        cache.pkg_map.iter().for_each(|(key, value)| {
            search_index.insert(key, value);
        });

        Self {
            pkg_map: cache.pkg_map,
            index: search_index,
            files: cache.files,
        }
    }

    /// Try to load a previously saved search index from its rkyv cache. The
    /// archive must carry the shared cache header; returns `None` on any
    /// failure so the caller falls back to building.
    fn load_search_cache(path: impl AsRef<Path>) -> Option<SearchCache> {
        let mut bytes = Vec::new();
        fs::File::open(path.as_ref())
            .and_then(|mut f| f.read_to_end(&mut bytes))
            .ok()?;

        if !cache::header_ok(&bytes, SEARCH_CACHE_MAGIC) {
            return None;
        }

        let archived = rkyv::access::<ArchivedSearchCache, rkyv::rancor::Error>(
            &bytes[cache::CACHE_HEADER_LEN..],
        )
        .ok()?;
        Some(cache::from_archived(archived))
    }

    /// Save the search index (pkg_map) to a binary cache file, atomically
    /// (temp file + rename).
    fn save_search_cache(&self, path: impl AsRef<Path>) -> std::io::Result<()> {
        let path = path.as_ref();
        let archive = rkyv::to_bytes::<rkyv::rancor::Error>(&SearchCache {
            pkg_map: self.pkg_map.clone(),
            files: self.files.clone(),
        })
        .map_err(std::io::Error::other)?;

        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }

        let mut buf = Vec::with_capacity(cache::CACHE_HEADER_LEN + archive.len());
        cache::push_header(&mut buf, SEARCH_CACHE_MAGIC);
        buf.extend_from_slice(&archive);

        let mut tmp = path.as_os_str().to_owned();
        tmp.push(".tmp");
        let tmp = std::path::PathBuf::from(tmp);

        let mut file = fs::File::create(&tmp)?;
        file.write_all(&buf)?;
        file.sync_all()?;
        fs::rename(tmp, path)?;
        Ok(())
    }
}

/// Determine if a package is upgradable: a newer version is available in the
/// repo than what is currently installed.
/// Uses proper Debian version comparison (handles `~`, epochs, etc.).
fn is_upgradable(candidate_version: Option<&str>, installed_version: Option<&str>) -> bool {
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
#[cfg(feature = "search-strsim")]
pub struct StrSimSearch<'a> {
    apt_db: &'a AptDb,
    dpkg: &'a DpkgState,
}

#[cfg(feature = "search-strsim")]
impl OmaSearch for StrSimSearch<'_> {
    fn search(&self, query: &str) -> OmaSearchResult<Vec<SearchResult>> {
        let mut scored: Vec<(String, u16, bool, bool)> = Vec::new(); // (name, score, installed, upgradable)
        let query_lower = query.to_lowercase();

        for entry in self.apt_db.entries() {
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
                && is_upgradable(entry.version.as_deref(), self.dpkg.installed_version(name));
            let score = (strsim::jaro_winkler(name, query_lower.as_str()) * 1000.0) as u16;

            scored.push((name.clone(), score, installed, upgradable));
        }

        scored.sort_unstable_by(|a, b| b.1.cmp(&a.1).then_with(|| a.0.cmp(&b.0)));

        let mut results: Vec<SearchResult> = scored
            .into_iter()
            .map(|(name, _, installed, upgradable)| {
                let entry = self.apt_db.get_candidate(&name);
                let (old_version, new_version) = if let Some(e) = entry.as_ref() {
                    extract_versions(
                        if upgradable {
                            PackageStatus::Upgrade
                        } else if installed {
                            PackageStatus::Installed
                        } else {
                            PackageStatus::Avail
                        },
                        self.dpkg,
                        &name,
                        &e.version,
                    )
                } else {
                    (None, "Unknown".to_string())
                };

                let desc = entry
                    .as_ref()
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

        sort_and_promote(&mut results);

        Ok(results)
    }
}

#[cfg(feature = "search-strsim")]
impl<'a> StrSimSearch<'a> {
    pub fn new(apt_db: &'a AptDb, dpkg: &'a DpkgState) -> Self {
        Self { apt_db, dpkg }
    }
}

/// Text / glob match search based on `memmem`.
#[cfg(feature = "search-text")]
pub struct TextSearch<'a> {
    apt_db: &'a AptDb,
    dpkg: &'a DpkgState,
}

#[cfg(feature = "search-text")]
impl<'a> TextSearch<'a> {
    pub fn new(apt_db: &'a AptDb, dpkg: &'a DpkgState) -> Self {
        Self { apt_db, dpkg }
    }
}

#[cfg(feature = "search-text")]
impl OmaSearch for TextSearch<'_> {
    fn search(&self, query: &str) -> OmaSearchResult<Vec<SearchResult>> {
        let mut results = vec![];

        for entry in self.apt_db.entries() {
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
                && is_upgradable(entry.version.as_deref(), self.dpkg.installed_version(name));

            let (old_version, new_version) = extract_versions(
                if upgradable {
                    PackageStatus::Upgrade
                } else if installed {
                    PackageStatus::Installed
                } else {
                    PackageStatus::Avail
                },
                self.dpkg,
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

        sort_and_promote(&mut results);

        Ok(results)
    }
}

/// Sort results by status (Upgrade > Installed > Avail)
/// and make full-match entries to the front.
fn sort_and_promote(results: &mut [SearchResult]) {
    results.sort_by(|a, b| match (a.full_match, b.full_match) {
        // Full-match 的包总在最前面
        (true, false) => std::cmp::Ordering::Less,
        (false, true) => std::cmp::Ordering::Greater,
        // 如果没有 full match 的包，则排序包的状态
        _ => b.status.cmp(&a.status),
    });
}

fn extract_versions(
    status: PackageStatus,
    dpkg: &DpkgState,
    name: &str,
    candidate_version: &Option<String>,
) -> (Option<String>, String) {
    let new = candidate_version
        .clone()
        .unwrap_or_else(|| "Unknown".to_string());

    // Keep the installed version for all installed packages, not just upgrades.
    let old = if status == PackageStatus::Installed || status == PackageStatus::Upgrade {
        dpkg.installed_version(name).map(str::to_string)
    } else {
        None
    };

    (old, new)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_upgradable_newer_candidate() {
        // 4.8.1 > 4.7.1 → upgradable
        assert!(is_upgradable(Some("4.8.1"), Some("4.7.1")));
    }

    #[test]
    fn test_is_upgradable_same_version() {
        assert!(!is_upgradable(Some("4.8.1"), Some("4.8.1")));
    }

    #[test]
    fn test_is_upgradable_installed_newer() {
        // installed 1:3.3-1~pre > candidate 1:3.3 → not upgradable
        assert!(!is_upgradable(
            Some("1:3.3"),
            Some("1:3.3-1~pre20250407T092541Z"),
        ));
    }

    #[test]
    fn test_is_upgradable_tilde_handling() {
        // 2.0~rc1 < 2.0 → not upgradable
        assert!(!is_upgradable(Some("2.0~rc1"), Some("2.0")));
        // 2.0 > 2.0~rc1 → upgradable
        assert!(is_upgradable(Some("2.0"), Some("2.0~rc1")));
    }

    #[test]
    fn test_is_upgradable_epoch() {
        // 2:1.0 > 1:2.0 (epoch takes precedence)
        assert!(is_upgradable(Some("2:1.0"), Some("1:2.0")));
    }

    #[test]
    fn test_is_upgradable_not_installed() {
        assert!(!is_upgradable(Some("1.0"), None));
    }

    #[test]
    fn test_is_upgradable_no_candidate() {
        assert!(!is_upgradable(None, Some("1.0")));
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
                essential: None,
                protected: None,
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
                essential: None,
                protected: None,
            },
        ];
        let db = AptDb::from_entries("", entries);
        assert!(db.has_package("foo"));
        assert!(db.has_package("foo-dbg"));
    }
}
