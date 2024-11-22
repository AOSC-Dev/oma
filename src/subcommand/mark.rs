use std::borrow::Cow;

use oma_console::{print::Action, success};
use oma_pm::{
    apt::{AptConfig, OmaApt, OmaAptArgs},
    matches::{GetArchMethod, PackagesMatcher},
};
use tracing::info;

use crate::{color_formatter, error::OutputError, utils::root};

use super::utils::handle_no_result;
use crate::fl;

pub fn execute(
    op: &str,
    pkgs: Vec<String>,
    dry_run: bool,
    sysroot: String,
    another_apt_options: Vec<String>,
    no_progress: bool,
) -> Result<i32, OutputError> {
    root()?;

    let oma_apt_args = OmaAptArgs::builder()
        .sysroot(sysroot.clone())
        .another_apt_options(another_apt_options)
        .build();

    let apt = OmaApt::new(vec![], oma_apt_args, false, AptConfig::new())?;

    let set = match op {
        "hold" | "unhold" => apt
            .mark_version_status(&pkgs, op == "hold", dry_run)?
            .into_iter()
            .map(|(x, y)| (Cow::Borrowed(x), y))
            .collect::<Vec<_>>(),
        "auto" | "manual" => {
            let matcher = PackagesMatcher::builder()
                .cache(&apt.cache)
                .native_arch(GetArchMethod::SpecifySysroot(&sysroot))
                .build();

            let (pkgs, no_result) =
                matcher.match_pkgs_and_versions(pkgs.iter().map(|x| x.as_str()))?;

            handle_no_result(sysroot, no_result, no_progress)?;

            apt.mark_install_status(pkgs, op == "auto", dry_run)?
                .into_iter()
                .map(|(x, y)| (Cow::Owned(x), y))
                .collect()
        }
        _ => unreachable!(),
    };

    for (pkg, is_set) in set {
        let pkg = color_formatter()
            .color_str(pkg, Action::Emphasis)
            .to_string();

        match op {
            "hold" if is_set => {
                success!("{}", fl!("set-to-hold", name = pkg));
            }
            "hold" => {
                info!("{}", fl!("already-hold", name = pkg));
            }
            "unhold" if is_set => {
                success!("{}", fl!("set-to-unhold", name = pkg));
            }
            "unhold" => {
                info!("{}", fl!("already-unhold", name = pkg));
            }
            "auto" if is_set => {
                success!("{}", fl!("setting-auto", name = pkg));
            }
            "auto" => {
                info!("{}", fl!("already-auto", name = pkg));
            }
            "manual" if is_set => {
                success!("{}", fl!("setting-manual", name = pkg));
            }
            "manual" => {
                info!("{}", fl!("already-manual", name = pkg));
            }
            _ => unreachable!(),
        };
    }

    Ok(0)
}
