use std::borrow::Cow;

use dialoguer::console::style;
use oma_pm::apt::{FilterMode, OmaApt, OmaAptArgsBuilder};
use tracing::info;

use crate::error::OutputError;
use crate::fl;
use anyhow::anyhow;
use smallvec::{smallvec, SmallVec};

use super::utils::check_unsupport_stmt;

pub fn execute(
    all: bool,
    installed: bool,
    upgradable: bool,
    pkgs: Vec<String>,
    sysroot: String,
) -> Result<i32, OutputError> {
    let oma_apt_args = OmaAptArgsBuilder::default().sysroot(sysroot).build()?;
    let apt = OmaApt::new(vec![], oma_apt_args, false)?;

    let mut filter_mode: SmallVec<[_; 4]> = smallvec![FilterMode::Names];

    if installed {
        filter_mode.push(FilterMode::Installed);
    }

    if upgradable {
        filter_mode.push(FilterMode::Upgradable)
    }

    if !pkgs.is_empty() {
        for pkg in &pkgs {
            check_unsupport_stmt(pkg);
        }
    }

    let filter_pkgs = apt.filter_pkgs(&filter_mode)?;
    let filter_pkgs: Box<dyn Iterator<Item = _>> = if pkgs.is_empty() {
        Box::new(filter_pkgs)
    } else {
        Box::new(filter_pkgs.filter(|x| pkgs.contains(&x.name().to_string())))
    };

    let mut display_tips = (false, 0);

    for pkg in filter_pkgs {
        let name = pkg.name();
        let versions = if all {
            pkg.versions().collect()
        } else {
            if !pkgs.is_empty() {
                let other_version = pkg
                    .versions()
                    .filter(|x| {
                        pkg.candidate().map(|x| Cow::Owned(x.version().to_string()))
                            != Some(Cow::Borrowed(x.version()))
                    })
                    .count();

                if other_version > 0 {
                    display_tips = (true, other_version);
                }
            }

            if let Some(version) = pkg.installed() {
                vec![version]
            } else {
                vec![pkg
                    .candidate()
                    .or_else(|| pkg.versions().next())
                    .ok_or_else(|| anyhow!("Has Package {} but no version?", pkg.name()))?]
            }
        };

        for version in &versions {
            let mut branches = vec![];

            let pkg_files = version.package_files();

            let mut installed = false;

            for pkg_file in pkg_files {
                let branch = pkg_file.archive();
                let branch = match branch {
                    Ok(branch) => Cow::Owned(branch.to_string()),
                    Err(_) => Cow::Borrowed("unknown"),
                };

                if let Some(inst) = pkg.installed() {
                    let mut inst_pkg_files = inst.package_files();
                    installed = inst_pkg_files
                        .any(|x| x.archive().map(|x| x == branch).unwrap_or(false))
                        && inst.version() == version.version();
                }

                branches.push(branch);
            }

            if branches.is_empty() {
                branches.push(Cow::Borrowed("unknown"));
            }

            let branches = branches.join(",");
            let mut version_str = Cow::Borrowed(version.version());
            let arch = version.arch();

            let upgradable = pkg.is_upgradable();
            let automatic = pkg.is_auto_installed();

            let mut s = vec![];

            if installed {
                s.push("installed");
            }

            if upgradable && installed {
                s.push("upgradable");
                version_str = Cow::Owned(format!(
                    "{} -> {}",
                    version_str,
                    pkg.candidate().map(|x| x.version().to_string()).unwrap()
                ));
            }

            if automatic {
                s.push("automatic");
            }

            if pkg.current_state() == 5 {
                s.push("residual-config")
            }

            let s = if s.is_empty() {
                Cow::Borrowed("")
            } else {
                Cow::Owned(format!("[{}]", s.join(",")))
            };

            println!(
                "{}/{} {} {arch} {s}",
                style(name).color256(148).bold(),
                style(branches).color256(182),
                if upgradable {
                    style(version_str).color256(214)
                } else {
                    style(version_str).color256(114)
                }
            );
        }
    }

    if display_tips.0 && pkgs.len() == 1 {
        info!("{}", fl!("additional-version", len = display_tips.1));
    }

    Ok(0)
}
