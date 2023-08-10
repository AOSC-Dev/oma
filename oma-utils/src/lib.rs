use std::{
    fmt,
    io::Write,
    path::PathBuf,
    process::{Command, Stdio},
};

use number_prefix::NumberPrefix;
use once_cell::sync::Lazy;

static LOCK: Lazy<PathBuf> = Lazy::new(|| PathBuf::from("/run/lock/oma.lock"));

type IOResult<T> = std::io::Result<T>;

pub use os_release::OsRelease;

#[derive(Debug, thiserror::Error)]
pub enum DpkgError {
    #[error(transparent)]
    IOError(#[from] std::io::Error),
    #[error("`dpkg' returned an error: {0}")]
    DpkgRunError(i32),
    #[error(transparent)]
    Utf8Error(#[from] std::str::Utf8Error),
    #[error("Failed to query dpkg database")]
    FailedToQueryDpkgDatabase,
}

pub fn dpkg_arch() -> Result<String, DpkgError> {
    let dpkg = Command::new("dpkg").arg("--print-architecture").output()?;

    if !dpkg.status.success() {
        return Err(DpkgError::DpkgRunError(dpkg.status.code().unwrap_or(1)));
    }

    let output = std::str::from_utf8(&dpkg.stdout)?.trim().to_string();

    Ok(output)
}

pub fn mark_version_status(pkg: &str, hold: bool) -> Result<bool, DpkgError> {
    let dpkg = Command::new("dpkg").arg("--get-selections").output()?;

    if !dpkg.status.success() {
        return Err(DpkgError::DpkgRunError(dpkg.status.code().unwrap_or(1)));
    }

    let mut seclections = std::str::from_utf8(&dpkg.stdout)?.split('\n');
    seclections.nth_back(0);

    let list = Some(())
        .and_then(|_| {
            let mut list = vec![];
            for i in seclections {
                let mut split = i.split_whitespace();
                let name = split.next()?;
                let status = split.next()?;

                list.push((name.to_string(), status.to_string()));
            }

            Some(list)
        })
        .ok_or_else(|| DpkgError::FailedToQueryDpkgDatabase)?;

    if list.iter().any(|(x, status)| {
        x == pkg
            && match hold {
                true => status == "hold",
                false => status == "install",
            }
    }) {
        return Ok(false);
    }

    let mut dpkg = Command::new("dpkg")
        .arg("--set-selections")
        .stdin(Stdio::piped())
        .spawn()?;

    let op = match hold {
        true => "hold",
        false => "install",
    };

    dpkg.stdin
        .as_mut()
        .unwrap()
        .write_all(format!("{pkg} {op}").as_bytes())?;

    dpkg.wait()?;

    Ok(true)
}

/// Formats bytes for human readability
#[derive(Debug)]
pub struct HumanBytes(pub u64);

impl fmt::Display for HumanBytes {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match NumberPrefix::binary(self.0 as f64) {
            NumberPrefix::Standalone(number) => write!(f, "{number:.0}B"),
            NumberPrefix::Prefixed(prefix, number) => write!(f, "{number:.2} {prefix}B"),
        }
    }
}

pub fn lock_oma() -> IOResult<()> {
    if !LOCK.is_file() {
        std::fs::create_dir_all("/run/lock")?;
        std::fs::File::create(&*LOCK)?;
    }

    Ok(())
}

pub fn unlock_oma() -> IOResult<()> {
    if LOCK.is_file() {
        std::fs::remove_file(&*LOCK)?;
    }

    Ok(())
}

pub fn terminal_ring() {
    eprint!("\x07"); // bell character
}
