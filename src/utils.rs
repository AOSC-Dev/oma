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
use rust_apt::{cache::Cache, util::DiskSpace};

use indicatif::HumanBytes;
use sysinfo::{Pid, System, SystemExt};
use time::OffsetDateTime;

use crate::{
    fl,
    history::{log_to_file, Operation},
    oma::Action,
    ARGS, TIME_OFFSET,
};

use crate::oma::apt_install;

use rust_apt::config::Config as AptConfig;

static LOCK: Lazy<PathBuf> = Lazy::new(|| PathBuf::from("/run/lock/oma.lock"));

pub fn get_arch_name() -> Result<String> {
    let dpkg = Command::new("dpkg")
        .arg("--print-architecture")
        .output()
        .map_err(|e| anyhow!(fl!("can-not-run-dpkg-print-arch", e = e.to_string())))?;

    if !dpkg.status.success() {
        bail!(fl!("dpkg-return-non-zero", e = dpkg.status.code().unwrap()));
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
        bail!(fl!(
            "need-more-size",
            n = need_str.to_string(),
            a = avail_str.to_string()
        ));
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
            bail!(fl!("old-pid-still-running", pid = old_pid.to_string()));
        } else {
            unlock_oma()?;
        }
    }

    if !Path::new("/run/lock").is_dir() {
        std::fs::create_dir_all("/run/lock")
            .map_err(|e| anyhow!(fl!("can-not-create-lock-dir", e = e.to_string())))?;
    }

    let mut lock_file = std::fs::File::create(LOCK.as_path())
        .map_err(|e| anyhow!(fl!("can-not-create-lock-file", e = e.to_string())))?;
    let pid = std::process::id().to_string();

    // Set global lock parameter
    crate::LOCKED.store(true, Ordering::Relaxed);

    lock_file
        .write_all(pid.as_bytes())
        .map_err(|e| anyhow!(fl!("can-not-write-lock-file", e = e.to_string())))?;

    Ok(())
}

/// Unlock oma
pub fn unlock_oma() -> Result<()> {
    if LOCK.exists() {
        std::fs::remove_file(LOCK.as_path())
            .map_err(|e| anyhow!(fl!("can-not-unlock-oma", e = e.to_string())))?;
    }

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
        .and_then(|x| x.wait_with_output())
        .map_err(|e| anyhow!(fl!("execute-pkexec-fail", e = e.to_string())))?;

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

#[macro_export]
macro_rules! handle_install_error {
    ($f:expr, $count:ident, $start_time:ident, $op:ident) => {
        loop {
            match $f {
                Err(e) => match e {
                    InstallError::Anyhow(e) => return Err(e),
                    InstallError::RustApt { source, action } => {
                        if $count == 3 {
                            let end_time = OffsetDateTime::now_utc()
                                .to_offset(*TIME_OFFSET)
                                .to_string();

                            log_to_file(&action, $start_time.as_str(), &end_time, $op, false)?;
                            return Err(source.into());
                        }
                        $count += 1;
                    }
                },
                Ok(v) => {
                    let end_time = OffsetDateTime::now_utc()
                        .to_offset(*TIME_OFFSET)
                        .to_string();

                    log_to_file(&v, &$start_time, &end_time, $op, true)?;

                    return Ok(0);
                }
            }
        }
    };
}

pub fn handle_install_error_no_retry(
    action: Action,
    cache: Cache,
    start_time: &str,
    yes: bool,
    force_yes: bool,
    force_confnew: bool,
    dpkg_force_all: bool,
) -> Result<()> {
    match apt_install(
        action.clone(),
        AptConfig::new_clear(),
        cache,
        yes,
        force_yes,
        force_confnew,
        dpkg_force_all,
    ) {
        Ok(_) => {
            let end_time = OffsetDateTime::now_utc()
                .to_offset(*TIME_OFFSET)
                .to_string();

            log_to_file(&action, start_time, &end_time, Operation::Other, true)?;

            Ok(())
        }
        Err(e) => {
            let end_time = OffsetDateTime::now_utc()
                .to_offset(*TIME_OFFSET)
                .to_string();

            log_to_file(&action, start_time, &end_time, Operation::Other, true)?;

            Err(e.into())
        }
    }
}

// input: like http://50.50.1.183/debs/pool/stable/main/f/fish_3.6.0-0_amd64.deb
// output: http://50.50.1.183/debs stable main
pub fn source_url_to_apt_style(s: &str) -> Option<String> {
    let mut s_split = s.split('/');
    let component = s_split.nth_back(2)?;
    let branch = s_split.next_back()?;
    let _ = s_split.next_back();

    let host = &s_split.collect::<Vec<_>>().join("/");

    Some(format!("{host} {branch} {component}"))
}

#[test]
fn test_source_url_to_apt_style() {
    let url = "http://50.50.1.183/debs/pool/stable/main/f/fish_3.6.0-0_amd64.deb";
    let s = source_url_to_apt_style(url);

    assert_eq!(s, Some("http://50.50.1.183/debs stable main".to_string()));
}
