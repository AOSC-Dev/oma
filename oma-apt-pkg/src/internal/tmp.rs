//! Temporary paths and atomic cache writes.

use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

static COUNTER: AtomicU64 = AtomicU64::new(0);

/// A temporary file next to its target, so a rename can replace the target
/// atomically. The path is unique per process and call — concurrent writers
/// cannot truncate each other's temporary file.
///
/// The file is removed on drop unless [`TempPath::persist`] renamed it.
pub(crate) struct TempPath {
    path: PathBuf,
    persisted: bool,
}

impl TempPath {
    pub(crate) fn new(target: &Path) -> Self {
        let mut name = target
            .file_name()
            .map(|name| name.to_os_string())
            .unwrap_or_default();
        name.push(format!(
            ".tmp.{}.{}",
            std::process::id(),
            COUNTER.fetch_add(1, Ordering::Relaxed)
        ));

        Self {
            path: target.with_file_name(name),
            persisted: false,
        }
    }

    pub(crate) fn path(&self) -> &Path {
        &self.path
    }

    /// Rename the temporary file over `target`.
    pub(crate) fn persist(mut self, target: &Path) -> io::Result<()> {
        std::fs::rename(&self.path, target)?;
        self.persisted = true;
        Ok(())
    }
}

impl Drop for TempPath {
    fn drop(&mut self) {
        if !self.persisted {
            let _ = std::fs::remove_file(&self.path);
        }
    }
}

/// Write `bytes` to `target` through a temporary file and rename, creating
/// the parent directory if needed. The temporary file is cleaned up if
/// writing or renaming fails.
pub(crate) fn write(target: &Path, bytes: &[u8]) -> io::Result<()> {
    if let Some(parent) = target.parent() {
        std::fs::create_dir_all(parent)?;
    }

    let tmp = TempPath::new(target);
    {
        let mut file = std::fs::File::create(tmp.path())?;
        file.write_all(bytes)?;
        file.sync_all()?;
    }
    tmp.persist(target)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn write_replaces_the_target_and_leaves_no_temp_file() {
        let dir = tempfile::tempdir().unwrap();
        let target = dir.path().join("cache.bin");

        write(&target, b"first").unwrap();
        assert_eq!(std::fs::read(&target).unwrap(), b"first");

        write(&target, b"second").unwrap();
        assert_eq!(std::fs::read(&target).unwrap(), b"second");

        let entries: Vec<_> = std::fs::read_dir(dir.path())
            .unwrap()
            .map(|entry| entry.unwrap().file_name())
            .collect();
        assert_eq!(entries, vec![std::ffi::OsString::from("cache.bin")]);
    }
}
