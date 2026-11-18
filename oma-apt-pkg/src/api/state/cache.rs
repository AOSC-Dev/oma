//! The rkyv cache for the Packages half of [`AptState`](super::AptState).

use std::collections::HashMap;
use std::path::Path;

use super::super::arch::Arch;
use super::super::model::Version;
use crate::internal::cache::{CACHE_HEADER_LEN, CacheFile};

/// Magic for the package-database cache (`Dir::Cache::oma-aptdb`).
const STATE_CACHE_MAGIC: &[u8; 8] = b"OMAST\x00\x00\x00";

/// On-disk form of an [`AptState`]: the Packages data only. dpkg status is
/// live and merged after loading, like apt's status cache refresh.
#[derive(Debug, rkyv::Archive, rkyv::Serialize, rkyv::Deserialize)]
pub(super) struct AptStateData {
    pub(super) groups: HashMap<String, HashMap<Arch, Vec<Version>>>,
    pub(super) package_order: HashMap<String, Vec<Arch>>,
    /// Native architecture the groups were built for; `all` versions are
    /// attached to it and must not be reused under a different one.
    pub(super) native_arch: Arch,
    /// Configured architecture order, which `:any`/preferred selection uses.
    pub(super) architectures: Vec<Arch>,
    pub(super) files: Vec<CacheFile>,
}

/// A validated, read-only map of the [`AptStateData`] archive.
///
/// Mapping instead of reading copies nothing: the package groups stay in the
/// page cache and are deserialized one package at a time, when a query asks
/// for them — the same trust model as apt's `pkgcache.bin` and as the former
/// `ArchivedAptDb`. [`Self::open`] validates the whole archive once, so
/// afterwards [`Self::archived`] may hand out unchecked references.
pub(super) struct StateCache {
    mmap: memmap2::Mmap,
}

impl StateCache {
    /// Map `path` and validate the header and archive. `None` means a
    /// missing/unreadable file, a foreign format, or a corrupt archive; the
    /// caller then rebuilds.
    pub(super) fn open(path: &str) -> Option<Self> {
        let file = std::fs::File::open(path).ok()?;
        // SAFETY: the cache file is only ever written atomically (temporary
        // file + rename) and mapped read-only, so the copy-on-write mapping
        // stays valid even if the file is replaced or truncated.
        let mmap = unsafe { memmap2::MmapOptions::new().map_copy_read_only(&file).ok()? };

        if !crate::internal::cache::header_ok(&mmap, STATE_CACHE_MAGIC) {
            return None;
        }
        rkyv::access::<ArchivedAptStateData, rkyv::rancor::Error>(&mmap[CACHE_HEADER_LEN..])
            .ok()?;

        Some(Self { mmap })
    }

    /// The validated archive.
    pub(super) fn archived(&self) -> &ArchivedAptStateData {
        // SAFETY: `open` validated the whole archive and the mapping is
        // immutable for `self`'s lifetime.
        unsafe { rkyv::access_unchecked::<ArchivedAptStateData>(&self.mmap[CACHE_HEADER_LEN..]) }
    }

    /// The lists files the archive was built from.
    pub(super) fn files(&self) -> Vec<CacheFile> {
        self.archived()
            .files
            .iter()
            .map(|file| CacheFile {
                filename: file.filename.as_str().to_string(),
                size: file.size.into(),
                mtime: file.mtime.into(),
            })
            .collect()
    }
}

impl std::fmt::Debug for StateCache {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("StateCache")
            .field("bytes", &self.mmap.len())
            .finish()
    }
}

pub(super) fn save_state_cache(path: &str, data: &AptStateData) -> std::io::Result<()> {
    let archive = rkyv::to_bytes::<rkyv::rancor::Error>(data).map_err(std::io::Error::other)?;
    let mut buf = Vec::with_capacity(CACHE_HEADER_LEN + archive.len());
    crate::internal::cache::push_header(&mut buf, STATE_CACHE_MAGIC);
    buf.extend_from_slice(&archive);

    crate::internal::cache::write_atomic(Path::new(path), &buf)
}
