use std::{
    fmt::Display,
    io::Write,
    path::{Path, PathBuf},
    process::{exit, Command},
    str::FromStr,
    sync::atomic::Ordering,
};

use std::fmt::Debug;

use anyhow::{anyhow, bail, Error, Result};
use once_cell::sync::Lazy;
use rust_apt::util::DiskSpace;

use indicatif::HumanBytes;
use sysinfo::{Pid, System, SystemExt};

use crate::{oma::Action, ARGS, DRYRUN};

static LOCK: Lazy<PathBuf> = Lazy::new(|| PathBuf::from("/run/lock/oma.lock"));

pub fn get_arch_name() -> Result<String> {
    let dpkg = Command::new("dpkg")
        .arg("--print-architecture")
        .output()
        .map_err(|e| anyhow!("Can not run dpkg, why: {e}"))?;

    if !dpkg.status.success() {
        bail!("dpkg return non-zero code: {:?}", dpkg.status.code());
    }

    let output = std::str::from_utf8(&dpkg.stdout)?.trim().to_string();

    Ok(output)
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
        let old_pid = std::fs::read_to_string(LOCK.as_path())?;
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

    if !Path::new("/run/lock").is_dir() {
        std::fs::create_dir_all("/run/lock")
            .map_err(|e| anyhow!("Can not create /run/lock dir! why: {e}"))?;
    }

    let mut lock_file = std::fs::File::create(LOCK.as_path())
        .map_err(|e| anyhow!("Can not create lock file! why: {e}"))?;
    let pid = std::process::id().to_string();

    // Set global lock parameter
    crate::LOCKED.store(true, Ordering::Relaxed);

    lock_file
        .write_all(pid.as_bytes())
        .map_err(|e| anyhow!("Can not write oma lock, why: {e}"))?;

    Ok(())
}

/// Unlock oma
pub fn unlock_oma() -> Result<()> {
    if LOCK.exists() {
        std::fs::remove_file(LOCK.as_path())
            .map_err(|e| anyhow!("Can not unlock oma, why: {e}"))?;
    }

    Ok(())
}

pub fn log_to_file(action: &Action, start_time: &str, end_time: &str) -> Result<()> {
    if DRYRUN.load(Ordering::Relaxed) {
        return Ok(());
    }

    std::fs::create_dir_all("/var/log/oma")
        .map_err(|e| anyhow!("Can not create oma log directory, why: {e}"))?;

    let mut f = std::fs::OpenOptions::new()
        .append(true)
        .create(true)
        .open("/var/log/oma/history")
        .map_err(|e| anyhow!("Can not create oma history file, why: {e}"))?;

    f.write_all(format!("Start-Date: {start_time}\n").as_bytes())?;
    f.write_all(format!("Action: {}\n{action:#?}", *ARGS).as_bytes())?;
    f.write_all(format!("End-Date: {end_time}\n\n").as_bytes())?;

    Ok(())
}

/// Check user is root
#[inline]
pub fn needs_root() -> Result<()> {
    polkit_run_itself()?;

    Ok(())
}

/// Get apt style url: like: go_1.19.4%2btools0.4.0%2bnet0.4.0_amd64.deb
#[inline]
pub fn apt_style_url(s: &str) -> String {
    s.replace(':', "%3a")
        .replace('+', "%2b")
        .replace('~', "%7e")
}

/// Reverse apt style url: like: file:/home/saki/aoscpt/go_1.19.4+tools0.4.0+net0.4.0_amd64.deb
#[inline]
pub fn reverse_apt_style_url(s: &str) -> String {
    s.replace("%3a", ":")
        .replace("%2b", "+")
        .replace("%7e", "~")
}

pub fn polkit_run_itself() -> Result<()> {
    if nix::unistd::geteuid().is_root() {
        return Ok(());
    }

    let args = ARGS.split(' ').collect::<Vec<_>>();
    let out = Command::new("pkexec")
        .args(args)
        .spawn()
        .map_err(|e| anyhow!("Spawn pkexec failed, why: {e}"))?
        .wait_with_output()
        .map_err(|e| anyhow!("Spawn pkexec failed, why: {e}"))?;

    exit(
        out.status
            .code()
            .expect("Can not get pkexec oma exit status"),
    );
}

pub fn error_due_to<
    M: Display + Debug + Send + Sync + 'static,
    S: Display + Debug + Send + Sync + 'static,
>(
    err: M,
    due_to: S,
) -> Error {
    let e = anyhow!(due_to);

    e.context(err)
}

// input: like http://50.50.1.183/debs/pool/stable/main/f/fish_3.6.0-0_amd64.deb
// output: http://50.50.1.183/debs stable main
pub fn source_url_to_apt_style(s: &str) -> Option<String> {
    let mut s_split = s.split('/').rev();
    let component = s_split.nth(2)?;
    let branch = s_split.next()?;
    let pool = s_split.position(|x| x == "pool")?;

    let host = &s_split.rev().collect::<Vec<_>>().join("/")[pool..];

    Some(format!("{host} {branch} {component}"))
}

#[test]
fn test_source_url_to_apt_style() {
    let url = "http://50.50.1.183/debs/pool/stable/main/f/fish_3.6.0-0_amd64.deb";
    let s = source_url_to_apt_style(url);

    assert_eq!(s, Some("http://50.50.1.183/debs stable main".to_string()));
}
