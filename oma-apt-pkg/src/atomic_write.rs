//! Atomic write-back for the lossless state files.
//!
//! State files (dpkg status, apt extended states) are databases, not caches:
//! a write that is interrupted halfway — full disk, killed process — must not
//! leave a truncated file behind. The new contents go to a unique sibling
//! temp file, are flushed and synced, and the temp file is renamed over the
//! target, which is atomic within a directory. This mirrors APT's
//! `FileFd::Atomic` (`mkstemp` + `rename`, `apt-pkg/contrib/fileutl.cc`).
//!
//! The replacement is the only thing that is atomic here: what is written has
//! to be up to date already, so serializing a read-modify-write against other
//! users of the file is the caller's job (see the single-writer note on
//! [`DpkgState::to_file`](crate::dpkg_state::DpkgState::to_file) and
//! [`AptExtendedStates::to_file`](crate::extended_states::AptExtendedStates::to_file)).

use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process;
use std::sync::atomic::{AtomicU64, Ordering};

/// Distinguishes the temp files of concurrent (and repeated) writes in this
/// process; the pid distinguishes processes.
static TEMP_SEQUENCE: AtomicU64 = AtomicU64::new(0);

/// Write `contents` to `path` atomically, so readers see either the old file
/// or the complete new one — never a partial write. A failed write leaves the
/// original file untouched (and removes the temp file).
pub(crate) fn write(path: &Path, contents: &str) -> std::io::Result<()> {
    let (mut file, tmp) = create_temp_file(path)?;

    match write_temp_file(&mut file, path, contents).and_then(|()| fs::rename(&tmp, path)) {
        Ok(()) => Ok(()),
        Err(e) => {
            let _ = fs::remove_file(&tmp);
            Err(e)
        }
    }
}

/// Create the unique temp file next to `path`. Like `mkstemp`, the name
/// carries a pid and a per-process sequence, and `create_new` (`O_EXCL`)
/// fails on an existing file instead of following it — two writers never
/// share a temp file, so neither can rename the other's bytes into place. A
/// collision (pid reuse, a leftover temp) just tries the next sequence.
fn create_temp_file(path: &Path) -> std::io::Result<(fs::File, PathBuf)> {
    for _ in 0..64 {
        let tmp = temp_path(path);
        match fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&tmp)
        {
            Ok(file) => return Ok((file, tmp)),
            Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => continue,
            Err(e) => return Err(e),
        }
    }
    Err(std::io::Error::new(
        std::io::ErrorKind::AlreadyExists,
        "no unique temporary file name available",
    ))
}

/// Fill the temp file, then give it the target's permissions — the rename
/// replaces the inode, so a temp created under the caller's umask would
/// otherwise narrow the target's mode (a root `umask 077` would turn the
/// world-readable dpkg status into `0600`). The target not existing yet
/// keeps the umask-derived default.
fn write_temp_file(file: &mut fs::File, path: &Path, contents: &str) -> std::io::Result<()> {
    file.write_all(contents.as_bytes())?;
    file.sync_all()?;

    if let Ok(metadata) = fs::metadata(path) {
        file.set_permissions(metadata.permissions())?;
    }
    Ok(())
}

/// The temp path next to `path` — same directory, so the rename stays within
/// one filesystem (a cross-device rename fails instead of replacing).
fn temp_path(path: &Path) -> PathBuf {
    let sequence = TEMP_SEQUENCE.fetch_add(1, Ordering::Relaxed);
    let mut tmp = path.as_os_str().to_owned();
    tmp.push(format!(".{}.{sequence}.tmp", process::id()));
    PathBuf::from(tmp)
}

/// Resolve `path` for reading and writing while keeping symlinks intact: the
/// final component is resolved when it exists (a symlinked state file is
/// replaced at its target, not turned into a regular file), and the parent
/// directory always is — so a file that does not exist yet still records the
/// directory it will be created in.
pub(crate) fn resolve(path: &Path) -> PathBuf {
    if let Ok(canonical) = fs::canonicalize(path) {
        return canonical;
    }
    match (path.parent(), path.file_name()) {
        (Some(parent), Some(name)) => match fs::canonicalize(parent) {
            Ok(parent) => parent.join(name),
            Err(_) => path.to_path_buf(),
        },
        _ => path.to_path_buf(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The sorted directory entries, so a test can assert that no temp file
    /// was left behind.
    fn entries(dir: &Path) -> Vec<String> {
        let mut names: Vec<String> = fs::read_dir(dir)
            .unwrap()
            .map(|entry| entry.unwrap().file_name().to_string_lossy().into_owned())
            .collect();
        names.sort();
        names
    }

    #[test]
    fn replaces_the_target_and_leaves_no_temp() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("state");
        fs::write(&path, "old").unwrap();

        write(&path, "new").unwrap();

        assert_eq!(fs::read_to_string(&path).unwrap(), "new");
        assert_eq!(entries(dir.path()), vec!["state"]);
    }

    /// The temp file adopts the target's mode, so the rename cannot narrow
    /// it (writing as root under `umask 077` would otherwise turn a
    /// world-readable status file into `0600`).
    #[test]
    fn preserves_the_target_permissions() {
        use std::os::unix::fs::PermissionsExt;

        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("state");
        fs::write(&path, "old").unwrap();
        fs::set_permissions(&path, fs::Permissions::from_mode(0o640)).unwrap();

        write(&path, "new").unwrap();

        let mode = fs::metadata(&path).unwrap().permissions().mode() & 0o777;
        assert_eq!(mode, 0o640);
    }

    /// Two writers must not share a temp file: with a deterministic
    /// `<path>.tmp` the second writer could truncate the file the first was
    /// writing, so the first would rename someone else's bytes into place.
    #[test]
    fn does_not_share_a_predictable_temp_name() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("state");
        fs::write(&path, "old").unwrap();
        // A decoy in the "obvious" location, as a second writer would use.
        let decoy = dir.path().join("state.tmp");
        fs::write(&decoy, "theirs").unwrap();

        write(&path, "new").unwrap();

        assert_eq!(fs::read_to_string(&path).unwrap(), "new");
        // The decoy is untouched: no foreign temp file was reused.
        assert_eq!(fs::read_to_string(&decoy).unwrap(), "theirs");
    }

    /// Concurrent writers each install a complete snapshot: the file always
    /// holds one of the two inputs, never a truncated or mixed write.
    #[test]
    fn concurrent_writes_install_a_complete_snapshot() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("state");
        fs::write(&path, "start").unwrap();

        let a = "a".repeat(16 * 1024);
        let b = "b".repeat(16 * 1024);
        let threads: Vec<_> = [(path.clone(), a.clone()), (path.clone(), b.clone())]
            .into_iter()
            .map(|(path, contents)| {
                std::thread::spawn(move || {
                    for _ in 0..10 {
                        write(&path, &contents).unwrap();
                    }
                })
            })
            .collect();
        for thread in threads {
            thread.join().unwrap();
        }

        let final_contents = fs::read_to_string(&path).unwrap();
        assert!(
            final_contents == a || final_contents == b,
            "expected a complete snapshot, got {} bytes",
            final_contents.len()
        );
        assert_eq!(entries(dir.path()), vec!["state"]);
    }

    /// A write whose rename fails keeps the target intact and cleans up the
    /// temp file.
    #[test]
    fn a_failed_write_leaves_the_target_untouched() {
        let dir = tempfile::tempdir().unwrap();
        // A non-empty directory as the target: the final rename cannot
        // replace it.
        let target = dir.path().join("state");
        fs::create_dir(&target).unwrap();
        fs::write(target.join("inner"), "keep").unwrap();

        assert!(write(&target, "new").is_err());

        assert_eq!(fs::read_to_string(target.join("inner")).unwrap(), "keep");
        assert_eq!(entries(dir.path()), vec!["state"]);
    }
}
