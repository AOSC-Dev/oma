use std::{fmt::Display, fs, io, path::Path, str::FromStr};

use deb822_lossless::{Deb822, Paragraph};
use spdlog::{debug, info};

use crate::apt::{OmaAptError, OmaAptResult};

const DPKG_STATUS_PATH: &str = "var/lib/dpkg/status";

#[derive(Debug, PartialEq, Eq)]
pub enum DpkgMarkStatus {
    Hold,
    UnHold,
}

impl Display for DpkgMarkStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}",
            match self {
                DpkgMarkStatus::Hold => "hold ok installed",
                DpkgMarkStatus::UnHold => "install ok installed",
            }
        )
    }
}

impl FromStr for DpkgMarkStatus {
    type Err = OmaAptError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "hold ok installed" => Ok(DpkgMarkStatus::Hold),
            "install ok installed" => Ok(DpkgMarkStatus::UnHold),
            status => Err(OmaAptError::WrongDpkgStatus(status.to_string())),
        }
    }
}

impl DpkgMarkStatus {
    fn set(&self, p: &mut Paragraph) {
        p.set("Status", &self.to_string());
    }
}

pub fn mark_status<'a>(
    pkgs: impl IntoIterator<Item = &'a str>,
    sysroot: impl AsRef<Path>,
    hold: bool,
    dry_run: bool,
) -> OmaAptResult<Vec<(&'a str, bool)>> {
    let mut res = vec![];
    let path = sysroot.as_ref().join(DPKG_STATUS_PATH);

    let dpkg_status = Deb822::from_file(&path).map_err(|e| {
        OmaAptError::FailedOperateDirOrFile(path.to_string_lossy().to_string(), io::Error::other(e))
    })?;

    let mut new_dpkg_status = dpkg_status.paragraphs().collect::<Vec<_>>();

    let status = if hold {
        DpkgMarkStatus::Hold
    } else {
        DpkgMarkStatus::UnHold
    };

    for pkg in pkgs {
        let binary = new_dpkg_status
            .iter_mut()
            .find(|entry| entry.get("Package").is_some_and(|p| p == pkg));

        let Some(binary) = binary else {
            return Err(OmaAptError::DpkgStatusGetPkg(pkg.to_string()));
        };

        if binary
            .get("Status")
            .ok_or_else(|| OmaAptError::DpkgStatusBroken(pkg.to_string()))?
            .parse::<DpkgMarkStatus>()?
            == status
        {
            res.push((pkg, false));
            continue;
        }

        status.set(binary);

        debug!("\n{}", binary);

        res.push((pkg, true));
    }

    if dry_run {
        info!("Dpkg status result:");
        info!("\n{}", Deb822::from_iter(new_dpkg_status));
    } else if res.iter().any(|e| e.1) {
        fs::copy(
            &path,
            path.parent()
                .ok_or_else(|| OmaAptError::FailedGetParentPath(path.to_path_buf()))?
                .join("status-old"),
        )
        .map_err(|e| OmaAptError::FailedOperateDirOrFile(DPKG_STATUS_PATH.to_string(), e))?;

        fs::write(
            path,
            Deb822::from_iter(new_dpkg_status).to_string().as_bytes(),
        )
        .map_err(|e| OmaAptError::FailedOperateDirOrFile(DPKG_STATUS_PATH.to_string(), e))?;
    }

    Ok(res)
}
