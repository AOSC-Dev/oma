use std::path::PathBuf;

use crate::error::OutputError;
use crate::fl;
use crate::pb;
use crate::table::table_for_install_pending;
use crate::utils::create_async_runtime;
use crate::utils::multibar;
use chrono::Local;
use dialoguer::console::style;
use oma_console::success;
use oma_console::writer::bar_writeln;
use oma_console::WRITER;
use oma_fetch::DownloadEvent;
use oma_history::connect_or_create_db;
use oma_history::write_history_entry;
use oma_history::SummaryType;
use oma_pm::apt::AptArgs;
use oma_pm::apt::OmaApt;
use oma_pm::operation::InstallEntry;
use oma_pm::operation::RemoveEntry;
use oma_refresh::db::OmaRefreshBuilder;
use oma_refresh::db::RefreshEvent;
use oma_utils::dpkg::dpkg_arch;
use tracing::error;
use tracing::info;
use tracing::warn;

pub(crate) fn handle_no_result(no_result: Vec<String>) {
    for word in no_result {
        if word == "266" {
            error!("无法找到匹配关键字为艾露露的软件包");
        } else {
            error!("{}", fl!("could-not-find-pkg-from-keyword", c = word));
        }
    }
}

pub(crate) fn refresh(
    dry_run: bool,
    no_progress: bool,
    download_pure_db: bool,
    sysroot: &str,
) -> Result<(), OutputError> {
    if dry_run {
        return Ok(());
    }

    info!("{}", fl!("refreshing-repo-metadata"));

    let download_pure_db = if dpkg_arch(sysroot)
        .map(|x| x == "mips64r6el")
        .unwrap_or(false)
    {
        false
    } else {
        download_pure_db
    };

    let sysroot = PathBuf::from(sysroot);

    let refresh = OmaRefreshBuilder::default()
        .source(sysroot.clone())
        .download_dir(sysroot.join("var/lib/apt/lists"))
        .download_compress(!download_pure_db)
        .arch(dpkg_arch(&sysroot)?)
        .build()
        .unwrap();

    let tokio = create_async_runtime()?;

    let (mb, pb_map, global_is_set) = multibar();

    let pbc = pb_map.clone();

    tokio.block_on(async move {
        refresh
            .start(
                |count, event, total| {
                    if !no_progress {
                        match event {
                            RefreshEvent::ClosingTopic(topic_name) => bar_writeln(
                                |s| {
                                    mb.println(s).ok();
                                },
                                &style("INFO").blue().bold().to_string(),
                                &fl!("scan-topic-is-removed", name = topic_name),
                            ),
                            RefreshEvent::DownloadEvent(event) => {
                                pb!(event, mb, pb_map, count, total, global_is_set)
                            }
                        }
                    } else {
                        match event {
                            RefreshEvent::DownloadEvent(d) => {
                                handle_event_without_progressbar(d);
                            }
                            RefreshEvent::ClosingTopic(topic_name) => {
                                info!("{}", fl!("scan-topic-is-removed", name = topic_name));
                            }
                        }
                    }
                },
                || fl!("do-not-edit-topic-sources-list"),
            )
            .await
    })?;

    if let Some(gpb) = pbc.get(&0) {
        gpb.finish_and_clear();
    }

    Ok(())
}

pub struct NormalCommitArgs {
    pub apt: OmaApt,
    pub dry_run: bool,
    pub typ: SummaryType,
    pub apt_args: AptArgs,
    pub no_fixbroken: bool,
    pub network_thread: usize,
    pub no_progress: bool,
    pub sysroot: String,
}

pub(crate) fn normal_commit(args: NormalCommitArgs) -> Result<(), OutputError> {
    let NormalCommitArgs {
        apt,
        dry_run,
        typ,
        apt_args,
        no_fixbroken,
        network_thread,
        no_progress,
        sysroot,
    } = args;

    let op = apt.summary()?;
    let op_after = op.clone();
    let install = op.install;
    let remove = op.remove;
    let disk_size = op.disk_size;
    if check_empty_op(&install, &remove) {
        return Ok(());
    }

    apt.resolve(no_fixbroken)?;
    apt.check_disk_size()?;

    table_for_install_pending(&install, &remove, &disk_size, !apt_args.yes(), dry_run)?;

    let (mb, pb_map, global_is_set) = multibar();

    let start_time = Local::now().timestamp();

    let res = apt.commit(Some(network_thread), &apt_args, |count, event, total| {
        if !no_progress {
            pb!(event, mb, pb_map, count, total, global_is_set)
        } else {
            handle_event_without_progressbar(event);
        }
    });

    write_history_entry(
        op_after,
        typ,
        connect_or_create_db(true, sysroot)?,
        dry_run,
        start_time,
    )?;

    info!("{}", fl!("history-tips-2"));

    if res.is_ok() {
        success!("{}", fl!("history-tips-1"));
    } else {
        res?;
    }

    Ok(())
}

pub(crate) fn handle_event_without_progressbar(event: DownloadEvent) {
    match event {
        DownloadEvent::ChecksumMismatchRetry { filename, times } => {
            error!(
                "{}",
                fl!("checksum-mismatch-retry", c = filename, retry = times)
            );
        }
        DownloadEvent::CanNotGetSourceNextUrl(e) => {
            error!("{}", fl!("can-not-get-source-next-url", e = e.to_string()));
        }
        DownloadEvent::Done(msg) => {
            WRITER.writeln("DONE", &msg).ok();
        }
        _ => {}
    }
}

pub(crate) fn check_empty_op(install: &[InstallEntry], remove: &[RemoveEntry]) -> bool {
    if install.is_empty() && remove.is_empty() {
        success!("{}", fl!("no-need-to-do-anything"));
        return true;
    }

    false
}

pub(crate) fn check_unsupport_stmt(s: &str) {
    for i in s.chars() {
        if !i.is_ascii_alphabetic() && !i.is_ascii_alphanumeric() && i != '-' && i != '.' {
            warn!("Unexpected pattern: {s}");
        }
    }
}
