use chrono::Local;
use oma_console::success;
use oma_history::connect_db;
use oma_history::create_db_file;
use oma_history::write_history_entry;
use oma_history::SummaryType;
use oma_pm::apt::AptArgsBuilder;
use oma_pm::apt::OmaApt;
use oma_pm::apt::OmaAptArgsBuilder;
use oma_pm::apt::OmaAptError;
use reqwest::Client;
use tracing::info;
use tracing::warn;

use crate::error::OutputError;
use crate::fl;
use crate::pb;
use crate::table::table_for_install_pending;
use crate::utils::create_async_runtime;
use crate::utils::dbus_check;
use crate::utils::multibar;
use crate::utils::root;
use crate::OmaArgs;
use crate::UpgradeArgs;

use super::remove::ask_user_do_as_i_say;
use super::utils::check_empty_op;
use super::utils::handle_event_without_progressbar;
use super::utils::handle_no_result;
use super::utils::lock_oma;
use super::utils::no_check_dbus_warn;
use super::utils::refresh;

pub fn execute(
    pkgs_unparse: Vec<String>,
    args: UpgradeArgs,
    oma_args: OmaArgs,
    client: Client,
) -> Result<i32, OutputError> {
    root()?;
    lock_oma()?;

    let no_refresh_topics = args.no_refresh_topcs;

    let OmaArgs {
        dry_run,
        network_thread,
        no_progress,
        download_pure_db,
        no_check_dbus,
        protect_essentials,
    } = oma_args;

    let fds = if !no_check_dbus {
        let rt = create_async_runtime()?;
        Some(dbus_check(&rt, args.yes)?)
    } else {
        no_check_dbus_warn();
        None
    };

    refresh(
        &client,
        dry_run,
        no_progress,
        download_pure_db,
        network_thread,
        &args.sysroot,
        !no_refresh_topics,
    )?;

    if args.yes {
        warn!("{}", fl!("automatic-mode-warn"));
    }

    let local_debs = pkgs_unparse
        .iter()
        .filter(|x| x.ends_with(".deb"))
        .map(|x| x.to_owned())
        .collect::<Vec<_>>();

    let pkgs_unparse = pkgs_unparse.iter().map(|x| x.as_str()).collect::<Vec<_>>();
    let mut retry_times = 1;

    let apt_args = AptArgsBuilder::default()
        .dpkg_force_all(args.dpkg_force_all)
        .dpkg_force_confnew(args.force_confnew)
        .force_yes(args.force_yes)
        .yes(args.yes)
        .no_progress(no_progress)
        .build()?;

    let oma_apt_args = OmaAptArgsBuilder::default()
        .sysroot(args.sysroot.clone())
        .build()?;

    loop {
        let mut apt = OmaApt::new(local_debs.clone(), oma_apt_args.clone(), dry_run)?;
        apt.upgrade()?;

        let (pkgs, no_result) = apt.select_pkg(&pkgs_unparse, false, true, false)?;
        handle_no_result(no_result)?;

        apt.install(&pkgs, false)?;
        apt.resolve(false, true)?;

        let op = apt.summary(|pkg| {
            if protect_essentials {
                false
            } else {
                ask_user_do_as_i_say(pkg).unwrap_or(false)
            }
        })?;

        apt.check_disk_size(&op)?;

        let op_after = op.clone();

        let install = &op.install;
        let remove = &op.remove;
        let disk_size = &op.disk_size;

        if check_empty_op(install, remove) {
            return Ok(0);
        }

        if retry_times == 1 {
            table_for_install_pending(install, remove, disk_size, !args.yes, dry_run)?;
        }

        let typ = SummaryType::Upgrade(
            pkgs.iter()
                .map(|x| format!("{} {}", x.raw_pkg.name(), x.version_raw.version()))
                .collect::<Vec<_>>(),
        );

        let start_time = Local::now().timestamp();

        let (mb, pb_map, global_is_set) = multibar();
        match apt.commit(
            &client,
            None,
            &apt_args,
            |count, event, total| {
                if !no_progress {
                    pb!(event, mb, pb_map, count, total, global_is_set)
                } else {
                    handle_event_without_progressbar(event);
                }
            },
            op,
        ) {
            Ok(()) => {
                if let Some(gpb) = pb_map.get(&0) {
                    gpb.finish_and_clear();
                }
                write_history_entry(
                    op_after,
                    typ,
                    {
                        let db = create_db_file(args.sysroot)?;
                        connect_db(db, true)?
                    },
                    dry_run,
                    start_time,
                    true,
                )?;
                success!("{}", fl!("history-tips-1"));
                info!("{}", fl!("history-tips-2"));
                drop(fds);
                return Ok(0);
            }
            Err(e) => match e {
                OmaAptError::AptErrors(_)
                | OmaAptError::AptError(_)
                | OmaAptError::AptCxxException(_) => {
                    if let Some(gpb) = pb_map.get(&0) {
                        gpb.finish_and_clear();
                    }
                    if retry_times == 3 {
                        write_history_entry(
                            op_after,
                            SummaryType::Upgrade(
                                pkgs.iter()
                                    .map(|x| {
                                        format!("{} {}", x.raw_pkg.name(), x.version_raw.version())
                                    })
                                    .collect::<Vec<_>>(),
                            ),
                            {
                                let db = create_db_file(args.sysroot)?;
                                connect_db(db, true)?
                            },
                            dry_run,
                            start_time,
                            false,
                        )?;
                        info!("{}", fl!("history-tips-2"));
                        return Err(OutputError::from(e));
                    }
                    warn!("{e}, retrying ...");
                    retry_times += 1;
                }
                _ => {
                    if let Some(gpb) = pb_map.get(&0) {
                        gpb.finish_and_clear();
                    }
                    drop(fds);
                    return Err(OutputError::from(e));
                }
            },
        }
    }
}
