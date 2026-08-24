//! oma package database — Parse APT `Packages` files with a zero-copy,
//! memory-mapped binary cache (mirroring apt's `pkgcache.bin` model: the
//! cache file *is* the memory layout, and lookups touch only the pages they
//! need).

use std::borrow::Cow;
use std::collections::HashMap;
use std::fmt;
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};

use memmap2::{Mmap, MmapOptions};
use rayon::prelude::*;
use rkyv::vec::ArchivedVec;
use spdlog::debug;

use crate::AptConfig;
use crate::apt_lists::{
    ArchivedPackageVersion, IndexSource, PackageEntry, PackageVersion,
    parse_apt_lists_dir_with_sources,
};
use crate::apt_sources::SourceLookup;
use crate::cache;
use crate::cache::CacheFile;
use crate::package::Package;
use crate::package_matcher::PackageMatcher;

/// Magic for the package-database cache file (`Dir::Cache::oma-aptdb`);
/// the rest of the header layout (version + reserved bytes) is shared via
/// [`crate::cache`].
const CACHE_MAGIC: &[u8; 8] = b"OMADB\x00\x00\x00";

/// The on-disk payload of the [`AptDb`] cache, archived with rkyv. The file
/// layout is a small header followed by this archive, which is
/// memory-mapped at load time so a single-package query never deserializes
/// the whole database.
#[derive(Debug, Clone, rkyv::Archive, rkyv::Serialize, rkyv::Deserialize)]
pub(crate) struct AptDbData {
    /// Map from package name to its versions, each carrying every source it
    /// is available from — a version seen in several mirrors is stored once.
    pub(crate) entries: HashMap<String, Vec<PackageVersion>>,
    /// Native architecture (`APT::Architecture`), used by [`AptDb::fullname`]
    /// to omit the `:arch` qualifier in the pretty form. Extracted from the
    /// config at build time and stored with the cache.
    pub(crate) native_arch: String,
    /// The lists files this database was built from (filename + size +
    /// mtime), mirroring apt's PackageFile IMS records. Checked by
    /// [`crate::cache::valid`] on cache load.
    pub(crate) files: Vec<CacheFile>,
}

/// A validated, memory-mapped view of the [`AptDbData`] archive.
///
/// Created by [`Self::open`], which maps the cache file and runs a full
/// rkyv validation pass once. Afterwards [`Self::archived`] hands out
/// `&ArchivedAptDbData` with zero-copy unchecked access — sound because the
/// mapping is read-only and was validated at open time. This is the same
/// trust model as apt's `pkgcache.bin`: the cache file is only as safe as
/// its writer, and its validity is re-checked via the recorded lists files
/// before the database is used.
pub(crate) struct ArchivedAptDb {
    mmap: Mmap,
}

impl ArchivedAptDb {
    /// Map `path` and validate the header + archive. `Err` is returned for
    /// a missing/unreadable file, a foreign format, or a corrupt archive —
    /// the caller treats every case as a miss and rebuilds, but logs the
    /// reason.
    pub(crate) fn open(path: impl AsRef<Path>) -> std::io::Result<Self> {
        let file = fs::File::open(path.as_ref())?;
        // SAFETY: the cache file is only ever written atomically (temp file
        // + rename) and we map it read-only; the MAP_PRIVATE copy-on-write
        // mapping stays valid even if the file is replaced or truncated
        // concurrently, and pages are shared from the page cache.
        let mmap = unsafe { MmapOptions::new().map_copy_read_only(&file) }?;

        if !cache::header_ok(&mmap, CACHE_MAGIC) {
            return Err(std::io::Error::other(
                "unrecognized cache format or version",
            ));
        }

        // One full validation pass; afterwards the unchecked accessor below
        // is sound.
        rkyv::access::<ArchivedAptDbData, rkyv::rancor::Error>(&mmap[cache::CACHE_HEADER_LEN..])
            .map_err(|e| std::io::Error::other(format!("cache archive failed validation: {e}")))?;

        Ok(Self { mmap })
    }

    /// The validated archive.
    ///
    /// # Safety
    ///
    /// Safe because [`Self::open`] validated the whole archive and `self`
    /// holds the mapping read-only for the lifetime of the borrow.
    pub(crate) fn archived(&self) -> &ArchivedAptDbData {
        // SAFETY: the archive was fully validated in `open` and the mapping
        // is immutable for `self`'s lifetime.
        unsafe {
            rkyv::access_unchecked::<ArchivedAptDbData>(&self.mmap[cache::CACHE_HEADER_LEN..])
        }
    }
}

impl fmt::Debug for ArchivedAptDb {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ArchivedAptDb")
            .field("len", &self.mmap.len())
            .finish()
    }
}

/// Write `data` as an rkyv archive behind the cache header, atomically (temp
/// file + rename) so a crash mid-write never leaves a half-written cache
/// that would later fail validation and force a rebuild.
fn save_aptdb(path: impl AsRef<Path>, data: &AptDbData) -> std::io::Result<()> {
    let path = path.as_ref();
    let archive = rkyv::to_bytes::<rkyv::rancor::Error>(data).map_err(std::io::Error::other)?;

    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }

    let mut buf = Vec::with_capacity(cache::CACHE_HEADER_LEN + archive.len());
    cache::push_header(&mut buf, CACHE_MAGIC);
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

/// Deserialize every version of one archived package into owned
/// [`PackageVersion`]s. Only called on an archive validated by
/// [`ArchivedAptDb::open`], so the per-version deserialization cannot fail.
fn deserialize_versions(versions: &ArchivedVec<ArchivedPackageVersion>) -> Vec<PackageVersion> {
    versions
        .iter()
        .map(cache::from_archived::<PackageVersion>)
        .collect()
}

/// Errors that can occur when resolving package queries.
#[derive(Debug, thiserror::Error)]
pub enum QueryError {
    #[error("Failed to parse .deb: {0}")]
    Deb(#[from] crate::deb::DebError),
    #[error(transparent)]
    Matcher(#[from] crate::package_matcher::MatcherError),
}

/// One resolved query: the [`Package`] view of the matched package plus its
/// version/source filtered versions.
#[derive(Debug)]
pub struct ResolvedPackage<'a> {
    /// The package view, providing package-level info (name, version
    /// count, installed state) without re-borrowing the database.
    pub pkg: Package<'a>,
    /// The (version/source filtered) versions for this query — all
    /// versions of the package, a single version (`pkg=1.2.3`), one branch
    /// (`pkg/suite`) or a local `.deb`'s own version. Versions borrowed
    /// from the database when no filtering needed them owned.
    pub versions: Vec<Cow<'a, PackageVersion>>,
}

/// Result of resolving package queries.
#[derive(Debug)]
pub struct QueryResolution<'a> {
    /// Resolved queries in query order.
    pub resolved: Vec<ResolvedPackage<'a>>,
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

/// Where the package data lives.
#[derive(Debug)]
enum Repo {
    /// Live in-memory data — freshly built databases and in-memory builders
    /// (tests). Mutations go straight into the map.
    Owned(HashMap<String, Vec<PackageVersion>>),
    /// Zero-copy, read-only view over the memory-mapped cache file. Point
    /// lookups deserialize only the queried package's versions; mutations
    /// (local `.deb` inserts) land in [`AptDb::overlay`].
    Archived(ArchivedAptDb),
}

/// Parse and cache APT package database.
///
/// The data is either owned in memory or — on the common cache-hit path —
/// memory-mapped from the rkyv cache file (see [`AptDbData`] and
/// [`ArchivedAptDb`]), so a single-package query like `oma show apt` touches
/// only the mapped pages of that package instead of deserializing the whole
/// database. Search/TUI consumers that iterate everything pay one
/// deserialize per entry — the same total work as the old eager load.
#[derive(Debug)]
pub struct AptDb {
    /// The repository data: owned or memory-mapped.
    repo: Repo,
    /// Packages inserted at runtime on top of a memory-mapped repo (local
    /// `.deb` queries). Empty when the repo is owned, where inserts merge
    /// directly. A name present here *shadows* the repo entry: its repo
    /// versions were copied in on first insert, so merges behave exactly
    /// like the eager database.
    overlay: HashMap<String, Vec<PackageVersion>>,
    /// Native architecture (`APT::Architecture`), used by [`Self::fullname`]
    /// to omit the `:arch` qualifier in the pretty form. Extracted from the
    /// config at build time and stored with the cache.
    pub(crate) native_arch: String,
}

/// Build the package-name → versions map from parsed entries and their
/// per-entry sources, merging same-version entries into one
/// [`PackageVersion`] whose source list grows.
fn build_map(
    entries: Vec<PackageEntry>,
    entry_sources: Vec<IndexSource>,
) -> HashMap<String, Vec<PackageVersion>> {
    let mut map = HashMap::new();
    for (e, src) in entries.into_iter().zip(entry_sources) {
        let pkg = e.package.clone();
        push_or_merge(map.entry(pkg).or_default(), e, Some(src));
    }
    map
}

/// Push `entry` into `versions`, merging it into the existing entry of the
/// same version (adding `source` to its source list) so a version shared by
/// several sources is stored once.
fn push_or_merge(
    versions: &mut Vec<PackageVersion>,
    entry: PackageEntry,
    source: Option<IndexSource>,
) {
    let version = entry.version.clone();
    if let Some(existing) = versions.iter_mut().find(|v| v.entry.version == version) {
        if let Some(source) = source
            && !existing.sources.contains(&source)
        {
            existing.sources.push(source);
        }
    } else {
        versions.push(PackageVersion {
            entry,
            sources: source.into_iter().collect(),
        });
    }
}

impl AptDb {
    /// All versions of `name` across the repo and any overlay entry: a
    /// borrow when the data is owned and unshadowed, an owned (deserialized)
    /// vec when it comes from the memory map or the overlay. Shared with the
    /// package matcher, which uses it as its versions accessor.
    pub(crate) fn versions(&self, name: &str) -> Cow<'_, [PackageVersion]> {
        if let Some(overlay) = self.overlay.get(name) {
            return Cow::Borrowed(overlay.as_slice());
        }
        match &self.repo {
            Repo::Owned(map) => map
                .get(name)
                .map_or(Cow::Borrowed(&[]), |v| Cow::Borrowed(v.as_slice())),
            Repo::Archived(archived) => archived
                .archived()
                .entries
                .get(name)
                .map_or(Cow::Borrowed(&[]), |versions| {
                    Cow::Owned(deserialize_versions(versions))
                }),
        }
    }

    /// Number of versions of `name` without materializing them — the
    /// memory-mapped path reads the rkyv vector length directly instead of
    /// deserializing every version just to count.
    pub(crate) fn version_count(&self, name: &str) -> usize {
        if let Some(overlay) = self.overlay.get(name) {
            return overlay.len();
        }
        match &self.repo {
            Repo::Owned(map) => map.get(name).map_or(0, |v| v.len()),
            Repo::Archived(archived) => {
                archived.archived().entries.get(name).map_or(0, |v| v.len())
            }
        }
    }

    /// The mutable version list for `name`, copying the repo's versions in
    /// when the repo is memory-mapped (copy-on-write: overlay entries shadow
    /// the repo so merges keep the eager semantics).
    fn mutate_versions(&mut self, name: &str) -> &mut Vec<PackageVersion> {
        if !self.overlay.contains_key(name) {
            match &mut self.repo {
                Repo::Owned(map) => return map.entry(name.to_string()).or_default(),
                Repo::Archived(archived) => {
                    let existing = archived
                        .archived()
                        .entries
                        .get(name)
                        .map(deserialize_versions);
                    self.overlay
                        .insert(name.to_string(), existing.unwrap_or_default());
                }
            }
        }
        self.overlay.get_mut(name).expect("inserted above")
    }

    /// Build from entries without source tracking: every version is stored
    /// with no sources. Used by tests and in-memory builders.
    #[allow(dead_code)]
    pub(crate) fn from_entries(native_arch: &str, entries: Vec<PackageEntry>) -> Self {
        let mut map: HashMap<String, Vec<PackageVersion>> = HashMap::new();
        for e in entries {
            let name = e.package.clone();
            let versions = map.entry(name).or_default();
            push_or_merge(versions, e, None);
        }
        Self {
            repo: Repo::Owned(map),
            overlay: HashMap::new(),
            native_arch: native_arch.to_string(),
        }
    }

    /// Insert a package entry without a recorded source.
    ///
    /// Entries inserted this way have an empty source list — for
    /// programmatic builds where no source is known. Local `.deb`s should
    /// instead go through [`insert_from_deb`](Self::insert_from_deb), which
    /// records their `file:` source so the entry renders with an
    /// `APT-Sources` entry.
    pub fn insert(&mut self, entry: PackageEntry) {
        let name = entry.package.clone();
        push_or_merge(self.mutate_versions(&name), entry, None);
    }

    /// Insert a package entry together with its source, merging into the
    /// version it matches: the same (package, version) seen from several
    /// sources stays one version whose source list grows.
    pub fn insert_with_source(&mut self, entry: PackageEntry, source: IndexSource) {
        let name = entry.package.clone();
        push_or_merge(self.mutate_versions(&name), entry, Some(source));
    }

    /// Parse a local `.deb` file and insert its control entry into the
    /// database as a local package, recording its `file:` source — the
    /// canonical way to add a local `.deb` so its source survives into
    /// `APT-Sources`. Returns the package name.
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
    /// source, and resolved against their own `(name, version)` directly —
    /// merged with any repo entries of that version — consistent with
    /// `pkg=1.2.3` / `pkg/suite` queries.
    ///
    /// Note: this inserts the local packages into the database for the
    /// lifetime of this instance; the caller owns the database, so that is
    /// harmless per process.
    pub fn resolve_queries<'a>(
        &'a mut self,
        queries: Vec<String>,
    ) -> Result<QueryResolution<'a>, QueryError> {
        let (deb_files, names): (Vec<String>, Vec<String>) = queries
            .into_iter()
            .partition(|q| q.ends_with(".deb") && Path::new(q).is_file());

        let deb_entries: Vec<PackageEntry> = deb_files
            .par_iter()
            .map(crate::deb::parse_deb)
            .collect::<Result<_, _>>()?;

        // Insert every local `.deb` with its `file:` source, remembering its
        // (name, version) so it can be resolved once the inserts are done
        // (the matcher borrows the database).
        let mut deb_versions: Vec<(String, Option<String>)> = Vec::with_capacity(deb_files.len());
        for (path, entry) in deb_files.iter().zip(deb_entries) {
            let source = local_deb_source(path);
            let name = entry.package.clone();
            let version = entry.version.clone();
            deb_versions.push((name.clone(), version));
            self.insert_with_source(entry, source);
        }

        let matcher = PackageMatcher::new(self);
        let mut no_match = Vec::new();
        let mut resolved = Vec::new();

        // Resolve each `.deb` against its own version directly — the
        // control file's `(name, version)` go straight into
        // `match_from_version` instead of being formatted into a
        // `name=version` pattern and re-parsed. A `.deb` without a
        // `Version` resolves to all versions; either way it resolves to
        // exactly one group.
        for (name, version) in deb_versions {
            let versions = match version {
                Some(v) => matcher.match_from_version(&name, &v)?,
                None => matcher.match_pkgs_and_versions_from_glob(&name)?,
            };
            resolved.push(ResolvedPackage {
                pkg: Package::new(self, name.clone()),
                versions: versions
                    .into_iter()
                    .next()
                    .expect("a `.deb` resolves to exactly one group"),
            });
        }

        // Resolve the remaining name/glob/version/branch expressions.
        if !names.is_empty() {
            let (matched, no_result) =
                matcher.match_pkgs_and_versions(names.iter().map(String::as_str))?;

            for versions in matched {
                // Groups are never empty (the matcher drops empty matches),
                // so the first version carries the package name.
                let name = versions
                    .first()
                    .expect("groups are never empty")
                    .entry
                    .package
                    .clone();
                resolved.push(ResolvedPackage {
                    pkg: Package::new(self, name),
                    versions,
                });
            }

            no_match = no_result.into_iter().map(str::to_owned).collect();
        }

        Ok(QueryResolution { resolved, no_match })
    }

    /// Build from entries with parallel source tracking. Test-only: the
    /// cache path builds its own map via [`build_map`].
    #[cfg(test)]
    pub(crate) fn from_entries_with_sources(
        native_arch: &str,
        entries: Vec<PackageEntry>,
        entry_sources: Vec<IndexSource>,
    ) -> Self {
        Self {
            repo: Repo::Owned(build_map(entries, entry_sources)),
            overlay: HashMap::new(),
            native_arch: native_arch.to_string(),
        }
    }

    /// Load from the memory-mapped cache, or build from scratch if the
    /// cache is missing, foreign or stale.
    ///
    /// `apt_cfg` supplies everything: the lists directory
    /// (`Dir::State::lists`), the cache path (`Dir::Cache::oma-aptdb`) and
    /// the `sources.list`-derived [`SourceLookup`] that drives which lists
    /// files are read. The native architecture (`APT::Architecture`) is
    /// extracted here for [`Self::fullname`].
    ///
    /// On a cache hit the database is zero-copy: the file is memory-mapped
    /// and only the queried package's pages are read, so a single-package
    /// `oma show` never deserializes the whole database.
    pub fn load_or_build(apt_cfg: &AptConfig) -> Result<Self, crate::error::Error> {
        let lists_dir = apt_cfg.get_dir("Dir::State::lists", "var/lib/apt/lists");
        let cache_path =
            apt_cfg.get_file("Dir::Cache::oma-aptdb", "var/cache/apt/oma-aptdb.bincode");
        let native_arch = apt_cfg.get("APT::Architecture", "");
        let lookup = SourceLookup::build(apt_cfg);
        let archs = apt_cfg.architectures();

        // Try the on-disk cache: map and validate it, then check the lists
        // files it records having been built from against the current state
        // (mirroring apt's CheckValidity). An unusable cache — missing,
        // foreign, or corrupt — is logged and treated as a miss.
        let archived = match ArchivedAptDb::open(&cache_path) {
            Ok(archived) => Some(archived),
            Err(e) => {
                debug!("oma packages database cache unusable: {e}");
                None
            }
        };

        if let Some(archived) = archived {
            let files: Vec<CacheFile> = archived
                .archived()
                .files
                .as_slice()
                .iter()
                .map(cache::from_archived::<CacheFile>)
                .collect();

            if cache::valid(&cache_path, &lists_dir, &lookup, &archs, &files) {
                let native_arch = archived.archived().native_arch.to_string();
                debug!("oma packages database cache hit: {}", cache_path);
                return Ok(Self {
                    repo: Repo::Archived(archived),
                    overlay: HashMap::new(),
                    native_arch,
                });
            }
        }

        debug!("oma packages database cache miss: {}", cache_path);

        let (entries, sources) = parse_apt_lists_dir_with_sources(&lists_dir, &lookup, &archs)?;
        let files = cache::collect(&lists_dir, &lookup, &archs);
        let data = AptDbData {
            entries: build_map(entries, sources),
            native_arch: native_arch.to_string(),
            files,
        };

        if let Err(e) = save_aptdb(&cache_path, &data) {
            debug!("Failed to save oma packages database cache: {e}");
        } else {
            debug!("oma packages database cache saved: {}", cache_path);
        }

        Ok(Self {
            repo: Repo::Owned(data.entries),
            overlay: HashMap::new(),
            native_arch: data.native_arch,
        })
    }

    /// Check if a package name exists in the database.
    pub fn has_package(&self, name: &str) -> bool {
        if self.overlay.contains_key(name) {
            return true;
        }
        match &self.repo {
            Repo::Owned(map) => map.contains_key(name),
            Repo::Archived(archived) => archived.archived().entries.contains_key(name),
        }
    }

    /// The display full name of an entry, `name:arch`, using this database's
    /// native architecture (from `APT::Architecture` in the stored config)
    /// for the pretty form.
    ///
    /// See [`PackageEntry::fullname`].
    pub(crate) fn fullname<'a>(&self, entry: &'a PackageEntry, pretty: bool) -> Cow<'a, str> {
        entry.fullname(pretty, &self.native_arch)
    }

    /// Iterate over all package entries (across all names and versions).
    ///
    /// Borrowed from the in-memory map when the repo is owned, owned
    /// (deserialized on the fly) when it comes from the memory map — mapped
    /// data is an rkyv archive, not a [`PackageEntry`], so it must be
    /// materialized. Only the entry is deserialized, not the version's
    /// source list. Consumers that need everything — search indexes — pay
    /// one copy/deserialize per entry, the same total work as an eager
    /// load.
    pub fn entries(&self) -> impl Iterator<Item = Cow<'_, PackageEntry>> + '_ {
        let overlay = self
            .overlay
            .values()
            .flatten()
            .map(|v| Cow::Borrowed(&v.entry));

        let repo: Box<dyn Iterator<Item = Cow<'_, PackageEntry>> + '_> = match &self.repo {
            Repo::Owned(map) => Box::new(map.values().flatten().map(|v| Cow::Borrowed(&v.entry))),
            Repo::Archived(archived) => Box::new(
                archived
                    .archived()
                    .entries
                    .iter()
                    .flat_map(|(_, versions)| versions.iter())
                    .map(|v| Cow::Owned(cache::from_archived::<PackageEntry>(&v.entry))),
            ),
        };

        overlay.chain(repo)
    }

    /// The canonical `'a`-borrowed package name for `name`, if the database
    /// knows it — the overlay or repo map key. Lets [`Package`] handles
    /// borrow the map key instead of cloning a transient query string.
    fn canonical_name<'a>(&'a self, name: &str) -> Option<&'a str> {
        if let Some((key, _)) = self.overlay.get_key_value(name) {
            return Some(key.as_str());
        }
        match &self.repo {
            Repo::Owned(map) => map.get_key_value(name).map(|(k, _)| k.as_str()),
            // rkyv `ArchivedHashMap::get_key_value` is a hash lookup, like
            // the owned map — never scan the archived keys linearly (that
            // would make per-package lookups O(all packages)).
            Repo::Archived(archived) => archived
                .archived()
                .entries
                .get_key_value(name)
                .map(|(k, _)| k.as_str()),
        }
    }

    /// The [`Package`] view of `name`, or `None` if the database knows no
    /// such package. The name is borrowed from the database's map key, so
    /// no allocation is needed for a known package.
    pub fn package(&self, name: &str) -> Option<Package<'_>> {
        Some(Package::new(self, self.canonical_name(name)?))
    }

    /// Iterate every package in the database as a [`Package`] view — one
    /// item per package name (overlay names first, shadowing repo names,
    /// like [`Self::packages`]).
    pub fn packages_iter(&self) -> impl Iterator<Item = Package<'_>> + '_ {
        let overlay = self
            .overlay
            .keys()
            .map(|name| Package::new(self, name.as_str()));

        let repo: Box<dyn Iterator<Item = Package<'_>> + '_> = match &self.repo {
            Repo::Owned(map) => Box::new(map.keys().map(|name| Package::new(self, name.as_str()))),
            Repo::Archived(archived) => Box::new(
                archived
                    .archived()
                    .entries
                    .iter()
                    .map(|(name, _)| Package::new(self, name.as_str())),
            ),
        };

        overlay.chain(repo.filter(move |p| !self.overlay.contains_key(p.name())))
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
        let pkg = db.package("localpkg").unwrap();
        assert_eq!(pkg.version_count(), 1);
        assert_eq!(
            pkg.candidate().unwrap().entry.version.as_deref(),
            Some("1.0")
        );

        // Versions inserted without a source have an empty source list.
        let versions = db.versions("localpkg");
        assert_eq!(versions.len(), 1);
        assert!(versions[0].sources.is_empty());
    }

    #[test]
    fn test_insert_appends_existing_package() {
        let mut db = AptDb::from_entries("", vec![entry("localpkg", "1.0")]);
        db.insert(entry("localpkg", "2.0"));

        assert_eq!(db.package("localpkg").unwrap().version_count(), 2);
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

        assert_eq!(resolution.resolved.len(), 1);
        let versions = &resolution.resolved[0].versions;
        assert_eq!(versions.len(), 2);
        assert!(versions.iter().all(|v| v.entry.package == "fish"));
        // two distinct versions (3.6, 3.7)
        assert_eq!(resolution.resolved[0].pkg.version_count(), 2);
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
        assert_eq!(resolution.resolved.len(), 1);
        let versions = &resolution.resolved[0].versions;
        assert_eq!(versions.len(), 1);
        assert_eq!(versions[0].entry.package, "hello");
        assert_eq!(versions[0].entry.version.as_deref(), Some("2.10-2"));
        // only the local `.deb` version exists in the database
        assert_eq!(resolution.resolved[0].pkg.version_count(), 1);

        // The local `.deb` carries a `file:` URI source.
        assert_eq!(versions[0].sources.len(), 1);
        assert!(versions[0].sources[0].base_url.starts_with("file:"));
    }

    #[test]
    fn test_resolve_queries_local_deb_merges_with_db() {
        use crate::deb::test_util::{CONTROL, build_deb};

        let dir = tempfile::tempdir().unwrap();
        let deb_path = dir.path().join("hello_2.10-2_amd64.deb");
        std::fs::write(&deb_path, build_deb(CONTROL)).unwrap();

        let mut db = AptDb::from_entries_with_sources(
            "",
            vec![entry("hello", "2.10-2")],
            vec![IndexSource {
                base_url: "https://example.com/debs".to_string(),
                suite: "stable".to_string(),
                component: Some("main".to_string()),
                arch: Some("amd64".to_string()),
            }],
        );
        let resolution = db
            .resolve_queries(vec![deb_path.to_string_lossy().into_owned()])
            .unwrap();

        assert_eq!(resolution.resolved.len(), 1);
        let versions = &resolution.resolved[0].versions;
        // repo (same version) + local `.deb` merge into one version whose
        // source list carries both the `stable` and `file:` entries.
        assert_eq!(versions.len(), 1);
        assert_eq!(versions[0].sources.len(), 2);
        assert_eq!(versions[0].sources[0].suite, "stable");
        assert!(versions[0].sources[1].base_url.starts_with("file:"));
        // repo 2.10-2 + local 2.10-2 dedupe to one distinct version
        assert_eq!(resolution.resolved[0].pkg.version_count(), 1);
    }

    fn stable_source() -> IndexSource {
        IndexSource {
            base_url: "https://example.com/debs".to_string(),
            suite: "stable".to_string(),
            component: Some("main".to_string()),
            arch: Some("amd64".to_string()),
        }
    }

    /// Write an `AptDbData` to a temp cache file, map it back through the
    /// memory-mapped path and verify the zero-copy view matches the input.
    /// Also exercises `&str` lookups against the archived `HashMap`.
    #[test]
    fn test_cache_roundtrip() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("oma-aptdb.bincode");

        let data = AptDbData {
            entries: build_map(
                vec![
                    entry("fish", "3.6"),
                    entry("fish", "3.7"),
                    entry("apt", "2.5.4"),
                ],
                vec![stable_source(), stable_source(), stable_source()],
            ),
            native_arch: "amd64".to_string(),
            files: Vec::new(),
        };
        save_aptdb(&path, &data).unwrap();

        let archived = ArchivedAptDb::open(&path).expect("round-tripped cache opens");
        let view = archived.archived();

        assert_eq!(view.native_arch.as_str(), "amd64");
        assert_eq!(view.entries.len(), 2);
        assert!(view.entries.contains_key("apt"));

        // `&str` query against `ArchivedHashMap<ArchivedString, _>`.
        let fish = view.entries.get("fish").expect("fish present");
        assert_eq!(fish.len(), 2);
        assert_eq!(fish[0].entry.package.as_str(), "fish");
        assert_eq!(fish[0].entry.version.as_ref().unwrap().as_str(), "3.6");
        assert_eq!(fish[1].entry.version.as_ref().unwrap().as_str(), "3.7");
        assert_eq!(fish[0].sources.len(), 1);
        assert_eq!(fish[0].sources[0].suite.as_str(), "stable");

        // A foreign or corrupt file must be rejected, not crash.
        let garbage = dir.path().join("garbage");
        std::fs::write(&garbage, b"this is not a cache file").unwrap();
        assert!(ArchivedAptDb::open(&garbage).is_err());
        let truncated = dir.path().join("truncated");
        std::fs::write(&truncated, &[b'O', b'M', b'A', b'D', b'B']).unwrap();
        assert!(ArchivedAptDb::open(&truncated).is_err());
    }

    /// A memory-mapped repo: point lookups deserialize only the queried
    /// package, and inserting a local `.deb` copy-on-writes the repo's
    /// versions into the overlay and merges — same result as the eager db.
    #[test]
    fn test_archived_repo_overlay_merge() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("oma-aptdb.bincode");

        let data = AptDbData {
            entries: build_map(vec![entry("hello", "2.10-2")], vec![stable_source()]),
            native_arch: "amd64".to_string(),
            files: Vec::new(),
        };
        save_aptdb(&path, &data).unwrap();

        let mut db = AptDb {
            repo: Repo::Archived(ArchivedAptDb::open(&path).unwrap()),
            overlay: HashMap::new(),
            native_arch: "amd64".to_string(),
        };

        // Point lookup straight from the mapping.
        let all = db.versions("hello");
        assert_eq!(all.len(), 1);
        assert_eq!(all[0].entry.version.as_deref(), Some("2.10-2"));
        assert_eq!(all[0].sources.len(), 1);
        assert!(db.has_package("hello"));
        assert!(!db.has_package("nosuchpkg"));

        // Insert a local `.deb` of the same version: repo versions are
        // copied into the overlay (which shadows the repo) and merged, so
        // the version's source list carries both entries.
        db.insert_with_source(
            entry("hello", "2.10-2"),
            IndexSource {
                base_url: "file:/tmp/hello_2.10-2_amd64.deb".to_string(),
                suite: "local-deb".to_string(),
                component: Some("local-deb".to_string()),
                arch: None,
            },
        );

        let merged = db.versions("hello");
        assert_eq!(merged.len(), 1);
        assert_eq!(merged[0].sources.len(), 2);
        assert!(merged[0].sources.iter().any(|s| s.suite == "stable"));
        assert!(merged[0].sources.iter().any(|s| s.suite == "local-deb"));
        assert_eq!(db.package("hello").unwrap().version_count(), 1);
    }

    /// `resolve_queries` works against a memory-mapped repo, matching a
    /// query by deserializing just that package's versions.
    #[test]
    fn test_archived_repo_resolve_queries() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("oma-aptdb.bincode");

        let data = AptDbData {
            entries: build_map(
                vec![entry("fish", "3.6"), entry("fish", "3.7")],
                vec![stable_source(), stable_source()],
            ),
            native_arch: "amd64".to_string(),
            files: Vec::new(),
        };
        save_aptdb(&path, &data).unwrap();

        let mut db = AptDb {
            repo: Repo::Archived(ArchivedAptDb::open(&path).unwrap()),
            overlay: HashMap::new(),
            native_arch: "amd64".to_string(),
        };

        let resolution = db
            .resolve_queries(vec!["fish".into(), "nosuchpkg".into()])
            .unwrap();

        assert_eq!(resolution.resolved.len(), 1);
        let versions = &resolution.resolved[0].versions;
        assert_eq!(versions.len(), 2);
        assert!(versions.iter().all(|v| v.entry.package == "fish"));
        assert_eq!(resolution.resolved[0].pkg.version_count(), 2);
        assert_eq!(resolution.no_match, vec!["nosuchpkg"]);
    }

    /// The [`Package`] view works against a memory-mapped repo, exercising
    /// the owned-deserialize (rkyv) path of `package()`/`packages_iter()`
    /// and the `Package` accessors.
    #[test]
    fn test_package_on_archived_repo() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("oma-aptdb.bincode");

        let data = AptDbData {
            entries: build_map(
                vec![
                    entry("fish", "3.6"),
                    entry("fish", "3.7"),
                    entry("apt", "2.5.4"),
                ],
                vec![stable_source(), stable_source(), stable_source()],
            ),
            native_arch: "amd64".to_string(),
            files: Vec::new(),
        };
        save_aptdb(&path, &data).unwrap();

        let db = AptDb {
            repo: Repo::Archived(ArchivedAptDb::open(&path).unwrap()),
            overlay: HashMap::new(),
            native_arch: "amd64".to_string(),
        };

        let fish = db.package("fish").expect("fish present");
        assert_eq!(fish.name(), "fish");
        assert_eq!(fish.version_count(), 2);
        assert_eq!(
            fish.candidate().unwrap().entry.version.as_deref(),
            Some("3.7")
        );
        assert_eq!(
            fish.get_version("3.6").unwrap().entry.version.as_deref(),
            Some("3.6")
        );
        assert_eq!(fish.fullname(true), "fish");
        assert!(db.package("nosuchpkg").is_none());

        let mut names = db
            .packages_iter()
            .map(|p| p.name().to_string())
            .collect::<Vec<_>>();
        names.sort();
        assert_eq!(names, vec!["apt", "fish"]);
    }
}
