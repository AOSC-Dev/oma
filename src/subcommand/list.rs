use std::{borrow::Cow, io::stdout, sync::atomic::Ordering};

use oma_console::print::Action;
use oma_pm::{
    apt::{AptConfig, FilterMode, OmaApt, OmaAptArgs},
    PkgCurrentState,
};
use tracing::info;

use crate::{color_formatter, error::OutputError, table::PagerPrinter};
use crate::{fl, ALLOWCTRLC};
use anyhow::anyhow;
use smallvec::{smallvec, SmallVec};

pub struct ListFlags {
    pub all: bool,
    pub installed: bool,
    pub upgradable: bool,
    pub manual: bool,
    pub auto: bool,
    pub autoremovable: bool,
}

pub fn execute(
    flags: ListFlags,
    pkgs: Vec<String>,
    sysroot: String,
    json: bool,
    another_apt_options: Vec<String>,
) -> Result<i32, OutputError> {
    let ListFlags {
        all,
        installed,
        upgradable,
        manual,
        auto,
        autoremovable,
    } = flags;

    let oma_apt_args = OmaAptArgs::builder()
        .sysroot(sysroot)
        .another_apt_options(another_apt_options)
        .build();

    let apt = OmaApt::new(vec![], oma_apt_args, false, AptConfig::new())?;

    let mut filter_mode: SmallVec<[_; 5]> = smallvec![FilterMode::Names];

    if installed {
        filter_mode.push(FilterMode::Installed);
    }

    if upgradable {
        filter_mode.push(FilterMode::Upgradable)
    }

    if auto {
        filter_mode.push(FilterMode::Automatic);
    }

    if manual {
        filter_mode.push(FilterMode::Manual);
    }

    if autoremovable {
        filter_mode.push(FilterMode::AutoRemovable);
    }

    let filter_pkgs = apt.filter_pkgs(&filter_mode)?;
    let filter_pkgs: Box<dyn Iterator<Item = _>> = if pkgs.is_empty() {
        Box::new(filter_pkgs)
    } else {
        Box::new(filter_pkgs.filter(|x| {
            for i in &pkgs {
                if glob_match::glob_match(i, x.name()) {
                    return true;
                }
            }

            false
        }))
    };

    let mut printer = PagerPrinter::new(stdout());
    ALLOWCTRLC.store(true, Ordering::Relaxed);

    let mut display_tips = (false, 0);

    let mut pkg_count = 0;

    for pkg in filter_pkgs {
        let name = pkg.fullname(true);
        pkg_count += 1;
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
                    .ok_or_else(|| anyhow!("Has Package {} but no version?", pkg.fullname(true)))?]
            }
        };

        for version in &versions {
            let mut branches = vec![];

            let pkg_files = version.package_files();

            let mut installed = false;

            for pkg_file in pkg_files {
                let branch = pkg_file.archive();
                let branch = match branch {
                    Some(branch) => Box::from(branch),
                    None => "unknown".into(),
                };

                if let Some(inst) = pkg.installed() {
                    let mut inst_pkg_files = inst.package_files();
                    installed = inst_pkg_files
                        .any(|x| x.archive().map(|x| x == &*branch).unwrap_or(false))
                        && inst.version() == version.version();
                }

                branches.push(branch);
            }

            if branches.is_empty() {
                branches.push("unknown".into());
            }

            let mut version_str = Cow::Borrowed(version.version());
            let arch = version.arch();

            let upgradable = pkg.is_upgradable();
            let automatic = pkg.is_auto_installed();

            let mut status = vec![];

            if installed {
                status.push("installed");
            }

            let mut new_version: Option<Box<str>> = None;

            if upgradable && installed {
                status.push("upgradable");
                new_version = pkg.candidate().map(|x| Box::from(x.version()));
            }

            if automatic {
                status.push("automatic");
            }

            if pkg.current_state() == PkgCurrentState::ConfigFiles {
                status.push("residual-config")
            }

            if !json {
                let s = if status.is_empty() {
                    Cow::Borrowed("")
                } else {
                    Cow::Owned(format!("[{}]", status.join(",")))
                };

                let branches_str = branches.join(",");

                printer
                    .print(format!(
                        "{}/{} {} {arch} {s}",
                        color_formatter().color_str(&name, Action::Emphasis).bold(),
                        color_formatter().color_str(branches_str, Action::Secondary),
                        if upgradable {
                            version_str = Cow::Owned(format!(
                                "{} -> {}",
                                version_str,
                                new_version.as_ref().unwrap(),
                            ));
                            color_formatter().color_str(version_str, Action::WARN)
                        } else {
                            color_formatter().color_str(version_str, Action::EmphasisSecondary)
                        }
                    ))
                    .ok();
            } else {
                printer
                    .print(serde_json::json!(
                        {
                            "name": name,
                            "branches": branches,
                            "current_version": version.version(),
                            "new_version": new_version,
                            "architecture": arch,
                            "status": status,
                        }
                    ))
                    .ok();
            }
        }
    }

    if display_tips.0 && pkg_count == 1 && !json {
        info!("{}", fl!("additional-version", len = display_tips.1));
    }

    Ok(0)
}
