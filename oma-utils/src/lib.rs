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
use oma_logger::debug;
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

/// True values are `y`, `yes`, `t`, `true`, `on`, and `1`; false values are
/// `n`, `no`, `f`, `false`, `off`, and `0`.
///
/// Both lists are clap's `BoolishValueParser` literals verbatim; matching is
/// case-insensitive. See:
/// - https://github.com/clap-rs/clap/blob/v4.6.2/clap_builder/src/util/str_to_bool.rs#L2-L24
///   (the literal lists and `str_to_bool`)
const TRUE_LITERALS: [&str; 6] = ["y", "yes", "t", "true", "on", "1"];
const FALSE_LITERALS: [&str; 6] = ["n", "no", "f", "false", "off", "0"];

/// Convert a string literal representation of truth to true or false, like
/// clap's `str_to_bool` behind [`BoolishValueParser`](https://docs.rs/clap/latest/clap/builder/struct.BoolishValueParser.html).
///
/// Treat true/false values as case-insensitive.
#[inline]
pub fn str_to_bool(val: impl AsRef<str>) -> Option<bool> {
    let pat = val.as_ref().to_ascii_lowercase();
    if TRUE_LITERALS.contains(&pat.as_str()) {
        Some(true)
    } else if FALSE_LITERALS.contains(&pat.as_str()) {
        Some(false)
    } else {
        None
    }
}

/// Detect if we are running in a CI environment (e.g. GitHub Actions, GitLab
/// CI).
#[inline]
pub fn is_ci() -> bool {
    std::env::var("CI")
        .ok()
        .and_then(str_to_bool)
        .unwrap_or(false)
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

#[cfg(test)]
mod tests {
    use super::is_ci;

    #[test]
    fn test_is_ci() {
        let original = std::env::var("CI").ok();

        unsafe {
            std::env::set_var("CI", "1");
            assert!(is_ci());

            std::env::set_var("CI", "true");
            assert!(is_ci());

            std::env::set_var("CI", "True");
            assert!(is_ci());

            // The remaining BoolishValueParser truthy literals.
            std::env::set_var("CI", "yes");
            assert!(is_ci());
            std::env::set_var("CI", "on");
            assert!(is_ci());
            std::env::set_var("CI", "y");
            assert!(is_ci());
            std::env::set_var("CI", "t");
            assert!(is_ci());

            std::env::set_var("CI", "0");
            assert!(!is_ci());

            std::env::set_var("CI", "false");
            assert!(!is_ci());

            // Explicit falses beyond "0"/"false" are also not-CI.
            std::env::set_var("CI", "no");
            assert!(!is_ci());
            std::env::set_var("CI", "off");
            assert!(!is_ci());

            // Unrecognized values are strict: not-CI.
            std::env::set_var("CI", "2");
            assert!(!is_ci());

            std::env::set_var("CI", "");
            assert!(!is_ci());

            std::env::remove_var("CI");
            assert!(!is_ci());

            match original {
                Some(v) => std::env::set_var("CI", v),
                None => std::env::remove_var("CI"),
            }
        }
    }
}
