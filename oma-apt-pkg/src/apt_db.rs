//! oma package database — Parse APT `Packages` files with binary cache support.

use std::collections::HashMap;
use std::io::Read;
use std::io::Write;
use std::path::Path;
use std::{fs, io};

use spdlog::debug;
use wincode::{SchemaRead, SchemaWrite};

use crate::apt_lists::{PackageEntry, parse_apt_lists_dir_with_sources};

/// A package entry together with its source file information.
#[derive(Debug, Clone)]
pub struct EntryWithSource<'a> {
    /// The parsed package entry data.
    pub entry: &'a PackageEntry,
    /// The APT lists filename, e.g.
    /// `mirrors.example.com_debian_dists_bookworm_main_binary-amd64_Packages`.
    pub source: Option<&'a str>,
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
        let mut map = HashMap::new();
        for e in entries {
            map.entry(e.package.clone())
                .or_insert_with(Vec::new)
                .push(e);
        }
        Self {
            entry_sources: map.keys().map(|k| (k.clone(), Vec::new())).collect(),
            entries: map,
        }
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
            map.entry(pkg.clone()).or_default().push(e);
            sources.entry(pkg).or_default().push(src);
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
                .and_then(|v| debversion::Version::parse_lenient(v).ok());
            let b_ver = b
                .version
                .as_deref()
                .and_then(|v| debversion::Version::parse_lenient(v).ok());
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
                entry,
                source: sources.and_then(|s| s.get(i)).map(|s| s.as_str()),
            })
            .collect()
    }
}
