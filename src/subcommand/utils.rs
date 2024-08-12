use std::error::Error;
use std::fmt::Debug;
use std::io;
use std::path::PathBuf;
use std::sync::atomic::Ordering;

use crate::error::OutputError;
use crate::fl;
use crate::pb;
use crate::table::table_for_install_pending;
use crate::utils::create_async_runtime;
use crate::utils::multibar;
use crate::AILURUS;
use crate::LOCKED;
use chrono::Local;
use dialoguer::console::style;
use oma_console::success;
use oma_console::writer::bar_writeln;
use oma_console::WRITER;
use oma_fetch::DownloadEvent;
use oma_history::connect_db;
use oma_history::create_db_file;
use oma_history::write_history_entry;
use oma_history::SummaryType;
use oma_pm::apt::AptArgs;
use oma_pm::apt::OmaApt;
use oma_pm::apt::{InstallEntry, RemoveEntry};
use oma_refresh::db::OmaRefresh;
use oma_refresh::db::OmaRefreshBuilder;
use oma_refresh::db::RefreshEvent;
use oma_utils::dpkg::dpkg_arch;
use oma_utils::oma::lock_oma_inner;
use reqwest::Client;
use std::fmt::Display;
use tracing::error;
use tracing::info;
use tracing::warn;

use super::remove::ask_user_do_as_i_say;

pub(crate) fn handle_no_result(no_result: Vec<String>) -> Result<(), OutputError> {
    for word in &no_result {
        if word == "266" {
            error!("无法找到匹配关键字为艾露露的软件包");
        } else {
            error!("{}", fl!("could-not-find-pkg-from-keyword", c = word));
        }
    }

    if no_result.is_empty() {
        Ok(())
    } else {
        Err(OutputError {
            description: fl!("has-error-on-top"),
            source: None,
        })
    }
}

#[derive(Debug)]
pub struct LockError {
    source: io::Error,
}

impl Display for LockError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("Failed to lock oma")
    }
}

impl Error for LockError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        Some(&self.source)
    }
}

pub(crate) fn lock_oma() -> Result<(), LockError> {
    lock_oma_inner().map_err(|e| LockError { source: e })?;
    LOCKED.store(true, Ordering::Relaxed);

    Ok(())
}

pub(crate) fn refresh(
    client: &Client,
    dry_run: bool,
    no_progress: bool,
    limit: usize,
    sysroot: &str,
    _refresh_topics: bool,
) -> Result<(), OutputError> {
    if dry_run {
        return Ok(());
    }

    info!("{}", fl!("refreshing-repo-metadata"));

    let sysroot = PathBuf::from(sysroot);

    let refresh: OmaRefresh = OmaRefreshBuilder {
        source: sysroot.clone(),
        limit: Some(limit),
        arch: dpkg_arch(&sysroot)?,
        download_dir: sysroot.join("var/lib/apt/lists"),
        client,
        #[cfg(feature = "aosc")]
        refresh_topics: _refresh_topics,
    }
    .into();

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
                            RefreshEvent::ScanningTopic => {
                                let (sty, inv) = oma_console::pb::oma_spinner(
                                    AILURUS.load(std::sync::atomic::Ordering::Relaxed),
                                );
                                let pb = mb.insert(
                                    count + 1,
                                    oma_console::indicatif::ProgressBar::new_spinner()
                                        .with_style(sty),
                                );
                                pb.set_message(fl!("refreshing-topic-metadata"));
                                pb.enable_steady_tick(inv);
                                pb_map.insert(count + 1, pb);
                            }
                            RefreshEvent::TopicNotInMirror(topic, mirror) => {
                                bar_writeln(
                                    |s| {
                                        mb.println(s).ok();
                                    },
                                    &style("WARNING").yellow().bold().to_string(),
                                    &fl!("topic-not-in-mirror", topic = topic, mirror = mirror),
                                );
                                bar_writeln(
                                    |s| {
                                        mb.println(s).ok();
                                    },
                                    &style("WARNING").yellow().bold().to_string(),
                                    &fl!("skip-write-mirror"),
                                );
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
                            RefreshEvent::ScanningTopic => {
                                info!("{}", fl!("refreshing-topic-metadata"));
                            }
                            RefreshEvent::TopicNotInMirror(topic, mirror) => {
                                warn!(
                                    "{}",
                                    fl!("topic-not-in-mirror", topic = topic, mirror = mirror)
                                );
                                warn!("{}", fl!("skip-write-mirror"));
                            }
                        }
                    }
                },
                || format!("{}\n", fl!("do-not-edit-topic-sources-list")),
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
    pub fix_dpkg_status: bool,
    pub protect_essential: bool,
}

pub(crate) fn normal_commit(args: NormalCommitArgs, client: &Client) -> Result<(), OutputError> {
    let NormalCommitArgs {
        mut apt,
        dry_run,
        typ,
        apt_args,
        no_fixbroken,
        network_thread,
        no_progress,
        sysroot,
        fix_dpkg_status,
        protect_essential,
    } = args;

    apt.resolve(no_fixbroken, fix_dpkg_status)?;

    let op = apt.summary(|pkg| {
        if protect_essential {
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
        return Ok(());
    }

    let b = table_for_install_pending(&install, &remove, &disk_size, !apt_args.yes(), dry_run)?;

    if !b {
        return Ok(());
    }

    let (mb, pb_map, global_is_set) = multibar();

    let start_time = Local::now().timestamp();

    let res = apt.commit(
        client,
        Some(network_thread),
        &apt_args,
        |count, event, total| {
            if !no_progress {
                pb!(event, mb, pb_map, count, total, global_is_set)
            } else {
                handle_event_without_progressbar(event);
            }
        },
        op,
    );

    match res {
        Ok(_) => {
            success!("{}", fl!("history-tips-1"));
            info!("{}", fl!("history-tips-2"));
            write_history_entry(
                op_after,
                typ,
                {
                    let db = create_db_file(sysroot)?;
                    connect_db(db, true)?
                },
                dry_run,
                start_time,
                true,
            )?;
            Ok(())
        }
        Err(e) => {
            info!("{}", fl!("history-tips-2"));
            write_history_entry(
                op_after,
                typ,
                {
                    let db = create_db_file(sysroot)?;
                    connect_db(db, true)?
                },
                dry_run,
                start_time,
                false,
            )?;
            Err(e.into())
        }
    }
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
        if !i.is_ascii_alphabetic()
            && !i.is_ascii_alphanumeric()
            && i != '-'
            && i != '.'
            && i != ':'
        {
            warn!("Unexpected pattern: {s}");
        }
    }
}

pub(crate) fn no_check_dbus_warn() {
    warn!("{}", fl!("no-check-dbus-tips"));
}
