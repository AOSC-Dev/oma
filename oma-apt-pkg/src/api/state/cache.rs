//! The rkyv cache for the Packages half of [`AptState`](super::AptState).

use std::collections::HashMap;
use std::path::Path;

use super::super::arch::Arch;
use super::super::model::Version;
use crate::internal::cache::CacheFile;

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

pub(super) fn load_state_cache(path: &str) -> Option<AptStateData> {
    let bytes = std::fs::read(path).ok()?;
    if !crate::internal::cache::header_ok(&bytes, STATE_CACHE_MAGIC) {
        return None;
    }
    let archived = rkyv::access::<ArchivedAptStateData, rkyv::rancor::Error>(
        &bytes[crate::internal::cache::CACHE_HEADER_LEN..],
    )
    .ok()?;
    Some(crate::internal::cache::from_archived::<AptStateData>(
        archived,
    ))
}

pub(super) fn save_state_cache(path: &str, data: &AptStateData) -> std::io::Result<()> {
    let archive = rkyv::to_bytes::<rkyv::rancor::Error>(data).map_err(std::io::Error::other)?;
    let mut buf = Vec::with_capacity(crate::internal::cache::CACHE_HEADER_LEN + archive.len());
    crate::internal::cache::push_header(&mut buf, STATE_CACHE_MAGIC);
    buf.extend_from_slice(&archive);

    crate::internal::cache::write_atomic(Path::new(path), &buf)
}
