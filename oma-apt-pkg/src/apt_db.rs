//! oma package database — Parse APT `Packages` files with binary cache support.

use std::borrow::Cow;
use std::collections::HashMap;
use std::io::Read;
use std::io::Write;
use std::path::Path;
use std::{fs, io};

use rayon::prelude::*;
use spdlog::debug;
use wincode::{SchemaRead, SchemaWrite};

use crate::apt_lists::{
    EntriesWithSource, PackageEntry, PackageIndex, parse_apt_lists_dir_with_sources,
};
use crate::package_matcher::PackageMatcher;

/// A package entry together with its source file information.
///
/// The entry is borrowed from the database when it comes from an APT lists
/// file, or owned when it is a local `.deb` (whose source is the `file:` URL
/// recorded at insert time).
#[derive(Debug, Clone)]
pub struct EntryWithSource<'a> {
    /// The parsed package entry data.
    pub entry: Cow<'a, PackageEntry>,
    /// The APT lists filename, e.g.
    /// `mirrors.example.com_debian_dists_bookworm_main_binary-amd64_Packages`,
    /// or the `file:` source of a local `.deb`.
    pub source: Option<Cow<'a, str>>,
}

/// Errors that can occur when resolving package queries.
#[derive(Debug, thiserror::Error)]
pub enum QueryError {
    #[error("Failed to parse .deb: {0}")]
    Deb(#[from] crate::deb::DebError),
    #[error(transparent)]
    Matcher(#[from] crate::package_matcher::MatcherError),
}

/// Result of resolving package queries.
#[derive(Debug)]
pub struct QueryResolution<'a> {
    /// Display groups in query order; each group holds the (version/source
    /// filtered) entries for one query — all versions of a package, a single
    /// version (`pkg=1.2.3`), one branch (`pkg/suite`) or a local `.deb`.
    pub groups: Vec<Vec<EntryWithSource<'a>>>,
    /// Queries that matched no package.
    pub no_match: Vec<String>,
}

/// Build the `file:` URI source for a local `.deb` path, e.g.
/// `file:/home/oma/go_1.26.4%2btools0.45.0_amd64.deb`. The path is
/// percent-encoded with lowercase hex (e.g. `+` → `%2b`) to match APT's URI
/// form. Like repository sources (which are stored as URIs), the
/// `local-deb/local-deb` suite/component is added when the source is
/// rendered.
fn local_deb_source(path: impl AsRef<Path>) -> String {
    let path = path.as_ref();
    let canonical = fs::canonicalize(path).unwrap_or_else(|_| path.to_path_buf());
    let raw = canonical.to_string_lossy();

    let mut encoded = String::with_capacity(raw.len());
    for &byte in raw.as_bytes() {
        match byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' | b'/' => {
                encoded.push(char::from(byte));
            }
            _ => encoded.push_str(&format!("%{byte:02x}")),
        }
    }

    format!("file:{encoded}")
}

/// Parse and cache APT package database.
#[derive(Debug, Clone, SchemaWrite, SchemaRead)]
pub struct AptDb {
    /// Map from package name to package version entries
    pub(crate) entries: HashMap<String, Vec<PackageEntry>>,
    /// Map from package name to apt lists filenames
    pub(crate) entry_sources: HashMap<String, Vec<String>>,
}

impl AptDb {
    /// Build from entries without source tracking
    #[allow(dead_code)]
    pub(crate) fn from_entries(entries: Vec<PackageEntry>) -> Self {
        let mut map: HashMap<String, Vec<PackageEntry>> = HashMap::new();
        let mut sources: HashMap<String, Vec<String>> = HashMap::new();
        for e in entries {
            let name = e.package.clone();
            map.entry(name.clone()).or_default().push(e);
            // Keep `entry_sources` in lockstep; empty string means "no source".
            sources.entry(name).or_default().push(String::new());
        }
        Self {
            entry_sources: sources,
            entries: map,
        }
    }

    /// Insert a local package entry (e.g. parsed from a local `.deb`) into
    /// the database.
    ///
    /// Local packages have no APT list source, so
    /// [`get_all_with_source`](Self::get_all_with_source) reports
    /// `source: None` for them.
    pub fn insert(&mut self, entry: PackageEntry) {
        let name = entry.package.clone();
        self.entries.entry(name.clone()).or_default().push(entry);
        // Keep `entry_sources` in lockstep; empty string means "no source".
        self.entry_sources
            .entry(name)
            .or_default()
            .push(String::new());
    }

    /// Insert a package entry together with its source, keeping `entries`
    /// and `entry_sources` in sync so
    /// [`get_all_with_source`](Self::get_all_with_source) reports the source.
    pub fn insert_with_source(&mut self, entry: PackageEntry, source: String) {
        let name = entry.package.clone();
        self.entries.entry(name.clone()).or_default().push(entry);
        self.entry_sources.entry(name).or_default().push(source);
    }

    /// Parse a local `.deb` file and insert its control entry into the
    /// database as a local package, recording its `file:` source. Returns
    /// the package name.
    pub fn insert_from_deb(
        &mut self,
        path: impl AsRef<Path>,
    ) -> Result<String, crate::deb::DebError> {
        let entry = crate::deb::parse_deb(&path)?;
        let name = entry.package.clone();
        let source = local_deb_source(path);
        self.insert_with_source(entry, source);

        Ok(name)
    }

    /// Resolve package queries into display groups.
    ///
    /// Each query is either a path to a local `.deb` file or a package
    /// name/glob/version/branch expression matched via [`PackageMatcher`].
    /// Local `.deb`s are parsed (in parallel), inserted with their `file:`
    /// source, and resolved like `pkg=<version>` so their own version is
    /// shown — merged with any repo entries of that version — consistent
    /// with `pkg=1.2.3` / `pkg/suite` queries.
    ///
    /// Note: this inserts the local packages into the database for the
    /// lifetime of this instance; the caller owns the database, so that is
    /// harmless per process.
    pub fn resolve_queries(
        &mut self,
        queries: Vec<String>,
    ) -> Result<QueryResolution<'_>, QueryError> {
        let (deb_files, names): (Vec<String>, Vec<String>) = queries
            .into_iter()
            .partition(|q| q.ends_with(".deb") && Path::new(q).is_file());

        let deb_entries: Vec<PackageEntry> = deb_files
            .par_iter()
            .map(crate::deb::parse_deb)
            .collect::<Result<_, _>>()?;

        let mut keywords: Vec<String> = Vec::with_capacity(deb_files.len() + names.len());
        for (path, entry) in deb_files.iter().zip(deb_entries) {
            let source = local_deb_source(path);
            let name = entry.package.clone();
            let version = entry.version.clone();
            self.insert_with_source(entry, source);
            // Resolve the `.deb` like `pkg=<version>` so its own version is
            // displayed (merged with any repo entries of that version).
            keywords.push(match version {
                Some(v) => format!("{name}={v}"),
                None => name,
            });
        }
        keywords.extend(names);

        let mut no_match = Vec::new();
        let mut groups = Vec::new();

        if !keywords.is_empty() {
            let matcher = PackageMatcher::new(self);
            let (matched, no_result) =
                matcher.match_pkgs_and_versions(keywords.iter().map(String::as_str))?;

            groups.extend(matched.into_iter().map(|pkg| {
                pkg.entries
                    .into_iter()
                    .map(|(entry, source)| EntryWithSource {
                        entry,
                        source: (!source.is_empty()).then_some(Cow::Owned(source)),
                    })
                    .collect()
            }));
            no_match = no_result.into_iter().map(str::to_owned).collect();
        }

        Ok(QueryResolution { groups, no_match })
    }

    /// Build from entries with parallel source tracking.
    pub(crate) fn from_entries_with_sources(
        entries: Vec<PackageEntry>,
        entry_sources: Vec<String>,
    ) -> Self {
        let mut map: HashMap<String, Vec<PackageEntry>> = HashMap::new();
        let mut sources: HashMap<String, Vec<String>> = HashMap::new();

        for (e, src) in entries.into_iter().zip(entry_sources) {
            let pkg = e.package.clone();
            sources.entry(pkg.clone()).or_default().push(src);
            map.entry(pkg).or_default().push(e);
        }

        Self {
            entries: map,
            entry_sources: sources,
        }
    }

    /// Load from a binary cache file, or build from scratch if the cache
    /// is missing or stale.
    pub fn load_or_build(
        cache_path: impl AsRef<Path>,
        lists_dir: impl AsRef<Path>,
    ) -> Result<Self, crate::error::Error> {
        if Self::cache_valid(&cache_path, &lists_dir) {
            match Self::load_cache(&cache_path) {
                Ok(db) => {
                    debug!(
                        "oma packages database cache hit: {}",
                        cache_path.as_ref().display()
                    );
                    return Ok(db);
                }
                Err(e) => debug!("oma packages database cache invalid, rebuilding: {e}"),
            }
        }

        debug!(
            "oma packages database cache miss: {}",
            cache_path.as_ref().display()
        );

        let (entries, sources) = parse_apt_lists_dir_with_sources(lists_dir)?;
        let db = Self::from_entries_with_sources(entries, sources);

        if let Err(e) = db.save_cache(&cache_path) {
            debug!("Failed to save oma packages database cache: {e}");
        } else {
            debug!(
                "oma packages database cache saved: {}",
                cache_path.as_ref().display()
            );
        }

        Ok(db)
    }

    /// Try to load from a saved cache file.
    pub(crate) fn load_cache(path: impl AsRef<Path>) -> io::Result<Self> {
        let mut buf = Vec::new();
        fs::File::open(path.as_ref()).and_then(|mut f| f.read_to_end(&mut buf))?;

        wincode::deserialize(&buf)
            .map_err(|e| std::io::Error::other(format!("Failed to decode cache: {e}")))
    }

    /// Save to a binary cache file.
    pub(crate) fn save_cache(&self, path: impl AsRef<Path>) -> std::io::Result<()> {
        if let Some(parent) = path.as_ref().parent() {
            fs::create_dir_all(parent)?;
        }

        let encoded = wincode::serialize(&self).map_err(std::io::Error::other)?;

        let mut file = fs::File::create(path.as_ref())?;
        file.write_all(&encoded)?;

        Ok(())
    }

    /// Check whether the cache is still valid by comparing mtimes with source files.
    pub(crate) fn cache_valid(cache_path: impl AsRef<Path>, lists_dir: impl AsRef<Path>) -> bool {
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

    /// Check if a package name exists in the database.
    pub fn has_package(&self, name: &str) -> bool {
        self.entries.contains_key(name)
    }

    /// Get the candidate entry for a package name (highest version).
    pub fn get_candidate(&self, name: &str) -> Option<&PackageEntry> {
        let entries = self.entries.get(name)?;
        entries.iter().max_by(|a, b| {
            let a_ver = a
                .version
                .as_deref()
                .and_then(|v| v.parse::<debversion::Version>().ok());

            let b_ver = b
                .version
                .as_deref()
                .and_then(|v| v.parse::<debversion::Version>().ok());

            a_ver.cmp(&b_ver)
        })
    }

    /// Get a specific version entry for a package name.
    /// Get all entries matching a package name and version (one per source).
    pub fn get(&self, name: &str, version: &str) -> Vec<&PackageEntry> {
        self.entries
            .get(name)
            .into_iter()
            .flatten()
            .filter(|e| e.version.as_deref().is_some_and(|v| v == version))
            .collect()
    }

    /// Iterate over all package entries (across all names).
    pub fn entries(&self) -> impl Iterator<Item = &PackageEntry> {
        self.entries.values().flatten()
    }

    /// Find all entries matching a package name.
    pub fn get_all(&self, name: &str) -> Vec<&PackageEntry> {
        self.entries.get(name).into_iter().flatten().collect()
    }

    /// Find all entries matching a package name, together with their source info.
    pub fn get_all_with_source(&self, name: &str) -> Vec<EntryWithSource<'_>> {
        let entries = match self.entries.get(name) {
            Some(v) => v,
            None => return vec![],
        };

        let sources = self.entry_sources.get(name);

        entries
            .iter()
            .enumerate()
            .map(|(i, entry)| EntryWithSource {
                entry: Cow::Borrowed(entry),
                source: sources
                    .and_then(|s| s.get(i))
                    .filter(|s| !s.is_empty())
                    .map(|s| Cow::Borrowed(s.as_str())),
            })
            .collect()
    }
}

impl PackageIndex for AptDb {
    fn has_package(&self, name: &str) -> bool {
        self.entries.contains_key(name)
    }

    fn packages(&self) -> Box<dyn Iterator<Item = &str> + '_> {
        Box::new(self.entries.keys().map(|s| s.as_str()))
    }

    fn get_all(&self, name: &str) -> Cow<'_, [PackageEntry]> {
        match self.entries.get(name) {
            Some(v) => Cow::Borrowed(v.as_slice()),
            None => Cow::Owned(Vec::new()),
        }
    }

    fn get_candidate(&self, name: &str) -> Option<Cow<'_, PackageEntry>> {
        self.entries
            .get(name)?
            .iter()
            .max_by(|a, b| {
                let a_ver = a
                    .version
                    .as_deref()
                    .and_then(|v| v.parse::<debversion::Version>().ok());
                let b_ver = b
                    .version
                    .as_deref()
                    .and_then(|v| v.parse::<debversion::Version>().ok());
                a_ver.cmp(&b_ver)
            })
            .map(Cow::Borrowed)
    }

    fn get_with_source(&self, name: &str) -> EntriesWithSource<'_> {
        let Some(entries) = self.entries.get(name) else {
            return Box::new(std::iter::empty());
        };
        let sources = self.entry_sources.get(name);
        Box::new(entries.iter().enumerate().map(move |(i, e)| {
            let src = sources.and_then(|s| s.get(i)).cloned().unwrap_or_default();
            (Cow::Borrowed(e), src)
        }))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn entry(name: &str, version: &str) -> PackageEntry {
        PackageEntry {
            package: name.to_string(),
            version: Some(version.to_string()),
            ..PackageEntry {
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

    #[test]
    fn test_insert_local_package() {
        let mut db = AptDb::from_entries(Vec::new());
        db.insert(entry("localpkg", "1.0"));

        assert!(db.has_package("localpkg"));
        let all = db.get_all("localpkg");
        assert_eq!(all.len(), 1);
        assert_eq!(all[0].version.as_deref(), Some("1.0"));

        // Local packages have no APT list source.
        let with_src = db.get_all_with_source("localpkg");
        assert_eq!(with_src.len(), 1);
        assert!(with_src[0].source.is_none());
    }

    #[test]
    fn test_insert_appends_existing_package() {
        let mut db = AptDb::from_entries(vec![entry("localpkg", "1.0")]);
        db.insert(entry("localpkg", "2.0"));

        let all = db.get_all("localpkg");
        assert_eq!(all.len(), 2);
    }

    #[test]
    fn test_resolve_queries_db() {
        let mut db = AptDb::from_entries(vec![
            entry("fish", "3.6"),
            entry("fish", "3.7"),
            entry("apt", "2.5"),
        ]);

        let resolution = db
            .resolve_queries(vec!["fish".into(), "nosuchpkg".into()])
            .unwrap();

        assert_eq!(resolution.groups.len(), 1);
        let entries = &resolution.groups[0];
        assert_eq!(entries.len(), 2);
        assert!(entries.iter().all(|e| e.entry.package == "fish"));
        assert_eq!(resolution.no_match, vec!["nosuchpkg"]);
    }

    #[test]
    fn test_resolve_queries_local_deb() {
        use crate::deb::test_util::{CONTROL, build_deb};

        let dir = tempfile::tempdir().unwrap();
        let deb_path = dir.path().join("hello_2.10-2_amd64.deb");
        std::fs::write(&deb_path, build_deb(CONTROL)).unwrap();

        let mut db = AptDb::from_entries(Vec::new());
        let resolution = db
            .resolve_queries(vec![deb_path.to_string_lossy().into_owned()])
            .unwrap();

        assert!(resolution.no_match.is_empty());
        assert_eq!(resolution.groups.len(), 1);
        let entries = &resolution.groups[0];
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].entry.package, "hello");
        assert_eq!(entries[0].entry.version.as_deref(), Some("2.10-2"));

        // The local `.deb` carries a `file:` URI source.
        let source = entries[0].source.as_deref().unwrap();
        assert!(source.starts_with("file:"));
    }

    #[test]
    fn test_resolve_queries_local_deb_merges_with_db() {
        use crate::deb::test_util::{CONTROL, build_deb};

        let dir = tempfile::tempdir().unwrap();
        let deb_path = dir.path().join("hello_2.10-2_amd64.deb");
        std::fs::write(&deb_path, build_deb(CONTROL)).unwrap();

        let mut db = AptDb::from_entries(vec![entry("hello", "2.10-2")]);
        let resolution = db
            .resolve_queries(vec![deb_path.to_string_lossy().into_owned()])
            .unwrap();

        assert_eq!(resolution.groups.len(), 1);
        let entries = &resolution.groups[0];
        // repo entry + merged local entry
        assert_eq!(entries.len(), 2);
        assert!(entries[0].source.is_none());
        assert!(entries[1].source.as_deref().unwrap().starts_with("file:"));
    }
}
