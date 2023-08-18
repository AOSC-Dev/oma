use std::{
    io::Write,
    process::{Command, Stdio},
};

use oma_console::debug;

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

/// Get architecture from dpkg
pub fn dpkg_arch() -> Result<String, DpkgError> {
    let dpkg = Command::new("dpkg").arg("--print-architecture").output()?;

    if !dpkg.status.success() {
        return Err(DpkgError::DpkgRunError(dpkg.status.code().unwrap_or(1)));
    }

    let output = std::str::from_utf8(&dpkg.stdout)?.trim().to_string();

    Ok(output)
}

/// Mark hold/unhold status use dpkg --set-selections
pub fn mark_version_status(
    pkgs: &[String],
    hold: bool,
    dry_run: bool,
) -> Result<Vec<(&str, bool)>, DpkgError> {
    let dpkg = Command::new("dpkg").arg("--get-selections").output()?;

    if !dpkg.status.success() {
        return Err(DpkgError::DpkgRunError(dpkg.status.code().unwrap_or(1)));
    }

    let mut selections = std::str::from_utf8(&dpkg.stdout)?.split('\n');
    selections.nth_back(0);

    let list = Some(())
        .and_then(|_| {
            let mut list = vec![];
            for i in selections {
                let mut split = i.split_whitespace();
                let name = split.next()?;
                let status = split.next()?;

                list.push((name, status));
            }

            Some(list)
        })
        .ok_or_else(|| DpkgError::FailedToQueryDpkgDatabase)?;

    let mut res = vec![];

    for pkg in pkgs {
        if list.iter().any(|(x, status)| {
            x == pkg
                && match hold {
                    true => status == &"hold",
                    false => status == &"install",
                }
        }) {
            debug!("pkg {pkg} set to hold = {hold} is set = false");
            res.push((pkg.as_str(), false));
            continue;
        }

        debug!("pkg {pkg} set to hold = {hold} is set = true");

        if dry_run {
            continue;
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

        res.push((pkg.as_str(), true));
    }

    Ok(res)
}
