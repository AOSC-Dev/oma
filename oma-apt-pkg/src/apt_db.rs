//! oma package database — Parse APT `Packages` files with binary cache support.

use std::borrow::Cow;
use std::cell::OnceCell;
use std::collections::HashMap;
use std::io::Read;
use std::io::Write;
use std::path::Path;
use std::{fs, io};

use rayon::prelude::*;
use spdlog::debug;
use wincode::{SchemaRead, SchemaWrite};

use crate::apt_lists::{
    EntriesWithSource, IndexSource, PackageEntry, PackageIndex, PackageVersion,
    parse_apt_lists_dir_with_sources,
};
use crate::apt_sources::SourceLookup;
use crate::package_matcher::PackageMatcher;
use crate::{AptConfig, ParsedDeps};

/// A package entry together with its source file information.
///
/// The entry is borrowed from the database when it comes from an APT lists
/// file, or owned when it is a local `.deb` (whose source is the `file:` URL
/// recorded at insert time).
#[derive(Debug, Clone)]
pub struct EntryWithSource<'a> {
    /// The parsed package entry data.
    pub entry: Cow<'a, PackageEntry>,
    /// The source this entry came from (resolved against `sources.list` at
    /// database build time), or the `file:` source of a local `.deb`.
    /// `None` for entries without a recorded source.
    pub source: Option<Cow<'a, IndexSource>>,
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
    /// Number of distinct versions each group's package has across the whole
    /// database (a version shared by several sources counts once). Parallel
    /// to [`groups`](Self::groups), computed while the database is still
    /// accessible; the display layer uses it to report "N additional
    /// versions" even when the group itself is version-filtered (e.g. a
    /// local `.deb` query resolves to `pkg=<version>`).
    pub version_counts: Vec<usize>,
    /// Queries that matched no package.
    pub no_match: Vec<String>,
}

/// Build the `file:` URI source for a local `.deb` path, e.g.
/// `file:/home/oma/go_1.26.4%2btools0.45.0_amd64.deb`. The path is
/// percent-encoded with lowercase hex (e.g. `+` → `%2b`) to match APT's URI
/// form.
fn file_uri(path: impl AsRef<Path>) -> String {
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

/// The [`IndexSource`] for a local `.deb` — its `file:` URI with APT's
/// conventional `local-deb/local-deb` suite/component, so repository
/// consumers can format it the same way.
fn local_deb_source(path: impl AsRef<Path>) -> IndexSource {
    IndexSource {
        base_url: file_uri(path),
        suite: "local-deb".to_string(),
        component: Some("local-deb".to_string()),
        arch: None,
    }
}

/// Parse and cache APT package database.
#[derive(Debug, Clone, SchemaWrite, SchemaRead)]
pub struct AptDb {
    /// Map from package name to its versions, each carrying every source it
    /// is available from — a version seen in several mirrors is stored once.
    pub(crate) entries: HashMap<String, Vec<PackageVersion>>,
    /// Native architecture (`APT::Architecture`), used by [`Self::fullname`]
    /// to omit the `:arch` qualifier in the pretty form. Extracted from the
    /// config at build time and stored with the cache.
    pub(crate) native_arch: String,
}

/// Push `entry` into `versions`, merging it into the existing entry of the
/// same version (adding `source` to its source list) so a version shared by
/// several sources is stored once.
fn push_or_merge(versions: &mut Vec<PackageVersion>, entry: PackageEntry, source: IndexSource) {
    let version = entry.version.clone();
    if let Some(existing) = versions.iter_mut().find(|v| v.entry.version == version) {
        if !existing.sources.contains(&source) {
            existing.sources.push(source);
        }
    } else {
        versions.push(PackageVersion {
            entry,
            sources: vec![source],
            deps: OnceCell::new(),
            parsed_version: OnceCell::new(),
        });
    }
}

impl AptDb {
    /// Build from entries without source tracking.
    #[allow(dead_code)]
    pub(crate) fn from_entries(native_arch: &str, entries: Vec<PackageEntry>) -> Self {
        let mut map: HashMap<String, Vec<PackageVersion>> = HashMap::new();
        for e in entries {
            let name = e.package.clone();
            let versions = map.entry(name).or_default();
            push_or_merge(versions, e, IndexSource::none());
        }
        Self {
            entries: map,
            native_arch: native_arch.to_string(),
        }
    }

    /// Insert a local package entry (e.g. parsed from a local `.deb`) into
    /// the database.
    ///
    /// Local packages have no APT list source, so
    /// [`get_all_with_source`](Self::get_all_with_source) reports
    /// `source: None` for them.
    pub fn insert(&mut self, entry: PackageEntry) {
        self.insert_with_source(entry, IndexSource::none());
    }

    /// Insert a package entry together with its source, merging into the
    /// version it matches: the same (package, version) seen from several
    /// sources stays one version whose source list grows.
    pub fn insert_with_source(&mut self, entry: PackageEntry, source: IndexSource) {
        let name = entry.package.clone();
        let versions = self.entries.entry(name).or_default();
        push_or_merge(versions, entry, source);
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
        let mut version_counts = Vec::new();

        if !keywords.is_empty() {
            let matcher = PackageMatcher::new(self);
            let (matched, no_result) =
                matcher.match_pkgs_and_versions(keywords.iter().map(String::as_str))?;

            groups.extend(matched.into_iter().map(|pkg| {
                version_counts.push(self.distinct_version_count(&pkg.name));
                pkg.entries
                    .into_iter()
                    .map(|(entry, source)| EntryWithSource {
                        entry,
                        source: (!source.is_none()).then_some(Cow::Owned(source)),
                    })
                    .collect()
            }));
            no_match = no_result.into_iter().map(str::to_owned).collect();
        }

        Ok(QueryResolution {
            groups,
            version_counts,
            no_match,
        })
    }

    /// Build from entries with parallel source tracking.
    pub(crate) fn from_entries_with_sources(
        native_arch: &str,
        entries: Vec<PackageEntry>,
        entry_sources: Vec<IndexSource>,
    ) -> Self {
        let mut map: HashMap<String, Vec<PackageVersion>> = HashMap::new();
        for (e, src) in entries.into_iter().zip(entry_sources) {
            let pkg = e.package.clone();
            let versions = map.entry(pkg).or_default();
            push_or_merge(versions, e, src);
        }
        Self {
            entries: map,
            native_arch: native_arch.to_string(),
        }
    }

    /// Load from a binary cache file, or build from scratch if the cache
    /// is missing or stale.
    ///
    /// `apt_cfg` supplies everything: the lists directory
    /// (`Dir::State::lists`), the cache path (`Dir::Cache::oma-aptdb`) and
    /// the `sources.list`-derived [`SourceLookup`] that drives which lists
    /// files are read. The native architecture (`APT::Architecture`) is
    /// extracted here for [`Self::fullname`].
    pub fn load_or_build(apt_cfg: &AptConfig) -> Result<Self, crate::error::Error> {
        let lists_dir = apt_cfg.get_dir("Dir::State::lists", "var/lib/apt/lists");
        let cache_path =
            apt_cfg.get_file("Dir::Cache::oma-aptdb", "var/cache/apt/oma-aptdb.bincode");
        let native_arch = apt_cfg.get("APT::Architecture", "");
        let lookup = SourceLookup::build(apt_cfg);
        if Self::cache_valid(&cache_path, &lists_dir) {
            match Self::load_cache(&cache_path) {
                Ok(db) => {
                    debug!(
                        "oma packages database cache hit: {}",
                        Path::new(&cache_path).display()
                    );
                    return Ok(db);
                }
                Err(e) => debug!("oma packages database cache invalid, rebuilding: {e}"),
            }
        }

        debug!(
            "oma packages database cache miss: {}",
            Path::new(&cache_path).display()
        );

        let archs = apt_cfg.architectures();
        let (entries, sources) = parse_apt_lists_dir_with_sources(&lists_dir, &lookup, &archs)?;
        let db = Self::from_entries_with_sources(&native_arch, entries, sources);

        if let Err(e) = db.save_cache(&cache_path) {
            debug!("Failed to save oma packages database cache: {e}");
        } else {
            debug!(
                "oma packages database cache saved: {}",
                Path::new(&cache_path).display()
            );
        }

        Ok(db)
    }

    /// Try to load from a saved cache file.
    pub(crate) fn load_cache(path: impl AsRef<Path>) -> io::Result<Self> {
        let mut buf = Vec::new();
        fs::File::open(path.as_ref()).and_then(|mut f| f.read_to_end(&mut buf))?;

        let db: Self = wincode::deserialize(&buf)
            .map_err(|e| std::io::Error::other(format!("Failed to decode cache: {e}")))?;
        Ok(db)
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

    /// The display full name of an entry, `name:arch`, using this database's
    /// native architecture (from `APT::Architecture` in the stored config)
    /// for the pretty form.
    ///
    /// See [`PackageEntry::fullname`].
    pub fn fullname<'a>(&self, entry: &'a PackageEntry, pretty: bool) -> Cow<'a, str> {
        entry.fullname(pretty, &self.native_arch)
    }

    /// Get the candidate entry for a package name (highest version).
    pub fn get_candidate(&self, name: &str) -> Option<&PackageEntry> {
        self.entries
            .get(name)?
            .iter()
            .max_by_key(|v| v.parsed_version())
            .map(|v| &v.entry)
    }

    /// Get a specific version entry for a package name.
    pub fn get(&self, name: &str, version: &str) -> Vec<&PackageEntry> {
        self.entries
            .get(name)
            .into_iter()
            .flatten()
            .filter(|v| v.entry.version.as_deref() == Some(version))
            .map(|v| &v.entry)
            .collect()
    }

    /// Iterate over all package entries (across all names).
    pub fn entries(&self) -> impl Iterator<Item = &PackageEntry> {
        self.entries.values().flatten().map(|v| &v.entry)
    }

    /// Find all entries matching a package name.
    pub fn get_all(&self, name: &str) -> Vec<&PackageEntry> {
        self.entries
            .get(name)
            .into_iter()
            .flatten()
            .map(|v| &v.entry)
            .collect()
    }

    /// Find all versions of a package together with their sources: one item
    /// per (version, source), so a version seen in several mirrors shows up
    /// once per source.
    pub fn get_all_with_source(&self, name: &str) -> Vec<EntryWithSource<'_>> {
        let versions = match self.entries.get(name) {
            Some(v) => v,
            None => return vec![],
        };

        let mut out = Vec::new();
        for v in versions {
            for src in &v.sources {
                out.push(EntryWithSource {
                    entry: Cow::Borrowed(&v.entry),
                    source: (!src.is_none()).then_some(Cow::Borrowed(src)),
                });
            }
        }
        out
    }

    /// Number of distinct versions of a package across the whole database.
    /// Versions are already deduplicated (each [`PackageVersion`] is one
    /// version), so this is simply the entry count.
    pub(crate) fn distinct_version_count(&self, name: &str) -> usize {
        self.entries.get(name).map_or(0, Vec::len)
    }
}

impl PackageIndex for AptDb {
    fn has_package(&self, name: &str) -> bool {
        self.entries.contains_key(name)
    }

    fn packages(&self) -> Box<dyn Iterator<Item = &str> + '_> {
        Box::new(self.entries.keys().map(|s| s.as_str()))
    }

    fn get_all(&self, name: &str) -> Cow<'_, [PackageVersion]> {
        match self.entries.get(name) {
            Some(v) => Cow::Borrowed(v.as_slice()),
            None => Cow::Owned(Vec::new()),
        }
    }

    fn get_with_source(&self, name: &str) -> EntriesWithSource<'_> {
        let Some(versions) = self.entries.get(name) else {
            return Box::new(std::iter::empty());
        };
        // One item per (version, source): a version from several mirrors
        // shows up once per source, like the pre-dedup per-source rows.
        Box::new(versions.iter().flat_map(move |v| {
            v.sources
                .iter()
                .map(move |src| (Cow::Borrowed(&v.entry), src.clone()))
        }))
    }

    fn get_candidate(&self, name: &str) -> Option<Cow<'_, PackageVersion>> {
        self.entries
            .get(name)?
            .iter()
            .max_by_key(|v| v.parsed_version())
            .map(Cow::Borrowed)
    }

    fn get_version(&self, name: &str, version: &str) -> Option<Cow<'_, PackageVersion>> {
        self.entries
            .get(name)?
            .iter()
            .find(|v| v.entry.version.as_deref() == Some(version))
            .map(Cow::Borrowed)
    }

    fn deps_of(&self, name: &str, version: &str) -> Option<Cow<'_, ParsedDeps>> {
        let v = self
            .entries
            .get(name)?
            .iter()
            .find(|v| v.entry.version.as_deref() == Some(version))?;
        Some(Cow::Borrowed(v.deps()))
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
        let mut db = AptDb::from_entries("", Vec::new());
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
        let mut db = AptDb::from_entries("", vec![entry("localpkg", "1.0")]);
        db.insert(entry("localpkg", "2.0"));

        let all = db.get_all("localpkg");
        assert_eq!(all.len(), 2);
    }

    #[test]
    fn test_resolve_queries_db() {
        let mut db = AptDb::from_entries(
            "",
            vec![
                entry("fish", "3.6"),
                entry("fish", "3.7"),
                entry("apt", "2.5"),
            ],
        );

        let resolution = db
            .resolve_queries(vec!["fish".into(), "nosuchpkg".into()])
            .unwrap();

        assert_eq!(resolution.groups.len(), 1);
        let entries = &resolution.groups[0];
        assert_eq!(entries.len(), 2);
        assert!(entries.iter().all(|e| e.entry.package == "fish"));
        // two distinct versions (3.6, 3.7)
        assert_eq!(resolution.version_counts, vec![2]);
        assert_eq!(resolution.no_match, vec!["nosuchpkg"]);
    }

    #[test]
    fn test_resolve_queries_local_deb() {
        use crate::deb::test_util::{CONTROL, build_deb};

        let dir = tempfile::tempdir().unwrap();
        let deb_path = dir.path().join("hello_2.10-2_amd64.deb");
        std::fs::write(&deb_path, build_deb(CONTROL)).unwrap();

        let mut db = AptDb::from_entries("", Vec::new());
        let resolution = db
            .resolve_queries(vec![deb_path.to_string_lossy().into_owned()])
            .unwrap();

        assert!(resolution.no_match.is_empty());
        assert_eq!(resolution.groups.len(), 1);
        let entries = &resolution.groups[0];
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].entry.package, "hello");
        assert_eq!(entries[0].entry.version.as_deref(), Some("2.10-2"));
        // only the local `.deb` version exists in the database
        assert_eq!(resolution.version_counts, vec![1]);

        // The local `.deb` carries a `file:` URI source.
        let source = entries[0].source.as_deref().unwrap();
        assert!(source.base_url.starts_with("file:"));
    }

    #[test]
    fn test_resolve_queries_local_deb_merges_with_db() {
        use crate::deb::test_util::{CONTROL, build_deb};

        let dir = tempfile::tempdir().unwrap();
        let deb_path = dir.path().join("hello_2.10-2_amd64.deb");
        std::fs::write(&deb_path, build_deb(CONTROL)).unwrap();

        let mut db = AptDb::from_entries("", vec![entry("hello", "2.10-2")]);
        let resolution = db
            .resolve_queries(vec![deb_path.to_string_lossy().into_owned()])
            .unwrap();

        assert_eq!(resolution.groups.len(), 1);
        let entries = &resolution.groups[0];
        // repo entry + merged local entry
        assert_eq!(entries.len(), 2);
        assert!(entries[0].source.is_none());
        assert!(
            entries[1]
                .source
                .as_deref()
                .unwrap()
                .base_url
                .starts_with("file:")
        );
        // repo 2.10-2 + local 2.10-2 dedupe to one distinct version
        assert_eq!(resolution.version_counts, vec![1]);
    }

    #[test]
    fn test_distinct_version_count() {
        let db = AptDb::from_entries(
            "",
            vec![
                entry("fish", "3.6"),
                entry("fish", "3.7"),
                entry("fish", "3.7"), // same version from another source
                entry("apt", "2.5"),
            ],
        );

        assert_eq!(db.distinct_version_count("fish"), 2);
        assert_eq!(db.distinct_version_count("apt"), 1);
        assert_eq!(db.distinct_version_count("nosuchpkg"), 0);
    }
}
