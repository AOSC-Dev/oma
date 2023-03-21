use std::{
    io::{Read, Write},
    path::{Path, PathBuf},
    str::FromStr,
    sync::atomic::Ordering,
};

use anyhow::{bail, Result};
use once_cell::sync::Lazy;
use rust_apt::util::DiskSpace;

use indicatif::HumanBytes;
use sysinfo::{Pid, System, SystemExt};

use crate::action::{Action, LogAction};

static LOCK: Lazy<PathBuf> = Lazy::new(|| PathBuf::from("/run/lock/oma.lock"));

#[cfg(target_arch = "powerpc64")]
#[inline]
pub fn get_arch_name() -> Option<&'static str> {
    use nix::libc;
    let mut endian: libc::c_int = -1;
    let result;
    unsafe {
        result = libc::prctl(libc::PR_GET_ENDIAN, &mut endian as *mut libc::c_int);
    }
    if result < 0 {
        return None;
    }
    match endian {
        libc::PR_ENDIAN_LITTLE | libc::PR_ENDIAN_PPC_LITTLE => Some("ppc64el"),
        libc::PR_ENDIAN_BIG => Some("ppc64"),
        _ => None,
    }
}

/// AOSC OS specific architecture mapping table
#[cfg(not(target_arch = "powerpc64"))]
#[inline]
pub fn get_arch_name() -> Option<&'static str> {
    use std::env::consts::ARCH;
    match ARCH {
        "x86_64" => Some("amd64"),
        "x86" => Some("i486"),
        "powerpc" => Some("powerpc"),
        "aarch64" => Some("arm64"),
        "mips64" => Some("loongson3"),
        "riscv64" => Some("riscv64"),
        _ => None,
    }
}

pub fn size_checker(size: &DiskSpace, download_size: u64) -> Result<()> {
    let size = match size {
        DiskSpace::Require(v) => *v as i64,
        DiskSpace::Free(v) => 0 - *v as i64,
    };

    let avail = fs4::available_space(Path::new("/"))? as i64;
    let avail_str = HumanBytes(avail as u64);
    let need = size + download_size as i64;
    let need_str = HumanBytes(need as u64);

    if need > avail {
        bail!("Your disk space is too small, need size: {need_str}, available space: {avail_str}")
    }

    Ok(())
}

/// Lock oma
pub fn lock_oma() -> Result<()> {
    if LOCK.is_file() {
        let mut lock_file = std::fs::File::open(LOCK.as_path())?;
        let mut old_pid = String::new();
        lock_file.read_to_string(&mut old_pid)?;

        let s = System::new_all();
        let old_pid = Pid::from_str(&old_pid)?;

        if s.process(old_pid).is_some() {
            bail!(
                "Another instance of oma (pid: {}) is still running!",
                old_pid
            );
        } else {
            unlock_oma()?;
        }
    }
    let mut lock_file = std::fs::File::create(LOCK.as_path())?;
    let pid = std::process::id().to_string();

    // Set global lock parameter
    crate::LOCKED.store(true, Ordering::Relaxed);

    lock_file.write_all(pid.as_bytes())?;

    Ok(())
}

/// Unlock oma
pub fn unlock_oma() -> Result<()> {
    if LOCK.exists() {
        std::fs::remove_file(LOCK.as_path())?;
    }

    Ok(())
}

pub fn log_to_file(action: &Action, start_time: &str, end_time: &str) -> Result<()> {
    std::fs::create_dir_all("/var/log/oma")?;

    let mut f = std::fs::OpenOptions::new()
        .append(true)
        .create(true)
        .open("/var/log/oma/history")?;

    f.write_all(format!("Start-Date: {start_time}\n").as_bytes())?;

    let _ = match &action.op {
        LogAction::Install(v) => {
            f.write_all(format!("Action: oma install {v:?}\nResolver: {action:?}\n").as_bytes())
        }
        LogAction::Upgrade(v) => {
            f.write_all(format!("Action: oma upgrade {v:?}\nResolver: {action:?}\n").as_bytes())
        }
        LogAction::Remove(v) => {
            f.write_all(format!("Action: oma remove {v:?}\nResolver: {action:?}\n").as_bytes())
        }
        LogAction::FixBroken => {
            f.write_all(format!("Action: oma fix-broken\nResolver: {action:?}\n").as_bytes())
        }
        LogAction::Pick(v) => {
            f.write_all(format!("Action: {v:?}\nResolver: {action:?}\n").as_bytes())
        }
    };

    f.write_all(format!("End-Date: {end_time}\n\n").as_bytes())?;

    Ok(())
}

/// Check user is root
pub fn is_root() -> Result<()> {
    if !nix::unistd::geteuid().is_root() {
        bail!("Please run me as root!");
    }

    Ok(())
}
