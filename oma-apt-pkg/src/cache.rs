//! Shared helpers for the binary caches of [`AptDb`](crate::AptDb) and
//! [`IndiciumSearch`](crate::search::IndiciumSearch): a
//! [`SourceLookup`]-driven staleness check, the rkyv cache-file header, and
//! zero-copy deserialization.

use std::fs;
use std::path::Path;
use std::time::UNIX_EPOCH;

use crate::apt_sources::SourceLookup;

/// Header shared by the rkyv cache files: 8-byte magic, u32 little-endian
/// format version, 4 reserved bytes, then the rkyv archive. The payload
/// starts at a 16-byte offset so it is 8-byte aligned (mapping bases and
/// freshly allocated buffers are page/8-aligned).
pub(crate) const CACHE_HEADER_LEN: usize = 16;
pub(crate) const CACHE_VERSION: u32 = 1;

/// Whether `bytes` begins with a valid header for `magic` (magic + current
/// version).
pub(crate) fn header_ok(bytes: &[u8], magic: &[u8; 8]) -> bool {
    bytes.len() >= CACHE_HEADER_LEN
        && &bytes[..8] == magic
        && u32::from_le_bytes(bytes[8..12].try_into().expect("len checked")) == CACHE_VERSION
}

/// Append the cache header with `magic` to `buf`; callers then append the
/// archive.
pub(crate) fn push_header(buf: &mut Vec<u8>, magic: &[u8; 8]) {
    buf.extend_from_slice(magic);
    buf.extend_from_slice(&CACHE_VERSION.to_le_bytes());
    buf.extend_from_slice(&[0u8; 4]); // reserved
}

/// One lists file a cache was built from, mirroring apt's PackageFile IMS
/// record (name + size + mtime) so validity can be checked against what
/// was actually read.
#[derive(Debug, Clone, PartialEq, Eq, rkyv::Archive, rkyv::Serialize, rkyv::Deserialize)]
pub(crate) struct CacheFile {
    /// The lists filename, e.g.
    /// `mirrors.example.com_debian_dists_stable_main_binary-amd64_Packages`.
    pub(crate) filename: String,
    pub(crate) size: u64,
    /// Modification time in whole seconds since the Unix epoch (apt uses
    /// `time_t`).
    pub(crate) mtime: i64,
}

/// Record the lists files the [`SourceLookup`] produces that currently
/// exist on disk, with their size and mtime — the exact set a fresh build
/// would read. Stored in the cache at build time so [`valid`] can compare
/// against it later.
pub(crate) fn collect(
    lists_dir: impl AsRef<Path>,
    lookup: &SourceLookup,
    archs: &[String],
) -> Vec<CacheFile> {
    lookup
        .index_files(archs)
        .into_iter()
        .filter_map(|(filename, _)| {
            let meta = fs::metadata(lists_dir.as_ref().join(&filename)).ok()?;
            Some(CacheFile {
                filename,
                size: meta.len(),
                mtime: mtime_secs(&meta)?,
            })
        })
        .collect()
}

/// Whether `cache_path` is a valid cache for the current lists state,
/// mirroring apt's `CheckValidity`.
///
/// The cache must have been built from exactly the files the
/// [`SourceLookup`] produces that still exist on disk with unchanged size
/// and mtime:
///
/// * a lists file that changed or was deleted invalidates the cache;
/// * a source whose lists were never downloaded (`apt update` not run)
///   contributes nothing to either side — it neither invalidates the cache
///   nor requires a rebuild, exactly like apt's `Exists() == false` skip;
/// * removing a source from `sources.list` invalidates the cache (its
///   recorded files are no longer produced by the lookup), so the removed
///   source's packages drop out on the next build.
pub(crate) fn valid(
    cache_path: impl AsRef<Path>,
    lists_dir: impl AsRef<Path>,
    lookup: &SourceLookup,
    archs: &[String],
    files: &[CacheFile],
) -> bool {
    // The cache file itself must exist.
    if fs::metadata(&cache_path)
        .and_then(|m| m.modified())
        .is_err()
    {
        return false;
    }

    // Snapshot the lists files the current lookup produces and that exist
    // on disk.
    let current: Vec<(String, u64, i64)> = lookup
        .index_files(archs)
        .into_iter()
        .filter_map(|(filename, _)| {
            let meta = fs::metadata(lists_dir.as_ref().join(&filename)).ok()?;
            Some((filename, meta.len(), mtime_secs(&meta)?))
        })
        .collect();

    // The recorded set and the current set must match exactly.
    files.len() == current.len()
        && files.iter().all(|f| {
            current.iter().any(|(name, size, mtime)| {
                name == &f.filename && *size == f.size && *mtime == f.mtime
            })
        })
}

/// Deserialize one archived value into its owned form. Infallible in
/// practice: only ever called on archives validated with [`rkyv::access`],
/// so the per-value deserialization cannot fail.
///
/// The deserializer is a [`rancor::Strategy`] over `()` — the standard
/// collection `Deserialize` impls only require `D: Fallible`, and
/// allocating types (String/Vec/HashMap/IndexMap) allocate directly.
pub(crate) fn from_archived<T>(value: &T::Archived) -> T
where
    T: rkyv::Archive,
    T::Archived: rkyv::Deserialize<T, rkyv::rancor::Strategy<(), rkyv::rancor::Error>>,
{
    let mut unit = ();
    let deserializer = rkyv::rancor::Strategy::<(), rkyv::rancor::Error>::wrap(&mut unit);
    rkyv::Deserialize::deserialize(value, deserializer).expect("validated archive")
}

/// The file's modification time in whole seconds since the Unix epoch,
/// `None` if unavailable.
fn mtime_secs(meta: &fs::Metadata) -> Option<i64> {
    meta.modified()
        .ok()?
        .duration_since(UNIX_EPOCH)
        .ok()
        .map(|d| d.as_secs() as i64)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    const STABLE: &str = "Types: deb\n\
        URIs: https://example.com/debs\n\
        Suites: stable\n\
        Components: main\n\
        Architectures: amd64\n\
        Signed-By: /dev/null\n";
    const PREVIEW: &str = "Types: deb\n\
        URIs: https://example.com/debs\n\
        Suites: preview\n\
        Components: main\n\
        Architectures: amd64\n\
        Signed-By: /dev/null\n";

    fn archs() -> Vec<String> {
        vec!["amd64".to_string()]
    }

    /// Build a `SourceLookup` from the given deb822 `.sources` texts.
    fn lookup(texts: &[&str]) -> SourceLookup {
        let dir = tempfile::tempdir().unwrap();
        let paths: Vec<PathBuf> = texts
            .iter()
            .enumerate()
            .map(|(i, text)| {
                let list = dir.path().join(format!("source-{i}.sources"));
                fs::write(&list, text).unwrap();
                list
            })
            .collect();
        SourceLookup::from_paths(&paths, |_| {})
    }

    /// The lists filename a lookup generates for `suite`/`main`/`amd64`.
    fn lists_file(lookup: &SourceLookup, suite: &str) -> String {
        lookup
            .index_files(&archs())
            .into_iter()
            .find(|(_, src)| src.suite == suite && src.arch.as_deref() == Some("amd64"))
            .unwrap()
            .0
    }

    /// Write `content` to the lists file `suite`/`main`/`amd64` generates
    /// inside `dir`; returns its filename.
    fn write_packages(dir: &Path, lookup: &SourceLookup, suite: &str, content: &str) -> String {
        let filename = lists_file(lookup, suite);
        fs::write(dir.join(&filename), content).unwrap();
        filename
    }

    /// Create the cache file so [`valid`]'s existence check passes.
    fn touch_cache(path: &Path) {
        fs::write(path, b"cache").unwrap();
    }

    #[test]
    fn test_collect_records_existing_files_only() {
        let dir = tempfile::tempdir().unwrap();
        let lookup = lookup(&[STABLE, PREVIEW]);
        write_packages(dir.path(), &lookup, "stable", "a");
        // preview never downloaded

        let files = collect(dir.path(), &lookup, &archs());
        assert_eq!(files.len(), 1);
        assert_eq!(files[0].filename, lists_file(&lookup, "stable"));
        assert_eq!(files[0].size, 1);
        assert_eq!(
            files[0].mtime,
            mtime_secs(&fs::metadata(dir.path().join(&files[0].filename)).unwrap()).unwrap()
        );
    }

    #[test]
    fn test_valid_missing_cache_file() {
        let dir = tempfile::tempdir().unwrap();
        let lookup = lookup(&[STABLE]);
        write_packages(dir.path(), &lookup, "stable", "a");
        let files = collect(dir.path(), &lookup, &archs());

        // The cache file was never written.
        assert!(!valid(
            dir.path().join("cache"),
            dir.path(),
            &lookup,
            &archs(),
            &files
        ));
    }

    #[test]
    fn test_valid_unchanged() {
        let dir = tempfile::tempdir().unwrap();
        let lookup = lookup(&[STABLE]);
        write_packages(dir.path(), &lookup, "stable", "a");
        touch_cache(&dir.path().join("cache"));
        let files = collect(dir.path(), &lookup, &archs());

        assert!(valid(
            dir.path().join("cache"),
            dir.path(),
            &lookup,
            &archs(),
            &files
        ));
    }

    #[test]
    fn test_valid_recorded_file_deleted() {
        let dir = tempfile::tempdir().unwrap();
        let lookup = lookup(&[STABLE]);
        write_packages(dir.path(), &lookup, "stable", "a");
        touch_cache(&dir.path().join("cache"));
        let files = collect(dir.path(), &lookup, &archs());

        fs::remove_file(dir.path().join(lists_file(&lookup, "stable"))).unwrap();
        assert!(!valid(
            dir.path().join("cache"),
            dir.path(),
            &lookup,
            &archs(),
            &files
        ));
    }

    #[test]
    fn test_valid_recorded_file_changed() {
        let dir = tempfile::tempdir().unwrap();
        let lookup = lookup(&[STABLE]);
        write_packages(dir.path(), &lookup, "stable", "old");
        touch_cache(&dir.path().join("cache"));
        let files = collect(dir.path(), &lookup, &archs());

        // A different size guarantees the IMS check fails even if the
        // rewrite lands in the same second (mtime is second-granular).
        write_packages(dir.path(), &lookup, "stable", "new-content");
        assert!(!valid(
            dir.path().join("cache"),
            dir.path(),
            &lookup,
            &archs(),
            &files
        ));
    }

    #[test]
    fn test_valid_new_file_appeared() {
        let dir = tempfile::tempdir().unwrap();
        let lookup = lookup(&[STABLE, PREVIEW]);
        write_packages(dir.path(), &lookup, "stable", "a");
        touch_cache(&dir.path().join("cache"));
        // Cache was built while only `stable` was downloaded.
        let files = collect(dir.path(), &lookup, &archs());
        assert_eq!(files.len(), 1);

        // `preview` downloaded after the cache was built.
        write_packages(dir.path(), &lookup, "preview", "b");
        assert!(!valid(
            dir.path().join("cache"),
            dir.path(),
            &lookup,
            &archs(),
            &files
        ));
    }

    #[test]
    fn test_valid_never_downloaded_source() {
        let dir = tempfile::tempdir().unwrap();
        let lookup = lookup(&[STABLE, PREVIEW]);
        write_packages(dir.path(), &lookup, "stable", "a");
        touch_cache(&dir.path().join("cache"));
        let files = collect(dir.path(), &lookup, &archs());
        assert_eq!(files.len(), 1);

        // `preview` is still not downloaded: it contributes to neither side,
        // so the cache stays valid — apt's `Exists() == false` skip.
        assert!(valid(
            dir.path().join("cache"),
            dir.path(),
            &lookup,
            &archs(),
            &files
        ));
    }

    #[test]
    fn test_valid_source_removed() {
        let dir = tempfile::tempdir().unwrap();
        let lookup_both = lookup(&[STABLE, PREVIEW]);
        write_packages(dir.path(), &lookup_both, "stable", "a");
        write_packages(dir.path(), &lookup_both, "preview", "b");
        touch_cache(&dir.path().join("cache"));
        let files = collect(dir.path(), &lookup_both, &archs());
        assert_eq!(files.len(), 2);

        // sources.list edited to drop `preview`: its recorded file is no
        // longer produced by the lookup, so the cache is stale.
        let lookup_stable = lookup(&[STABLE]);
        assert!(!valid(
            dir.path().join("cache"),
            dir.path(),
            &lookup_stable,
            &archs(),
            &files
        ));
    }

    #[test]
    fn test_cache_header() {
        // `push_header` writes magic + version; `header_ok` accepts it and
        // rejects wrong magic or a truncated file.
        let mut buf = Vec::new();
        push_header(&mut buf, b"TESTMAG\x00");
        assert!(header_ok(&buf, b"TESTMAG\x00"));
        assert!(!header_ok(&buf, b"WRONGMG\x00"));
        assert!(!header_ok(&buf[..8], b"TESTMAG\x00"));
    }

    #[test]
    fn test_from_archived_roundtrip() {
        let value = CacheFile {
            filename: "example.com_debs_dists_stable_main_binary-amd64_Packages".to_string(),
            size: 42,
            mtime: 12345,
        };

        // rkyv-serialize, validate with `access`, deserialize back.
        let archive = rkyv::to_bytes::<rkyv::rancor::Error>(&value).unwrap();
        let archived = rkyv::access::<ArchivedCacheFile, rkyv::rancor::Error>(&archive).unwrap();
        let loaded: CacheFile = from_archived(archived);
        assert_eq!(loaded, value);
    }
}
