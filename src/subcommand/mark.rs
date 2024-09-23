use std::borrow::Cow;

use oma_console::success;
use oma_pm::apt::{AptConfig, OmaApt, OmaAptArgs};
use tracing::info;

use crate::{error::OutputError, utils::root};

use super::utils::handle_no_result;
use crate::fl;

pub fn execute(
    op: &str,
    pkgs: Vec<String>,
    dry_run: bool,
    sysroot: String,
) -> Result<i32, OutputError> {
    root()?;

    let oma_apt_args = OmaAptArgs::builder().sysroot(sysroot.clone()).build();
    let mut apt = OmaApt::new(vec![], oma_apt_args, false, AptConfig::new())?;

    let set = match op {
        "hold" | "unhold" => apt
            .mark_version_status(&pkgs, op == "hold", dry_run)?
            .into_iter()
            .map(|(x, y)| (Cow::Borrowed(x), y))
            .collect::<Vec<_>>(),
        "auto" | "manual" => {
            let (pkgs, no_result) = apt.select_pkg(
                &pkgs.iter().map(|x| x.as_str()).collect::<Vec<_>>(),
                false,
                true,
                false,
            )?;

            handle_no_result(sysroot, no_result)?;

            apt.mark_install_status(pkgs, op == "auto", dry_run)?
                .into_iter()
                .map(|(x, y)| (Cow::Owned(x), y))
                .collect()
        }
        _ => unreachable!(),
    };

    for (pkg, is_set) in set {
        match op {
            "hold" if is_set => {
                success!("{}", fl!("set-to-hold", name = pkg.to_string()));
            }
            "hold" => {
                info!("{}", fl!("already-hold", name = pkg.to_string()));
            }
            "unhold" if is_set => {
                success!("{}", fl!("set-to-unhold", name = pkg.to_string()));
            }
            "unhold" => {
                info!("{}", fl!("already-unhold", name = pkg.to_string()));
            }
            "auto" if is_set => {
                success!("{}", fl!("setting-auto", name = pkg.to_string()));
            }
            "auto" => {
                info!("{}", fl!("already-auto", name = pkg.to_string()));
            }
            "manual" if is_set => {
                success!("{}", fl!("setting-manual", name = pkg.to_string()));
            }
            "manual" => {
                info!("{}", fl!("already-manual", name = pkg.to_string()));
            }
            _ => unreachable!(),
        };
    }

    Ok(0)
}
