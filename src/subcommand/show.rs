use std::io::stdout;

use oma_pm::{
    apt::{AptConfig, OmaApt, OmaAptArgs},
    pkginfo::OmaPackage,
};
use tracing::info;

use crate::error::OutputError;

use super::utils::handle_no_result;
use crate::fl;

use std::io::Write;

pub fn execute(
    all: bool,
    input: Vec<&str>,
    sysroot: String,
    json: bool,
    another_apt_options: Vec<String>,
    no_progress: bool,
) -> Result<i32, OutputError> {
    let oma_apt_args = OmaAptArgs::builder()
        .another_apt_options(another_apt_options)
        .sysroot(sysroot.clone())
        .build();
    let mut apt = OmaApt::new(vec![], oma_apt_args, false, AptConfig::new())?;
    let (pkgs, no_result) = apt.select_pkg(&input, false, false, false)?;
    handle_no_result(sysroot, no_result, no_progress)?;

    let mut stdout = stdout();

    if !all {
        let mut filter_pkgs: Vec<OmaPackage> = vec![];
        let pkgs_len = pkgs.len();
        for pkg in pkgs {
            if !filter_pkgs
                .iter()
                .any(|x| pkg.raw_pkg.fullname(true) == x.raw_pkg.fullname(true))
            {
                filter_pkgs.push(pkg);
            }
        }

        if filter_pkgs.is_empty() {
            return Ok(1);
        }

        for (i, pkg) in filter_pkgs.iter().enumerate() {
            if json {
                writeln!(
                    stdout,
                    "{}",
                    serde_json::to_string(&pkg.pkg_info(&apt.cache)?).map_err(|e| OutputError {
                        description: e.to_string(),
                        source: None
                    })?
                )
                .ok();
            } else {
                writeln!(stdout, "{}", pkg.pkg_info(&apt.cache)?).ok();
                if i != filter_pkgs.len() - 1 {
                    writeln!(stdout).ok();
                }
            }
        }

        if filter_pkgs.len() == 1 && !json {
            let other_version = pkgs_len - 1;

            if other_version > 0 {
                info!("{}", fl!("additional-version", len = other_version));
            }
        }
    } else {
        for (i, pkg) in pkgs.iter().enumerate() {
            if json {
                writeln!(
                    stdout,
                    "{}",
                    serde_json::to_string(&pkg.pkg_info(&apt.cache)?).map_err(|e| OutputError {
                        description: e.to_string(),
                        source: None
                    })?
                )
                .ok();
            } else if i != pkgs.len() - 1 {
                writeln!(stdout, "{}", pkg.pkg_info(&apt.cache)?).ok();
                writeln!(stdout).ok();
            } else {
                writeln!(stdout, "{}", pkg.pkg_info(&apt.cache)?).ok();
            }
        }
    }

    Ok(0)
}
