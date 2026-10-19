//! Full-text package search with `indicium`.
//!
//! Builds a search index from parsed APT list entries and dpkg status,
//! without depending on the C++ `oma-apt` binding.

use std::collections::HashSet;
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::str::FromStr;

use ahash::RandomState;
use debian_control::lossy::Relations;
#[cfg(any(feature = "search-strsim", feature = "search-text"))]
use glob_match::glob_match;
use oma_logger::debug;
use serde::{Deserialize, Serialize};

use crate::apt_sources::SourceLookup;
use crate::internal::cache::CacheFile;
use crate::{AptConfig, AptState};

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
    /// The lists files this index was built from (filename + size +
    /// mtime), mirroring apt's PackageFile IMS records. Checked by
    /// [`crate::internal::cache::valid`] on cache load.
    pub(crate) files: Vec<CacheFile>,
}

/// Magic for the search-index cache file (`Dir::Cache::oma-search`); the
/// rest of the header layout is shared via [`crate::internal::cache`].
const SEARCH_CACHE_MAGIC: &[u8; 8] = b"OMASCH\x00\x00";

/// On-disk form of the search cache: the package map plus the lists files
/// it was built from.
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
    /// Build a new search index from the unified [`AptState`]: one entry per
    /// package group, using apt's preferred package (native, else the first
    /// configured architecture).
    pub fn new(state: &AptState, search_type: SearchType, progress: impl Fn(usize)) -> Self {
        let mut pkg_map: IndexMap<String, SearchEntry> = IndexMap::with_hasher(RandomState::new());
        for (i, entry) in build_search_entries(state).into_iter().enumerate() {
            progress(i);
            pkg_map.insert(entry.name.clone(), entry);
        }

        let mut search_index: SearchIndex<String> = SearchIndexBuilder::default()
            .search_type(search_type.clone())
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

    /// Refresh status and versions in place from a fresh state, adding new
    /// packages and dropping ones the indexes no longer carry. The index is
    /// only touched for those changes, so a cache hit does not rebuild it.
    pub fn refresh_from(&mut self, state: &AptState) {
        let Self { pkg_map, index, .. } = self;
        let mut seen: HashSet<&str> = HashSet::new();

        for group in state.groups() {
            let name = group.name();
            if name.ends_with("-dbg") {
                continue;
            }
            let Some(package) = group.preferred() else {
                continue;
            };
            let Some(candidate) = package.candidate() else {
                continue;
            };
            let installed = group.installed_version();
            let status = search_status(installed, candidate.version());
            let candidate_version = candidate.version().unwrap_or_default();
            seen.insert(name);

            // Candidate-derived fields (description, Provides, base section)
            // and the index change with the candidate, not only with status.
            if pkg_map.get(name).map(|entry| entry.new_version.as_str()) != Some(candidate_version)
            {
                if let Some(entry) = search_entry(state, name, &package, installed) {
                    if let Some(old) = pkg_map.insert(name.to_string(), entry) {
                        index.remove(&name.to_string(), &old);
                    }
                    if let Some(entry) = pkg_map.get(name) {
                        index.insert(&name.to_string(), entry);
                    }
                }
            } else if let Some(entry) = pkg_map.get_mut(name) {
                entry.status = status;
                entry.old_version = installed.map(str::to_string);
                entry.new_version = candidate_version.to_string();
            }
        }

        let removed: Vec<String> = pkg_map
            .keys()
            .filter(|name| !seen.contains(name.as_str()))
            .cloned()
            .collect();
        for name in removed {
            if let Some(entry) = pkg_map.shift_remove(&name) {
                index.remove(&name, &entry);
            }
        }
    }
}

/// One search entry per package name, taken from apt's preferred package.
fn build_search_entries(state: &AptState) -> Vec<SearchEntry> {
    let mut entries = Vec::new();
    for group in state.groups() {
        let name = group.name();
        if name.ends_with("-dbg") {
            continue;
        }
        let Some(package) = group.preferred() else {
            continue;
        };
        if let Some(entry) = search_entry(state, name, &package, group.installed_version()) {
            entries.push(entry);
        }
    }
    entries
}

fn search_status(installed: Option<&str>, candidate: Option<&str>) -> PackageStatus {
    match installed {
        Some(installed) if is_upgradable(candidate, Some(installed)) => PackageStatus::Upgrade,
        Some(_) => PackageStatus::Installed,
        None => PackageStatus::Avail,
    }
}

fn search_entry(
    state: &AptState,
    name: &str,
    package: &crate::Package<'_>,
    installed: Option<&str>,
) -> Option<SearchEntry> {
    let candidate = package.candidate()?;
    let description = candidate
        .control
        .description
        .as_deref()
        .map(|d| d.lines().next().unwrap_or(d).to_string())
        .unwrap_or_else(|| "No description".to_string());
    let provides: IndexSet<String> = candidate
        .control
        .provides
        .as_deref()
        .map(|s| {
            s.parse::<Relations>()
                .map(|r| r.iter().flatten().map(|d| d.name.clone()).collect())
                .unwrap_or_default()
        })
        .unwrap_or_default();

    Some(SearchEntry {
        name: name.to_string(),
        description,
        status: search_status(installed, candidate.version()),
        provides,
        has_dbg: state.group(&format!("{name}-dbg")).is_some(),
        section_is_base: candidate.control.section.as_deref() == Some("Bases"),
        old_version: installed.map(str::to_string),
        new_version: candidate.version().unwrap_or_default().to_string(),
    })
}

#[cfg(feature = "search-indicium")]
impl IndiciumSearch {
    /// Build a search index, optionally loading from search cache when valid.
    ///
    /// * If `search_cache_path` is valid the index is loaded from cache
    ///   (fastest path) and the status is refreshed from `dpkg`.
    /// * Otherwise the index is built from `apt_db` + `dpkg` and persisted
    ///   to the search cache for next time.
    ///
    /// `apt_cfg` supplies everything: the lists directory and source lookup
    /// used to validate the cache (`Dir::State::lists`), and the search
    /// cache path (`Dir::Cache::oma-search` — overridden to `~/.cache/oma/`
    /// for unprivileged users, the system default otherwise).
    pub fn new_with_cache(
        state: &AptState,
        apt_cfg: &AptConfig,
        search_type: SearchType,
        progress: impl Fn(usize),
    ) -> Result<Self, crate::error::Error> {
        let lists_dir = apt_cfg.get_dir("Dir::State::lists", "var/lib/apt/lists");
        let search_cache_path =
            apt_cfg.get_file("Dir::Cache::oma-search", "var/cache/apt/oma-search.bincode");
        let lookup = SourceLookup::build(apt_cfg);
        let archs = apt_cfg.architectures();

        // Tier 1: try search cache (fastest): load it, then check the
        // lists files it records having been built from against the current
        // state.
        if let Some(mut searcher) = Self::load_search_cache(&search_cache_path, search_type.clone())
            && crate::internal::cache::valid(
                &search_cache_path,
                &lists_dir,
                &lookup,
                &archs,
                &searcher.files,
            )
        {
            debug!("Search cache hit");
            searcher.refresh_from(state);
            return Ok(searcher);
        }

        debug!("Search cache miss, building index ...");

        let mut searcher = Self::new(state, search_type, progress);
        searcher.files = state.files().to_vec();

        // Persist search cache for next time
        if let Err(e) = searcher.save_search_cache(&search_cache_path) {
            debug!("Failed to save search cache: {e}");
        } else {
            debug!("Search cache saved");
        }

        Ok(searcher)
    }

    /// Try to load a previously saved search index from its binary cache.
    ///
    /// The file is a validated rkyv archive behind the shared cache header;
    /// a foreign, corrupt or version-mismatched file simply reads as a miss
    /// and the index is rebuilt.
    fn load_search_cache(path: impl AsRef<Path>, search_type: SearchType) -> Option<Self> {
        let bytes = fs::read(path.as_ref()).ok()?;
        if !crate::internal::cache::header_ok(&bytes, SEARCH_CACHE_MAGIC) {
            return None;
        }
        let archived = rkyv::access::<ArchivedSearchCache, rkyv::rancor::Error>(
            &bytes[crate::internal::cache::CACHE_HEADER_LEN..],
        )
        .ok()?;
        let cache: SearchCache = crate::internal::cache::from_archived(archived);

        let mut search_index: SearchIndex<String> = SearchIndexBuilder::default()
            .search_type(search_type.clone())
            .exclude_keywords(None)
            .build();
        cache.pkg_map.iter().for_each(|(key, value)| {
            search_index.insert(key, value);
        });

        Some(Self {
            pkg_map: cache.pkg_map,
            index: search_index,
            files: cache.files,
        })
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

        let mut buf = Vec::with_capacity(crate::internal::cache::CACHE_HEADER_LEN + archive.len());
        crate::internal::cache::push_header(&mut buf, SEARCH_CACHE_MAGIC);
        buf.extend_from_slice(&archive);

        let mut tmp = path.as_os_str().to_owned();
        tmp.push(".tmp");
        let tmp = PathBuf::from(tmp);

        let mut file = fs::File::create(&tmp)?;
        file.write_all(&buf)?;
        file.sync_all()?;
        fs::rename(&tmp, path)?;

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
                // Directional string fallback: only an unparsable candidate
                // that sorts above the installed string counts as an upgrade.
                _ => cand > inst,
            }
        }
        (Some(_), None) => false, // not installed
        (None, _) => false,       // no candidate available
    }
}

#[cfg(feature = "search-strsim")]
pub struct StrSimSearch {
    entries: Vec<SearchEntry>,
}

#[cfg(feature = "search-strsim")]
impl StrSimSearch {
    /// Build the name/description table from the unified state.
    pub fn new(state: &AptState) -> Self {
        Self {
            entries: build_search_entries(state),
        }
    }
}

#[cfg(feature = "search-strsim")]
impl OmaSearch for StrSimSearch {
    fn search(&self, query: &str) -> OmaSearchResult<Vec<SearchResult>> {
        let query_lower = query.to_lowercase();
        let mut scored: Vec<(usize, u16)> = Vec::new();

        for (i, entry) in self.entries.iter().enumerate() {
            let name_match = memmem::find(entry.name.as_bytes(), query.as_bytes()).is_some();
            let desc_match = memmem::find(entry.description.as_bytes(), query.as_bytes()).is_some();
            if !name_match && !desc_match {
                continue;
            }
            let score = (strsim::jaro_winkler(&entry.name, &query_lower) * 1000.0) as u16;
            scored.push((i, score));
        }

        scored.sort_unstable_by(|a, b| {
            b.1.cmp(&a.1)
                .then_with(|| self.entries[a.0].name.cmp(&self.entries[b.0].name))
        });

        let results: Vec<SearchResult> = scored
            .into_iter()
            .map(|(i, _)| entry_to_result(&self.entries[i], query))
            .collect();
        Ok(results)
    }
}

#[cfg(feature = "search-text")]
pub struct TextSearch {
    entries: Vec<SearchEntry>,
}

#[cfg(feature = "search-text")]
impl TextSearch {
    /// Build the text-search table from the unified state.
    pub fn new(state: &AptState) -> Self {
        Self {
            entries: build_search_entries(state),
        }
    }
}

#[cfg(feature = "search-text")]
impl OmaSearch for TextSearch {
    fn search(&self, query: &str) -> OmaSearchResult<Vec<SearchResult>> {
        let mut results: Vec<SearchResult> = self
            .entries
            .iter()
            .filter(|entry| {
                memmem::find(entry.name.as_bytes(), query.as_bytes()).is_some()
                    || glob_match(query, &entry.name)
            })
            .map(|entry| entry_to_result(entry, query))
            .collect();

        sort_and_promote(&mut results);
        Ok(results)
    }
}

/// Convert a prebuilt search entry to the caller-facing result.
#[cfg(any(feature = "search-strsim", feature = "search-text"))]
fn entry_to_result(entry: &SearchEntry, query: &str) -> SearchResult {
    SearchResult {
        name: entry.name.clone(),
        desc: entry.description.clone(),
        old_version: entry.old_version.clone(),
        new_version: entry.new_version.clone(),
        full_match: query == entry.name,
        dbg_package: entry.has_dbg,
        status: entry.status,
        is_base: entry.name.ends_with("-base"),
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
    fn test_is_upgradable_more_trailing_segments() {
        // dpkg: 1.0 < 1.0.0 (longer trailing version segment is newer)
        assert!(is_upgradable(Some("1.0.0"), Some("1.0")));
        assert!(!is_upgradable(Some("1.0"), Some("1.0.0")));
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
    fn test_is_upgradable_malformed_fallback_is_directional() {
        // An unparsable candidate that sorts below the installed version is
        // not an upgrade: the string fallback compares directionally instead
        // of reporting any unequal pair as an upgrade.
        assert!(!is_upgradable(Some("1.0!"), Some("5.0")));
        // An unparsable candidate that sorts above → upgradable.
        assert!(is_upgradable(Some("5.0!"), Some("1.0")));
        // Identical unparsable strings → not upgradable.
        assert!(!is_upgradable(Some("1.0!"), Some("1.0!")));
    }

    /// Round-trip the search cache through its rkyv file format: save a
    /// pkg_map, load it back, and verify the entry survives — exercising
    /// rkyv's `IndexMap`/`IndexSet` support with the `ahash::RandomState`
    /// hasher, `PackageStatus`, and the shared cache header.
    #[cfg(feature = "search-indicium")]
    #[test]
    fn test_search_cache_roundtrip() {
        let mut pkg_map: IndexMap<String, SearchEntry> = IndexMap::with_hasher(RandomState::new());
        pkg_map.insert(
            "fish".to_string(),
            SearchEntry {
                name: "fish".to_string(),
                description: "Friendly Interactive SHell".to_string(),
                status: PackageStatus::Installed,
                provides: IndexSet::from_iter(["fish".to_string()]),
                has_dbg: true,
                section_is_base: false,
                old_version: Some("3.6".to_string()),
                new_version: "4.8.1".to_string(),
            },
        );

        let searcher = IndiciumSearch {
            pkg_map,
            index: SearchIndexBuilder::default()
                .search_type(SearchType::Live)
                .exclude_keywords(None)
                .build(),
            files: Vec::new(),
        };

        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("oma-search.bincode");
        searcher.save_search_cache(&path).unwrap();

        let loaded = IndiciumSearch::load_search_cache(&path, SearchType::Live)
            .expect("round-tripped search cache loads");
        assert_eq!(loaded.pkg_map.len(), 1);
        let entry = &loaded.pkg_map["fish"];
        assert_eq!(entry.name, "fish");
        assert_eq!(entry.status, PackageStatus::Installed);
        assert_eq!(entry.provides.len(), 1);
        assert!(entry.provides.contains("fish"));

        // A foreign or corrupt file must read as a miss, not crash.
        let garbage = dir.path().join("garbage");
        std::fs::write(&garbage, b"not a cache").unwrap();
        assert!(IndiciumSearch::load_search_cache(&garbage, SearchType::Live).is_none());
    }
}
