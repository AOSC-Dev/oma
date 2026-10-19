//! Atomic cache writes through a temporary file.

use std::io::{self, Write};
use std::path::Path;

use tempfile::NamedTempFile;

/// Write `bytes` to `target` through a uniquely named temporary file in the
/// same directory, then rename it into place. `NamedTempFile` creates the
/// file with `O_EXCL`, so concurrent writers cannot truncate each other, and
/// removes it on drop if writing or renaming fails.
pub(crate) fn write(target: &Path, bytes: &[u8]) -> io::Result<()> {
    let parent = match target.parent() {
        Some(parent) if !parent.as_os_str().is_empty() => parent,
        _ => Path::new("."),
    };
    std::fs::create_dir_all(parent)?;

    let mut tmp = NamedTempFile::new_in(parent)?;
    tmp.write_all(bytes)?;
    tmp.as_file().sync_all()?;
    tmp.persist(target).map_err(|error| error.error)?;
    Ok(())
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
