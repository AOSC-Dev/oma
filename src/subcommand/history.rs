use anyhow::anyhow;
use apt_auth_config::AuthConfig;
use chrono::{Local, LocalResult, TimeZone};
use clap::Args;
use dialoguer::{theme::ColorfulTheme, Select};
use oma_history::{
    connect_db, find_history_by_id, list_history, HistoryListEntry, SummaryType, DATABASE_PATH,
};
use oma_pm::apt::{AptConfig, InstallOperation, OmaAptArgs};
use oma_pm::matches::{GetArchMethod, PackagesMatcher};
use oma_pm::pkginfo::PtrIsNone;
use oma_pm::{
    apt::{FilterMode, OmaApt},
    pkginfo::OmaPackage,
};

use std::path::{Path, PathBuf};
use std::{borrow::Cow, sync::atomic::Ordering};

use crate::config::Config;
use crate::HTTP_CLIENT;
use crate::{
    error::OutputError,
    table::table_for_history_pending,
    utils::{dbus_check, root},
    ALLOWCTRLC,
};

use super::utils::{
    handle_no_result, lock_oma, no_check_dbus_warn, select_tui_display_msg, tui_select_list_size,
    CommitChanges,
};
use crate::args::CliExecuter;

#[derive(Debug, Args)]
pub struct History {
    /// Set sysroot target directory
    #[arg(from_global)]
    sysroot: PathBuf,
}

impl CliExecuter for History {
    fn execute(self, _config: &Config, _no_progress: bool) -> Result<i32, OutputError> {
        let History { sysroot } = self;
        let conn = connect_db(sysroot.join(DATABASE_PATH), false)?;

        let list = list_history(&conn)?;
        let display_list = format_summary_log(&list, false)
            .into_iter()
            .map(|x| x.0)
            .collect::<Vec<_>>();

        ALLOWCTRLC.store(true, Ordering::Relaxed);

        let mut old_selected = 0;

        loop {
            let selected =
                dialoguer_select_history(&display_list, old_selected).map_err(|_| anyhow!(""))?;
            old_selected = selected;

            let selected = &list[selected];
            let id = selected.id;
            let op = find_history_by_id(&conn, id)?;
            let install = &op.install;
            let remove = &op.remove;
            let disk_size = &op.disk_size;

            table_for_history_pending(install, remove, disk_size)?;
        }
    }
}

#[derive(Debug, Args)]
pub struct Undo {
    /// Do not fix apt broken status
    #[arg(short, long)]
    no_fixbroken: bool,
    /// Install package(s) without fsync(2)
    #[arg(long)]
    force_unsafe_io: bool,
    /// Ignore repository and package dependency issues
    #[arg(long)]
    force_yes: bool,
    /// Replace configuration file(s) in the system those shipped in the package(s) to be installed (invokes `dpkg --force-confnew`)
    #[arg(long)]
    force_confnew: bool,
    /// Auto remove unnecessary package(s)
    #[arg(long)]
    autoremove: bool,
    /// Remove package(s) also remove configuration file(s), like apt purge
    #[arg(long, visible_alias = "purge")]
    remove_config: bool,
    /// Run oma in “dry-run” mode. Useful for testing changes and operations without making changes to the system
    #[arg(from_global)]
    dry_run: bool,
    /// Run oma do not check dbus
    #[arg(from_global)]
    no_check_dbus: bool,
    /// Set sysroot target directory
    #[arg(from_global)]
    sysroot: PathBuf,
    /// Set apt options
    #[arg(from_global)]
    apt_options: Vec<String>,
}

impl CliExecuter for Undo {
    fn execute(self, config: &Config, no_progress: bool) -> Result<i32, OutputError> {
        root()?;
        lock_oma()?;

        let Undo {
            no_fixbroken,
            force_unsafe_io,
            force_yes,
            force_confnew,
            autoremove,
            remove_config,
            dry_run,
            no_check_dbus,
            sysroot,
            apt_options,
        } = self;

        let _fds = if !no_check_dbus && !config.no_check_dbus() {
            Some(dbus_check(false)?)
        } else {
            no_check_dbus_warn();
            None
        };

        let conn = connect_db(Path::new(&sysroot).join(DATABASE_PATH), false)?;

        let list = list_history(&conn)?;
        let display_list = format_summary_log(&list, true);
        let selected = dialoguer_select_history(
            &display_list
                .clone()
                .into_iter()
                .map(|x| x.0)
                .collect::<Vec<_>>(),
            0,
        )?;

        let selected = &list[display_list[selected].1];
        let id = selected.id;
        let op = find_history_by_id(&conn, id)?;

        let oma_apt_args = OmaAptArgs::builder()
            .sysroot(sysroot.to_string_lossy().to_string())
            .another_apt_options(apt_options)
            .dpkg_force_confnew(force_confnew)
            .dpkg_force_unsafe_io(force_unsafe_io)
            .force_yes(force_yes)
            .build();

        let mut apt = OmaApt::new(vec![], oma_apt_args, false, AptConfig::new())?;

        let mut glob = vec![];
        let mut install = vec![];

        if !op.install.is_empty() {
            for i in &op.install {
                match i.op() {
                    InstallOperation::Default | InstallOperation::Download => unreachable!(),
                    InstallOperation::Install => glob.push(i.name()),
                    InstallOperation::ReInstall => continue,
                    InstallOperation::Upgrade => install.push((i.name(), i.old_version().unwrap())),
                    InstallOperation::Downgrade => {
                        install.push((i.name(), i.old_version().unwrap()))
                    }
                }
            }
        }

        if !op.remove.is_empty() {
            for i in &op.remove {
                if let Some(ver) = i.version() {
                    install.push((i.name(), ver));
                }
            }
        }

        let matcher = PackagesMatcher::builder()
            .cache(&apt.cache)
            .native_arch(GetArchMethod::SpecifySysroot(&sysroot))
            .build();

        let mut delete = vec![];
        let mut no_result = vec![];
        for i in glob {
            let res = matcher.match_pkgs_from_glob(i)?;
            if res.is_empty() {
                no_result.push(i);
            } else {
                delete.extend(res);
            }
        }

        handle_no_result(&sysroot, no_result, no_progress)?;

        apt.remove(delete, false, true)?;

        let pkgs = apt.filter_pkgs(&[FilterMode::Default])?.collect::<Vec<_>>();

        let install = install
            .iter()
            .filter_map(|(pkg, ver)| {
                let pkg = pkgs.iter().find(move |y| &y.name() == pkg);

                if let Some(pkg) = pkg {
                    Some((pkg, pkg.get_version(ver)?))
                } else {
                    None
                }
            })
            .map(|(x, y)| OmaPackage::new(&y, x))
            .collect::<Result<Vec<OmaPackage>, PtrIsNone>>()
            .map_err(|e| OutputError {
                description: e.to_string(),
                source: None,
            })?;

        apt.install(&install, false)?;

        let auth_config = AuthConfig::system(&sysroot)?;

        CommitChanges::builder()
            .apt(apt)
            .dry_run(dry_run)
            .request_type(SummaryType::Undo)
            .no_fixbroken(no_fixbroken)
            .network_thread(config.network_thread())
            .no_progress(no_progress)
            .sysroot(sysroot.to_string_lossy().to_string())
            .fix_dpkg_status(true)
            .protect_essential(config.protect_essentials())
            .client(&HTTP_CLIENT)
            .yes(false)
            .remove_config(remove_config)
            .auth_config(&auth_config)
            .autoremove(autoremove)
            .build()
            .run()
    }
}

fn dialoguer_select_history(
    display_list: &[String],
    old_selected: usize,
) -> Result<usize, OutputError> {
    let page_size = tui_select_list_size();

    let selected = Select::with_theme(&ColorfulTheme::default())
        .items(display_list)
        .default(old_selected)
        .max_length(page_size.into())
        .interact()
        .map_err(|_| anyhow!(""))?;

    Ok(selected)
}

fn format_summary_log(list: &[HistoryListEntry], undo: bool) -> Vec<(String, usize)> {
    let display_list = list
        .iter()
        .enumerate()
        .filter(|(_, log)| {
            if undo {
                log.t != SummaryType::FixBroken && log.t != SummaryType::Undo
            } else {
                true
            }
        })
        .map(|(index, log)| {
            let date = format_date(log.time);
            let s = match &log.t {
                SummaryType::Install(v) if v.len() > 3 => format!(
                    "{}Installed {} ... (and {} more) [{}]",
                    format_success(log.is_success),
                    v[..3].join(" "),
                    v.len() - 3,
                    date
                ),
                SummaryType::Install(v) => format!(
                    "{}Installed {} [{date}]",
                    format_success(log.is_success),
                    v.join(" "),
                ),
                SummaryType::Upgrade(v) if v.is_empty() => {
                    format!("Upgraded system [{date}]")
                }
                SummaryType::Upgrade(v) if v.len() > 3 => format!(
                    "{}Upgraded system and installed {}... (and {} more) [{date}]",
                    format_success(log.is_success),
                    v[..3].join(" "),
                    v.len() - 3
                ),
                SummaryType::Upgrade(v) => format!(
                    "{}Upgraded system and install {} [{date}]",
                    format_success(log.is_success),
                    v.join(" "),
                ),
                SummaryType::Remove(v) if v.len() > 3 => format!(
                    "{}Removed {} ... (and {} more)",
                    format_success(log.is_success),
                    v[..3].join(" "),
                    v.len() - 3
                ),
                SummaryType::Remove(v) => format!("Removed {} [{date}]", v.join(" ")),
                SummaryType::FixBroken => format!("Attempted to fix broken dependencies [{date}]"),
                SummaryType::TopicsChanged { add, remove } if remove.is_empty() => {
                    format!(
                        "{}Topics changed: enabled {}{} [{date}]",
                        format_success(log.is_success),
                        if add.len() <= 3 {
                            add.join(" ")
                        } else {
                            add[..3].join(" ")
                        },
                        if add.len() <= 3 {
                            Cow::Borrowed("")
                        } else {
                            Cow::Owned(format!(" ... (and {} more)", add.len() - 3))
                        }
                    )
                }
                SummaryType::TopicsChanged { add, remove } if add.is_empty() => {
                    format!(
                        "{}Topics changed: disabled {}{} [{date}]",
                        format_success(log.is_success),
                        if remove.len() <= 3 {
                            add.join(" ")
                        } else {
                            remove[..3].join(" ")
                        },
                        if remove.len() <= 3 {
                            Cow::Borrowed("")
                        } else {
                            Cow::Owned(format!(" ... (and {} more)", remove.len() - 3))
                        }
                    )
                }
                SummaryType::TopicsChanged { add, remove } => {
                    format!(
                        "{}Topics changed: enabled {}{}, disabled {}{} [{date}]",
                        format_success(log.is_success),
                        if add.len() <= 3 {
                            add.join(" ")
                        } else {
                            add[..3].join(" ")
                        },
                        if add.len() <= 3 {
                            Cow::Borrowed("")
                        } else {
                            Cow::Owned(format!(" ... (and {} more)", add.len() - 3))
                        },
                        if remove.len() <= 3 {
                            remove.join(" ")
                        } else {
                            remove[..3].join(" ")
                        },
                        if remove.len() <= 3 {
                            Cow::Borrowed("")
                        } else {
                            Cow::Owned(format!(" ... (and {} more)", add.len() - 3))
                        },
                    )
                }
                SummaryType::Undo => format!("Undone [{date}]"),
                SummaryType::Changes => "Change packages".to_string(),
            };

            let s = select_tui_display_msg(&s, false).to_string();

            (s, index)
        })
        .collect::<Vec<_>>();

    display_list
}

fn format_date(date: i64) -> String {
    let dt = match Local.timestamp_opt(date, 0) {
        LocalResult::None => Local.timestamp_opt(0, 0).unwrap(),
        x => x.unwrap(),
    };

    let s = dt.format("%H:%M:%S on %Y-%m-%d").to_string();

    s
}

fn format_success(is_success: bool) -> &'static str {
    if is_success {
        ""
    } else {
        "[FAIL] "
    }
}
