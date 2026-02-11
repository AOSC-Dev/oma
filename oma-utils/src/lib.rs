use std::{os::fd::OwnedFd, path::Path};

use nix::{
    errno::Errno,
    fcntl::{
        FcntlArg::{F_GETLK, F_SETFD, F_SETLK},
        FdFlag, OFlag, fcntl, open,
    },
    libc::{F_WRLCK, SEEK_SET, flock},
    sys::stat::Mode,
    unistd::close,
};
pub use os_release::OsRelease;

#[cfg(feature = "dbus")]
pub mod dbus;
use spdlog::debug;
use sysinfo::{Pid, System};

#[cfg(feature = "dbus")]
pub use zbus;

#[cfg(feature = "dpkg")]
pub mod dpkg;
#[cfg(feature = "human-bytes")]
pub mod human_bytes;
#[cfg(feature = "url-no-escape")]
pub mod url_no_escape;

#[inline]
pub fn is_termux() -> bool {
    std::env::var("TERMUX_VERSION").is_ok_and(|v| !v.is_empty())
}

#[inline]
pub fn concat_url(url: &str, path: &str) -> String {
    format!(
        "{}/{}",
        url.trim_end_matches('/'),
        path.trim_start_matches('/')
    )
}

#[derive(thiserror::Error, Debug)]
pub enum GetLockError {
    #[error("Set lock failed")]
    SetLock(Errno),
    #[error("Set lock failed: process {0} ({1}) is using.")]
    SetLockWithProcess(String, i32),
}

/// Create unix file lock
pub fn get_file_lock(lock_path: &Path) -> Result<OwnedFd, GetLockError> {
    let fd = open(
        lock_path,
        OFlag::O_RDWR | OFlag::O_CREAT | OFlag::O_NOFOLLOW,
        Mode::from_bits_truncate(0o640),
    )
    .map_err(GetLockError::SetLock)?;

    fcntl(&fd, F_SETFD(FdFlag::FD_CLOEXEC)).map_err(GetLockError::SetLock)?;

    // From apt libapt-pkg/fileutil.cc:287
    let mut fl = flock {
        l_type: F_WRLCK as i16,
        l_whence: SEEK_SET as i16,
        l_start: 0,
        l_len: 0,
        l_pid: -1,
    };

    if let Err(e) = fcntl(&fd, F_SETLK(&fl)) {
        debug!("{e}");

        if e == Errno::EACCES || e == Errno::EAGAIN {
            fl.l_type = F_WRLCK as i16;
            fl.l_whence = SEEK_SET as i16;
            fl.l_len = 0;
            fl.l_start = 0;
            fl.l_pid = -1;

            fcntl(&fd, F_GETLK(&mut fl)).ok();
        } else {
            fl.l_pid = -1;
        }

        close(fd).map_err(GetLockError::SetLock)?;

        if fl.l_pid != -1 {
            let mut sys = System::new();
            sys.refresh_processes(sysinfo::ProcessesToUpdate::All, true);
            let Some(process) = sys.process(Pid::from(fl.l_pid as usize)) else {
                return Err(GetLockError::SetLock(e));
            };

            return Err(GetLockError::SetLockWithProcess(
                process.name().to_string_lossy().into(),
                fl.l_pid,
            ));
        }

        return Err(GetLockError::SetLock(e));
    }

    Ok(fd)
}
