use std::error::Error;
use std::fmt::Debug;
use std::io;
use std::panic;
use std::path::Path;
use std::path::PathBuf;
use std::sync::atomic::Ordering;

use crate::color_formatter;
use crate::error::OutputError;
use crate::fl;
use crate::pb::NoProgressBar;
use crate::pb::OmaProgressBar;
use crate::table::table_for_install_pending;
use crate::utils::create_async_runtime;
use crate::LOCKED;
use chrono::Local;
use dialoguer::console::style;
use oma_console::msg;
use oma_console::pager::PagerExit;
use oma_console::print::Action;
use oma_console::success;
use oma_console::writer::bar_writeln;
use oma_console::{indicatif::ProgressBar, pb::spinner_style};
use oma_contents::searcher::pure_search;
use oma_contents::searcher::ripgrep_search;
use oma_contents::searcher::Mode;
use oma_fetch::DownloadProgressControl;
use oma_history::connect_db;
use oma_history::create_db_file;
use oma_history::write_history_entry;
use oma_history::SummaryType;
use oma_pm::apt::AptArgs;
use oma_pm::apt::AptConfig;
use oma_pm::apt::OmaApt;
use oma_pm::apt::{InstallEntry, RemoveEntry};
use oma_refresh::db::HandleRefresh;
use oma_refresh::db::OmaRefresh;
use oma_utils::dpkg::dpkg_arch;
use oma_utils::oma::lock_oma_inner;
use oma_utils::oma::unlock_oma;
use reqwest::Client;
use std::fmt::Display;
use tracing::info;
use tracing::warn;

use super::remove::ask_user_do_as_i_say;

pub(crate) fn handle_no_result(
    sysroot: impl AsRef<Path>,
    no_result: Vec<String>,
) -> Result<(), OutputError> {
    let mut bin = vec![];

    let (sty, inv) = spinner_style();
    let pb = ProgressBar::new_spinner().with_style(sty);
    pb.enable_steady_tick(inv);
    pb.set_message(fl!("searching"));

    for word in &no_result {
        if word == "266" {
            bar_writeln(
                |s| pb.println(s),
                &style("ERROR").red().bold().to_string(),
                "无法找到匹配关键字为艾露露的软件包",
            );
        } else {
            bar_writeln(
                |s| pb.println(s),
                &style("ERROR").red().bold().to_string(),
                &fl!("could-not-find-pkg-from-keyword", c = word),
            );

            contents_search(&sysroot, Mode::BinProvides, word, |(pkg, file)| {
                if file == format!("/usr/bin/{}", word) {
                    bin.push((pkg, word));
                }
            })
            .ok();

            pb.finish_and_clear();
        }
    }

    if !bin.is_empty() {
        info!("{}", fl!("no-result-bincontents-tips"));
        for (pkg, cmd) in bin {
            msg!(
                "{}",
                fl!(
                    "no-result-bincontents-tips-2",
                    pkg = color_formatter()
                        .color_str(pkg, Action::Emphasis)
                        .to_string(),
                    cmd = color_formatter()
                        .color_str(cmd, Action::Secondary)
                        .to_string()
                )
            );
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

pub fn contents_search(
    sysroot: impl AsRef<Path>,
    mode: Mode,
    input: &str,
    cb: impl FnMut((String, String)) + Send + Sync,
) -> Result<(), OutputError> {
    if which::which("rg").is_ok() {
        ripgrep_search(sysroot.as_ref().join("var/lib/apt/lists"), mode, input, cb)?;
    } else {
        pure_search(sysroot.as_ref().join("var/lib/apt/lists"), mode, input, cb)?;
    };

    Ok(())
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
    let hook = std::panic::take_hook();

    panic::set_hook(Box::new(move |info| {
        unlock_oma().ok();
        hook(info);
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

        let msg = fl!("do-not-edit-topic-sources-list");

        let pm: &dyn HandleRefresh = if !no_progress {
            &OmaProgressBar::default()
        } else {
            &NoProgressBar
        };

        let arch = dpkg_arch(&sysroot)?;

        let refresh = OmaRefresh::builder()
            .download_dir(sysroot.join("var/lib/apt/lists"))
            .source(sysroot)
            .threads(limit)
            .arch(arch)
            .apt_config(config)
            .client(client)
            .progress_manager(pm)
            .topic_msg(&msg);

        #[cfg(feature = "aosc")]
        let refresh = refresh.refresh_topics(_refresh_topics).build();

        #[cfg(not(feature = "aosc"))]
        let refresh = refresh.build();

        let tokio = create_async_runtime()?;

        tokio.block_on(async move { refresh.start().await })?;

        Ok(())
    }
}

pub struct CommitRequest<'a> {
    pub apt: OmaApt,
    pub dry_run: bool,
    pub request_type: SummaryType,
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
            request_type: typ,
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

        if is_nothing_to_do(install, remove) {
            return Ok(0);
        }

        match table_for_install_pending(install, remove, disk_size, !apt_args.yes(), dry_run)? {
            PagerExit::NormalExit => {}
            x @ PagerExit::Sigint => return Ok(x.into()),
            x @ PagerExit::DryRun => return Ok(x.into()),
        }

        let start_time = Local::now().timestamp();

        let pm: Box<dyn DownloadProgressControl> = if !no_progress {
            let pb = OmaProgressBar::default();
            Box::new(pb)
        } else {
            Box::new(NoProgressBar)
        };

        let res = apt.commit(client, Some(network_thread), &apt_args, pm.as_ref(), op);

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

pub(crate) fn is_nothing_to_do(install: &[InstallEntry], remove: &[RemoveEntry]) -> bool {
    if install.is_empty() && remove.is_empty() {
        success!("{}", fl!("no-need-to-do-anything"));
        return true;
    }

    false
}

pub(crate) fn check_unsupported_stmt(s: &str) {
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
