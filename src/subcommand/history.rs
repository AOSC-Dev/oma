use anyhow::anyhow;
use chrono::format::{DelayedFormat, StrftimeItems};
use chrono::{Local, LocalResult, TimeZone};
use clap::Args;
use dialoguer::{Select, theme::ColorfulTheme};
use oma_history::{DATABASE_PATH, HistoryEntry, connect_db, find_history_by_id, list_history};
use oma_pm::apt::{AptConfig, InstallOperation, OmaAptArgs};
use oma_pm::matches::{GetArchMethod, PackagesMatcher};
use oma_pm::oma_apt::PackageSort;
use oma_pm::pkginfo::PtrIsNone;
use oma_pm::{apt::OmaApt, pkginfo::OmaPackage};
use spdlog::warn;
use std::sync::atomic::Ordering;

use crate::HTTP_CLIENT;
use crate::config::OmaConfig;
use crate::utils::ExitHandle;
#[cfg(feature = "aosc")]
use crate::utils::ExitStatus;
use crate::{
    NOT_DISPLAY_ABORT,
    error::OutputError,
    fl,
    table::table_for_history_pending,
    utils::{dbus_check, root},
};

use super::utils::{
    CommitChanges, Refresh, auth_config, handle_no_result, lock_oma, select_tui_display_msg,
    tui_select_list_size,
};
use crate::args::CliExecuter;

#[derive(Debug, Args)]
pub struct History;

impl CliExecuter for History {
    fn execute(self, config: OmaConfig) -> Result<ExitHandle, OutputError> {
        let conn = connect_db(config.sysroot.join(DATABASE_PATH), false)?;

        let list = list_history(&conn)?;
        let display_list = format_summary_log(&list, false)
            .into_iter()
            .map(|x| x.0)
            .collect::<Vec<_>>();

        NOT_DISPLAY_ABORT.store(true, Ordering::Relaxed);

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
            let disk_size = op.disk_size;

            table_for_history_pending(install, remove, disk_size)?;
        }
    }
}

#[derive(Debug, Args)]
pub struct Undo {
    /// Do not fix apt broken status
    #[arg(long, help = fl!("clap-no-fixbroken-help"))]
    no_fixbroken: bool,
    /// Do not fix dpkg broken status
    #[arg(long, help = fl!("clap-no-fix-dpkg-status-help"))]
    no_fix_dpkg_status: bool,
    /// Install package(s) without fsync(2)
    #[arg(
        long,
        help = &**crate::args::FORCE_UNSAGE_IO_TRANSLATE
    )]
    force_unsafe_io: bool,
    /// Ignore repository and package dependency issues
    #[arg(long, help = fl!("clap-force-yes-help"))]
    force_yes: bool,
    /// Replace configuration file(s) in the system those shipped in the package(s) to be installed (invokes `dpkg --force-confnew`)
    #[arg(long, help = fl!("clap-force-confnew-help"))]
    force_confnew: bool,
    /// Auto remove unnecessary package(s)
    #[arg(long, help = fl!("clap-autoremove-help"))]
    autoremove: bool,
    /// Remove package(s) also remove configuration file(s), like apt purge
    #[arg(long, visible_alias = "purge", help = fl!("clap-remove-config-help"))]
    remove_config: bool,
    /// Only download dependencies, not install
    #[arg(long, short, help = fl!("clap-download-only-help"))]
    download_only: bool,
    /// Do not refresh repository metadata
    #[arg(long, help = fl!("clap-no-refresh-help"))]
    no_refresh: bool,
}

impl CliExecuter for Undo {
    fn execute(self, config: OmaConfig) -> Result<ExitHandle, OutputError> {
        root()?;

        let Undo {
            no_fixbroken,
            force_unsafe_io,
            force_yes,
            force_confnew,
            autoremove,
            remove_config,
            no_fix_dpkg_status,
            no_refresh,
            download_only,
        } = self;

        lock_oma(&config.sysroot)?;

        let _fds = dbus_check(false, &config)?;

        let conn = connect_db(config.sysroot.join(DATABASE_PATH), false)?;

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

        #[cfg(feature = "aosc")]
        let (opt_in, opt_out) = oma_history::find_history_topics_status_by_id(&conn, id)?;

        let apt_config = AptConfig::new();
        let auth_config = auth_config(&config.sysroot);

        if !no_refresh {
            let sysroot = config.sysroot.to_string_lossy();
            let builder = Refresh::builder()
                .client(&HTTP_CLIENT)
                .dry_run(config.dry_run)
                .no_progress(config.no_progress())
                .network_thread(config.download_threads)
                .sysroot(&sysroot)
                .config(&apt_config)
                .maybe_auth_config(auth_config.as_ref())
                .apt_options(config.apt_options.clone());

            #[cfg(feature = "aosc")]
            let refresh = builder.refresh_topics(config.no_refresh_topics).build();

            #[cfg(not(feature = "aosc"))]
            let refresh = builder.build();

            refresh.run()?;
        }

        let no_progress = config.no_progress();

        let oma_apt_args = OmaAptArgs::builder()
            .sysroot(config.sysroot.to_string_lossy().to_string())
            .another_apt_options(config.apt_options)
            .dpkg_force_confnew(force_confnew)
            .dpkg_force_unsafe_io(force_unsafe_io)
            .force_yes(force_yes)
            .build();

        let mut apt = OmaApt::new(vec![], oma_apt_args, false, AptConfig::new())?;

        let mut remove = vec![];
        let mut install = vec![];

        if !op.install.is_empty() {
            for i in &op.install {
                match i.operation {
                    InstallOperation::Default | InstallOperation::Download => unreachable!(),
                    InstallOperation::Install => remove.push(&i.pkg_name),
                    InstallOperation::ReInstall => continue,
                    InstallOperation::Upgrade | InstallOperation::Downgrade => {
                        install.push((&i.pkg_name, i.old_version.as_ref().unwrap()))
                    }
                }
            }
        }

        if !op.remove.is_empty() {
            for i in &op.remove {
                install.push((&i.pkg_name, &i.version));
            }
        }

        let matcher = PackagesMatcher::builder()
            .cache(&apt.cache)
            .native_arch(GetArchMethod::SpecifySysroot(&config.sysroot))
            .build();

        let mut delete = vec![];
        let mut no_result = vec![];
        for i in remove {
            let res = matcher.match_pkgs_from_glob(i)?;
            if res.is_empty() {
                no_result.push(i.as_str());
            } else {
                delete.extend(res);
            }
        }

        handle_no_result(no_result, no_progress)?;

        apt.remove(delete, false, true)?;

        let pkgs = apt
            .cache
            .packages(&PackageSort::default())
            .collect::<Vec<_>>();

        let install = install
            .iter()
            .filter_map(|(pkg, ver)| {
                let select = pkgs.iter().find(move |y| y.name() == *pkg);

                if let Some(pkg) = select {
                    if let Some(v) = pkg.get_version(ver) {
                        Some((pkg, v))
                    } else {
                        warn!("Failed to get package {} version: {}", pkg, ver);
                        None
                    }
                } else {
                    warn!("Failed to get package: {} {}", pkg, ver);
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

        let exit = CommitChanges::builder()
            .apt(apt)
            .dry_run(config.dry_run)
            .is_undo(true)
            .no_fixbroken(no_fixbroken)
            .no_progress(no_progress)
            .sysroot(config.sysroot.to_string_lossy().to_string())
            .fix_dpkg_status(!no_fix_dpkg_status)
            .protect_essential(config.protect_essentials)
            .yes(false)
            .remove_config(remove_config)
            .autoremove(autoremove)
            .network_thread(config.download_threads)
            .maybe_auth_config(auth_config.as_ref())
            .download_only(download_only)
            .build()
            .run()?;

        #[cfg(feature = "aosc")]
        if exit.get_status() == ExitStatus::Success && (!opt_in.is_empty() || !opt_out.is_empty()) {
            use crate::RT;
            use crate::fl;
            use spdlog::warn;

            let arch = oma_utils::dpkg::dpkg_arch(&config.sysroot)?;
            let mut tm = oma_topics::TopicManager::new_blocking(
                &crate::HTTP_CLIENT,
                config.sysroot,
                &arch,
                config.dry_run,
            )?;

            RT.block_on(tm.refresh())?;

            for i in opt_in {
                if let Err(e) = tm.remove(&i) {
                    warn!("Could not disable topic {i}: {e}");
                }
            }

            for i in opt_out {
                if let Err(e) = tm.add(&i) {
                    warn!("Could not enable topic {i}: {e}");
                }
            }

            RT.block_on(tm.write_enabled(false))?;
            RT.block_on(tm.write_sources_list(
                &fl!("do-not-edit-topic-sources-list"),
                false,
                async |topic, mirror| {
                    warn!(
                        "{}",
                        fl!("topic-not-in-mirror", topic = topic, mirror = mirror)
                    );
                    warn!("{}", fl!("skip-write-mirror"));
                },
            ))?;
        }

        Ok(exit)
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

fn format_summary_log(list: &[HistoryEntry], undo: bool) -> Vec<(String, usize)> {
    list.iter()
        .enumerate()
        .filter(|(_, log)| {
            if undo {
                !log.is_fixbroken && !log.is_undo
            } else {
                true
            }
        })
        .map(|(index, log)| {
            let date = format_date(log.time);
            let command = &log.command;

            let s = format!("{}[{}] {}", format_success(log.is_success), date, command);
            let s = select_tui_display_msg(&s, false).to_string();

            (s, index)
        })
        .collect::<Vec<_>>()
}

fn format_date(date: i64) -> DelayedFormat<StrftimeItems<'static>> {
    let dt = match Local.timestamp_opt(date, 0) {
        LocalResult::None => Local.timestamp_opt(0, 0).unwrap(),
        x => x.unwrap(),
    };

    dt.format("%H:%M:%S on %Y-%m-%d")
}

fn format_success(is_success: bool) -> &'static str {
    if is_success { "" } else { "[FAIL] " }
}
