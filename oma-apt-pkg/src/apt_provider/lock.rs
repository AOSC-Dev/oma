//! File locks for the install flow — apt's dpkg frontend lock and the
//! archive (acquire) lock, so concurrent package managers don't stomp on
//! each other.
//!
//! Reuses [`oma_utils::get_file_lock`] — the same `fcntl` write-lock as
//! apt's `GetLock` (libapt fileutil.cc): it fails immediately (instead of
//! blocking) when another process holds the lock, so a second package
//! manager gets an error instead of hanging.

use std::os::fd::OwnedFd;
use std::path::Path;

use oma_utils::{GetLockError, get_file_lock};
use thiserror::Error;

/// Errors from acquiring a lock.
#[derive(Debug, Error)]
pub enum LockError {
    /// Opening or locking the file failed.
    #[error("failed to lock {path}: {err}")]
    Failed { path: String, err: String },
    /// The file is already locked by another process.
    #[error("{path} is locked by another process")]
    Held { path: String },
}

/// An exclusive advisory lock on a file, released when dropped (the file
/// descriptor is closed, dropping the `fcntl` lock). Reuses
/// [`oma_utils::get_file_lock`].
pub struct LockGuard {
    // Held open (and only that) so the fcntl lock lives as long as the
    // guard; closing it on drop releases the lock.
    #[allow(dead_code)]
    fd: OwnedFd,
}

impl LockGuard {
    /// Acquire the lock at `path`, creating the file (mode `0640`, like
    /// apt's lock files) if it does not exist. Fails with
    /// [`LockError::Held`] when another process holds the lock.
    pub fn acquire(path: impl AsRef<Path>) -> Result<Self, LockError> {
        let path = path.as_ref();
        match get_file_lock(path) {
            Ok(fd) => Ok(Self { fd }),
            Err(GetLockError::SetLock(errno)) => Err(LockError::Failed {
                path: path.display().to_string(),
                err: errno.to_string(),
            }),
            // Another process (with a reported name/pid) holds the lock.
            Err(GetLockError::SetLockWithProcess(_, _)) => Err(LockError::Held {
                path: path.display().to_string(),
            }),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn acquire_creates_the_lock_file_and_is_reentrant() {
        let path = std::env::temp_dir().join("oma-lock-test");
        let _ = std::fs::remove_file(&path);

        // Acquiring creates the file; a second acquire in the same process
        // also succeeds (POSIX fcntl locks are per-process), and dropping
        // the guards releases it for the next acquire.
        let guard = LockGuard::acquire(&path).unwrap();
        assert!(path.exists());
        let guard2 = LockGuard::acquire(&path).unwrap();
        drop(guard);
        drop(guard2);
        LockGuard::acquire(&path).unwrap();

        let _ = std::fs::remove_file(&path);
    }
}
