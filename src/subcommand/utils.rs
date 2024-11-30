use std::borrow::Cow;
use std::collections::HashMap;
use std::error::Error;
use std::fmt::Debug;
use std::io;
use std::io::stderr;
use std::io::stdin;
use std::io::stdout;
use std::io::IsTerminal;
use std::panic;
use std::path::Path;
use std::path::PathBuf;
use std::sync::atomic::Ordering;
use std::thread;

use crate::color_formatter;
use crate::error::OutputError;
use crate::fl;
use crate::install_progress::NoInstallProgressManager;
use crate::install_progress::OmaInstallProgressManager;
use crate::pb::NoProgressBar;
use crate::pb::OmaMultiProgressBar;
use crate::pb::OmaProgressBar;
use crate::pb::RenderDownloadProgress;
use crate::pb::RenderRefreshProgress;
use crate::table::table_for_install_pending;
use crate::LOCKED;
use crate::RT;
use ahash::HashSet;
use apt_auth_config::AuthConfig;
use bon::builder;
use bon::Builder;
use chrono::Local;
use dialoguer::console;
use dialoguer::console::style;
use dialoguer::theme::ColorfulTheme;
use dialoguer::Confirm;
use flume::unbounded;
use oma_console::indicatif::HumanBytes;
use oma_console::msg;
use oma_console::pager::PagerExit;
use oma_console::print::Action;
use oma_console::success;
use oma_console::writer::Writeln;
use oma_console::WRITER;
use oma_contents::searcher::pure_search;
use oma_contents::searcher::ripgrep_search;
use oma_contents::searcher::Mode;
use oma_history::connect_db;
use oma_history::create_db_file;
use oma_history::write_history_entry;
use oma_history::SummaryType;
use oma_pm::apt::AptConfig;
use oma_pm::apt::CommitDownloadConfig;
use oma_pm::apt::OmaApt;
use oma_pm::apt::SummarySort;
use oma_pm::apt::{InstallEntry, RemoveEntry};
use oma_refresh::db::OmaRefresh;
use oma_utils::dpkg::dpkg_arch;
use oma_utils::oma::lock_oma_inner;
use oma_utils::oma::unlock_oma;
use reqwest::Client;
use std::fmt::Display;
use tracing::debug;
use tracing::error;
use tracing::info;
use tracing::warn;

use super::remove::ask_user_do_as_i_say;

pub(crate) fn handle_no_result(
    sysroot: impl AsRef<Path>,
    no_result: Vec<&str>,
    no_progress: bool,
) -> Result<(), OutputError> {
    if no_result.is_empty() {
        return Ok(());
    }

    let mut bin = vec![];

    let pb = if !no_progress || is_terminal() {
        Some(OmaProgressBar::new_spinner(Some(fl!("searching"))))
    } else {
        None
    };

    for word in no_result {
        if word == "266" {
            if let Some(ref pb) = pb {
                pb.writeln(
                    &style("ERROR").red().bold().to_string(),
                    "无法找到匹配关键字为艾露露的软件包",
                )
                .ok();
            } else {
                error!("无法找到匹配关键字为艾露露的软件包");
            }
        } else {
            if let Some(ref pb) = pb {
                pb.writeln(
                    &style("ERROR").red().bold().to_string(),
                    &fl!("could-not-find-pkg-from-keyword", c = word),
                )
                .ok();
            } else {
                error!("{}", fl!("could-not-find-pkg-from-keyword", c = word));
            }

            contents_search(&sysroot, Mode::BinProvides, word, |(pkg, file)| {
                if file == format!("/usr/bin/{}", word) {
                    bin.push((pkg, word));
                }
            })
            .ok();
        }
    }

    if let Some(ref pb) = pb {
        pb.inner.finish_and_clear();
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

    Err(OutputError {
        description: fl!("has-error-on-top"),
        source: None,
    })
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
    pub auth_config: &'a AuthConfig,
}

impl RefreshRequest<'_> {
    pub(crate) fn run(self) -> Result<(), OutputError> {
        let RefreshRequest {
            client,
            dry_run,
            no_progress,
            limit,
            sysroot,
            _refresh_topics,
            config,
            auth_config,
        } = self;

        if dry_run {
            return Ok(());
        }

        info!("{}", fl!("refreshing-repo-metadata"));

        let sysroot = PathBuf::from(sysroot);

        let msg = fl!("do-not-edit-topic-sources-list");

        let arch = dpkg_arch(&sysroot)?;

        let refresh = OmaRefresh::builder()
            .download_dir(sysroot.join("var/lib/apt/lists"))
            .source(sysroot)
            .threads(limit)
            .arch(arch)
            .apt_config(config)
            .client(client)
            .auth_config(auth_config)
            .topic_msg(&msg);

        #[cfg(feature = "aosc")]
        let refresh = refresh.refresh_topics(_refresh_topics).build();

        #[cfg(not(feature = "aosc"))]
        let refresh = refresh.build();

        let rx = refresh.get_recviver().clone();

        thread::spawn(move || {
            let mut pb: Box<dyn RenderRefreshProgress> = if no_progress || !is_terminal() {
                Box::new(NoProgressBar::default())
            } else {
                Box::new(OmaMultiProgressBar::default())
            };
            pb.render_refresh_progress(&rx);
        });

        RT.block_on(async move { refresh.start().await })?;

        Ok(())
    }
}

#[derive(Builder)]
pub(crate) struct CommitChanges<'a> {
    apt: OmaApt,
    #[builder(default = true)]
    dry_run: bool,
    request_type: SummaryType,
    #[builder(default = true)]
    no_fixbroken: bool,
    #[builder(default = 4)]
    network_thread: usize,
    #[builder(default)]
    no_progress: bool,
    #[builder(default = "/".into())]
    sysroot: String,
    #[builder(default)]
    fix_dpkg_status: bool,
    #[builder(default = true)]
    protect_essential: bool,
    client: &'a Client,
    #[builder(default)]
    yes: bool,
    #[builder(default)]
    remove_config: bool,
    #[builder(default)]
    autoremove: bool,
    auth_config: &'a AuthConfig,
}

impl CommitChanges<'_> {
    pub fn run(self) -> Result<i32, OutputError> {
        let CommitChanges {
            mut apt,
            dry_run,
            request_type: typ,
            no_fixbroken,
            network_thread,
            no_progress,
            sysroot,
            fix_dpkg_status,
            protect_essential,
            client,
            yes,
            remove_config,
            autoremove,
            auth_config,
        } = self;

        let pb = if !no_progress {
            OmaProgressBar::new_spinner(Some(fl!("resolving-dependencies"))).into()
        } else {
            None
        };

        if autoremove {
            apt.autoremove(remove_config)?;
        }

        apt.fix_broken(!no_fixbroken, fix_dpkg_status)?;
        apt.resolve(no_fixbroken, remove_config)?;

        if let Some(pb) = pb {
            pb.inner.finish_and_clear()
        }

        let op = apt.summary(
            SummarySort::Operation,
            |pkg| {
                if dry_run {
                    true
                } else if protect_essential {
                    false
                } else {
                    ask_user_do_as_i_say(pkg).unwrap_or(false)
                }
            },
            |features| {
                if dry_run {
                    true
                } else {
                    handle_features(features, protect_essential).unwrap_or(false)
                }
            },
        )?;

        debug!("{op}");

        apt.check_disk_size(&op)?;

        let op_after = op.clone();
        let install = &op.install;
        let remove = &op.remove;
        let disk_size = &op.disk_size;
        let (ar_count, ar_size) = op.autoremovable;

        if is_nothing_to_do(install, remove, !no_fixbroken) {
            autoremovable_tips(ar_count, ar_size)?;
            return Ok(0);
        }

        match table_for_install_pending(install, remove, disk_size, !yes, dry_run)? {
            PagerExit::NormalExit => {}
            x @ PagerExit::Sigint => return Ok(x.into()),
            x @ PagerExit::DryRun => return Ok(x.into()),
        }

        let start_time = Local::now().timestamp();

        let (tx, rx) = unbounded();

        thread::spawn(move || {
            let mut pb: Box<dyn RenderDownloadProgress> = if no_progress || !is_terminal() {
                Box::new(NoProgressBar::default())
            } else {
                Box::new(OmaMultiProgressBar::default())
            };
            pb.render_progress(&rx);
        });

        let res = apt.commit(
            client,
            CommitDownloadConfig {
                network_thread: Some(network_thread),
                auth: auth_config,
            },
            if no_progress || !is_terminal() {
                Box::new(NoInstallProgressManager)
            } else {
                Box::new(OmaInstallProgressManager)
            },
            tx,
            &op,
        );

        match res {
            Ok(_) => {
                let cmd = color_formatter().color_str("oma undo", Action::Emphasis);

                if !dry_run {
                    success!("{}", fl!("history-tips-1"));
                    info!("{}", fl!("history-tips-2", cmd = cmd.to_string()));
                }

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

                autoremovable_tips(ar_count, ar_size)?;

                Ok(0)
            }
            Err(e) => {
                let cmd = color_formatter().color_str("oma undo", Action::Emphasis);
                info!("{}", fl!("history-tips-2", cmd = cmd.to_string()));
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

pub fn autoremovable_tips(count: u64, total_size: u64) -> Result<(), OutputError> {
    if count == 0 {
        return Ok(());
    }

    let total_size = HumanBytes(total_size).to_string();
    let cmd1 = color_formatter()
        .color_str("oma list --autoremovable", Action::Emphasis)
        .to_string();
    let cmd2 = color_formatter()
        .color_str("oma mark manual <packages>", Action::Note)
        .to_string();
    let cmd3 = color_formatter()
        .color_str("oma autoremove", Action::Secondary)
        .to_string();
    let count = color_formatter()
        .color_str(count, Action::Secondary)
        .to_string();
    let total_size = color_formatter()
        .color_str(total_size, Action::Secondary)
        .to_string();
    info!(
        "{}",
        fl!(
            "autoremove-tips-1",
            count = count,
            size = total_size,
            cmd = cmd1
        )
    );
    info!("{}", fl!("autoremove-tips-2", cmd1 = cmd2, cmd2 = cmd3));

    Ok(())
}

pub(crate) fn is_nothing_to_do(
    install: &[InstallEntry],
    remove: &[RemoveEntry],
    fix_broken: bool,
) -> bool {
    if install.is_empty() && remove.is_empty() {
        if fix_broken {
            success!("{}", fl!("success"));
        } else {
            success!("{}", fl!("no-need-to-do-anything"));
        }

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

pub fn handle_features(features: &HashSet<Box<str>>, protect: bool) -> Result<bool, OutputError> {
    debug!("{:?}", features);

    let theme = ColorfulTheme::default();

    let features = match format_features(features) {
        Ok(v) => v,
        Err(e) => {
            warn!("{e}");

            if protect {
                error!("{}", fl!("features-without-value"));
                return Ok(false);
            }

            warn!("{}", fl!("features-without-value"));

            let delete = Confirm::with_theme(&theme)
                .with_prompt(fl!("features-continue-prompt"))
                .default(false)
                .interact()
                .map_err(|_| anyhow::anyhow!(""))?;

            return Ok(delete);
        }
    };

    if protect {
        error!("{}", fl!("features-tips-1"));
        msg!("\n{}\n", features);
        return Ok(false);
    }

    warn!("{}", fl!("features-tips-1"));
    msg!("\n{}\n", features);

    let delete = Confirm::with_theme(&theme)
        .with_prompt(fl!("features-continue-prompt"))
        .default(false)
        .interact()
        .map_err(|_| anyhow::anyhow!(""))?;

    Ok(delete)
}

pub fn format_features(features: &HashSet<Box<str>>) -> anyhow::Result<String> {
    let mut res = String::new();
    let features_data = std::fs::read_to_string("/usr/share/aosc-os/features.toml")?;
    let features_data: HashMap<Box<str>, HashMap<Box<str>, Box<str>>> =
        toml::from_str(&features_data)?;

    let lang = std::env::var("LANG")?;
    let lang = lang.split_once('.').unwrap_or(("en_US", "")).0;

    for (index, f) in features.iter().enumerate() {
        if let Some(v) = features_data.get(f) {
            let text = v.get(lang).unwrap_or_else(|| v.get("en_US").unwrap());
            res.push_str(&format!("  * {}", text));
        } else {
            res.push_str(&format!("  * {}", f));
        }

        if index != features.len() - 1 {
            res.push('\n');
        }
    }

    Ok(res)
}

pub fn tui_select_list_size() -> u16 {
    match WRITER.get_height() {
        0 => panic!("Terminal height must be greater than 0"),
        1..=6 => 1,
        x @ 7..=25 => x - 6,
        26.. => 20,
    }
}

pub fn select_tui_display_msg(s: &str, is_inquire: bool) -> Cow<str> {
    let term_width = WRITER.get_length() as usize;

    // 4 是 inquire 前面有四个空格缩进
    // 2 是 dialoguer 的保留字符长度
    let indent = if is_inquire { 4 } else { 2 };

    // 3 是 ... 的长度
    if console::measure_text_width(s) + indent > term_width {
        console::truncate_str(s, term_width - indent - 3, "...")
    } else {
        s.into()
    }
}

pub fn is_terminal() -> bool {
    let res = stderr().is_terminal() && stdout().is_terminal() && stdin().is_terminal();
    debug!("is terminal: {}", res);
    res
}
