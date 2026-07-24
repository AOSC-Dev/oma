//! oma package database — Parse APT `Packages` files with binary cache support.

use std::io::Read;
use std::path::Path;
use std::{collections::HashSet, io::Write};
use std::{fs, io};

use spdlog::debug;
use wincode::{SchemaRead, SchemaWrite};

use crate::apt_lists::{PackageEntry, parse_apt_lists_dir};

/// Parse and cache APT package database.
///
/// Wraps all `PackageEntry` items from `*_Packages` files and can be
/// passed to / loaded from from a binary cache file
#[derive(Debug, Clone, SchemaWrite, SchemaRead)]
pub struct AptDb {
    pub(crate) entries: Vec<PackageEntry>,
    pub(crate) available_names: HashSet<String>,
}

impl AptDb {
    /// Build from a vector of entries (from parsing).
    pub(crate) fn from_entries(entries: Vec<PackageEntry>) -> Self {
        let available_names = entries.iter().map(|e| e.package.clone()).collect();
        Self {
            entries,
            available_names,
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
        let entries = parse_apt_lists_dir(lists_dir)?;
        let db = Self::from_entries(entries);

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

        let mut db: Self = wincode::deserialize(&buf)
            .map_err(|e| std::io::Error::other(format!("Failed to decode cache: {e}")))?;

        // Rebuild the transient field
        db.available_names = db.entries.iter().map(|e| e.package.clone()).collect();
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

    /// Check if a package name exists in the database
    pub fn has_package(&self, name: &str) -> bool {
        self.available_names.contains(name)
    }

    /// Get an entry by exact package name.
    pub fn get(&self, name: &str) -> Option<&PackageEntry> {
        self.entries.iter().find(|e| e.package == name)
    }
}
