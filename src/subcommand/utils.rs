use std::backtrace::Backtrace;
use std::error::Error;
use std::fmt::Debug;
use std::io;
use std::panic;
use std::path::PathBuf;
use std::sync::atomic::Ordering;

use crate::error::OutputError;
use crate::fl;
use crate::pb::NoProgressBar;
use crate::pb::OmaProgress;
use crate::pb::OmaProgressBar;
use crate::pb::ProgressEvent;
use crate::table::table_for_install_pending;
use crate::utils::create_async_runtime;
use crate::LOCKED;
use chrono::Local;
use oma_console::pager::PagerExit;
use oma_console::success;
use oma_history::connect_db;
use oma_history::create_db_file;
use oma_history::write_history_entry;
use oma_history::SummaryType;
use oma_pm::apt::AptArgs;
use oma_pm::apt::AptConfig;
use oma_pm::apt::OmaApt;
use oma_pm::apt::{InstallEntry, RemoveEntry};
use oma_refresh::db::OmaRefresh;
use oma_refresh::db::OmaRefreshBuilder;
use oma_utils::dpkg::dpkg_arch;
use oma_utils::oma::lock_oma_inner;
use oma_utils::oma::unlock_oma;
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

    panic::set_hook(Box::new(|info| {
        let backtrace = Backtrace::force_capture();
        eprintln!("{}", info);
        eprintln!("Backtrace:");
        eprintln!("{}", backtrace);
        unlock_oma().ok();
    }));

    LOCKED.store(true, Ordering::Relaxed);

    Ok(())
}

pub struct RefreshRequest<'a> {
    pub client: &'a Client,
    pub dry_run: bool,
    pub no_progress: bool,
    pub limit: usize,
    pub sysroot: &'a str,
    pub _refresh_topics: bool,
    pub config: &'a AptConfig,
}

impl<'a> RefreshRequest<'a> {
    pub(crate) fn run(self) -> Result<(), OutputError> {
        let RefreshRequest {
            client,
            dry_run,
            no_progress,
            limit,
            sysroot,
            _refresh_topics,
            config,
        } = self;

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
            apt_config: config,
        }
        .into();

        let tokio = create_async_runtime()?;

        let oma_pb: Box<dyn OmaProgress + Send + Sync> = if !no_progress {
            let pb = OmaProgressBar::new();
            Box::new(pb)
        } else {
            Box::new(NoProgressBar)
        };

        tokio.block_on(async move {
            refresh
                .start(
                    |count, event, total| oma_pb.change(ProgressEvent::from(event), count, total),
                    || format!("{}\n", fl!("do-not-edit-topic-sources-list")),
                )
                .await
        })?;

        Ok(())
    }
}

pub struct CommitRequest<'a> {
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
    pub client: &'a Client,
}

impl<'a> CommitRequest<'a> {
    pub fn run(self) -> Result<i32, OutputError> {
        let CommitRequest {
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
            client,
        } = self;

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
            return Ok(0);
        }

        match table_for_install_pending(install, remove, disk_size, !apt_args.yes(), dry_run)? {
            PagerExit::NormalExit => {}
            x @ PagerExit::Sigint => return Ok(x.into()),
            x @ PagerExit::DryRun => return Ok(x.into()),
        }

        let oma_pb: Box<dyn OmaProgress + Sync + Send> = if !no_progress {
            let pb = OmaProgressBar::new();
            Box::new(pb)
        } else {
            Box::new(NoProgressBar)
        };

        let start_time = Local::now().timestamp();

        let res = apt.commit(
            client,
            Some(network_thread),
            &apt_args,
            |count, event, total| {
                oma_pb.change(ProgressEvent::from(event), count, total);
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
                Ok(0)
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
