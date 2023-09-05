use std::borrow::Cow;

use dialoguer::console::style;
use oma_console::info;
use oma_pm::apt::{FilterMode, OmaApt, OmaAptArgsBuilder};

use crate::error::OutputError;
use crate::fl;
use anyhow::anyhow;

pub fn execute(
    all: bool,
    installed: bool,
    upgradable: bool,
    pkgs: Vec<String>,
) -> Result<i32, OutputError> {
    let oma_apt_args = OmaAptArgsBuilder::default().build()?;
    let apt = OmaApt::new(vec![], oma_apt_args, false)?;

    let mut filter_mode = vec![];
    if installed {
        filter_mode.push(FilterMode::Installed);
    }
    if upgradable {
        filter_mode.push(FilterMode::Upgradable)
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
                        pkg.candidate().map(|x| x.version().to_string())
                            != Some(x.version().to_string())
                    })
                    .collect::<Vec<_>>()
                    .len();

                if other_version > 0 {
                    display_tips = (true, other_version);
                }
            }

            vec![pkg
                .candidate()
                .ok_or_else(|| anyhow!(fl!("no-candidate-ver", pkg = name)))?]
        };

        for version in &versions {
            let mut branches = vec![];

            let pkg_files = version.package_files();

            let mut installed = false;

            for pkg_file in pkg_files {
                let branch = pkg_file.archive().unwrap_or("unknown").to_string();
                branches.push(Cow::Owned(branch.clone()));

                if let Some(inst) = pkg.installed() {
                    let mut inst_pkg_files = inst.package_files();
                    installed = inst_pkg_files
                        .any(|x| x.archive().map(|x| x == branch).unwrap_or(false))
                        && inst.version() == version.version();
                }
            }

            if branches.is_empty() {
                branches.push(Cow::Borrowed("unknown"));
            }

            let branches = branches.join(",");
            let version_str = version.version();
            let arch = version.arch();

            let upgradable = pkg.is_upgradable();
            let automatic = pkg.is_auto_installed();

            let mut s = vec![];

            if installed {
                s.push("installed");
            }

            if upgradable && installed {
                s.push("upgradable");
            }

            if automatic {
                s.push("automatc");
            }

            let s = if s.is_empty() {
                Cow::Borrowed("")
            } else {
                Cow::Owned(format!("[{}]", s.join(",")))
            };

            println!(
                "{}/{branches} {version_str} {arch} {s}",
                style(name).green().bold()
            );
        }
    }

    if display_tips.0 && pkgs.len() == 1 {
        info!("{}", fl!("additional-version", len = display_tips.1));
    }

    Ok(0)
}
