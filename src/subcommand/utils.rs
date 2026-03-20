use std::borrow::Cow;
use std::io::IsTerminal;
use std::io::Write;
use std::io::stderr;
use std::io::stdin;
use std::io::stdout;
use std::os::fd::OwnedFd;
use std::panic;
use std::path::Path;

use crate::WRITER;
use crate::color_formatter;
use crate::error::OutputError;
use crate::find_another_oma;
use crate::fl;
use crate::get_lock;
use crate::msg;
use crate::pb::OmaProgressBar;
use crate::utils::get_lists_dir;
use anyhow::Context;
use apt_auth_config::AuthConfig;
use dialoguer::console;
use dialoguer::console::style;
use fs_extra::dir::get_size as get_dir_size;
use indexmap::IndexSet;
use inquire::MultiSelect;
use inquire::ui::RenderConfig;
use oma_console::indicatif::HumanBytes;
use oma_console::print::Action;
use oma_console::writer::Writeln;
use oma_contents::searcher::Mode;
use oma_contents::searcher::search;
use oma_pm::CustomDownloadMessage;
use oma_pm::apt::AptConfig;
use oma_pm::apt::OmaApt;
use oma_utils::GetLockError;
use oma_utils::get_file_lock;
use spdlog::{debug, error, info, warn};
use std::fmt::Display;

pub(crate) fn handle_no_result(no_result: Vec<&str>, no_progress: bool) -> Result<(), OutputError> {
    if no_result.is_empty() {
        return Ok(());
    }

    let mut bin = IndexSet::with_hasher(ahash::RandomState::new());

    let pb = create_progress_spinner(no_progress, fl!("searching"));

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

            search(
                get_lists_dir(&AptConfig::new()),
                Mode::BinProvides,
                word,
                |(pkg, file)| {
                    if file == format!("/usr/bin/{word}") {
                        bin.insert((pkg, word));
                    }
                },
            )
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

pub(crate) fn lock_oma(sysroot: impl AsRef<Path>) -> Result<OwnedFd, OutputError> {
    let lock = get_lock(sysroot.as_ref());
    std::fs::create_dir_all(
        lock.parent()
            .ok_or_else(|| anyhow::anyhow!("Path {} is root", lock.display()))?,
    )
    .context("Failed to create lock dir")?;

    let lock = get_file_lock(lock).map_err(|e| match e {
        GetLockError::SetLock(errno) => OutputError {
            description: fl!("failed-to-lock-oma"),
            source: Some(Box::new(errno)),
        },
        GetLockError::SetLockWithProcess(_, pid) => {
            let error_str = match find_another_oma() {
                Ok(status) => fl!("another-oma-is-running", s = status, pid = pid),
                Err(_) => fl!("another-oma-is-running-without-status", pid = pid),
            };

            OutputError {
                description: error_str,
                source: None,
            }
        }
    })?;

    Ok(lock)
}

pub fn auth_config(sysroot: impl AsRef<Path>) -> Option<AuthConfig> {
    AuthConfig::system(sysroot)
        .inspect(|res| debug!("Auth config: {res:#?}"))
        .inspect_err(|e| debug!("Couldn't read auth config: {e}"))
        .ok()
}

pub fn download_message() -> Option<CustomDownloadMessage> {
    const NAME_VERSION_LENGTH_LIMIT: usize = 35;

    Some(Box::new(|entry| {
        let name_and_version = format!("{} {}", entry.name(), entry.new_version());
        let name_and_version = if console::measure_text_width(&name_and_version)
            > NAME_VERSION_LENGTH_LIMIT
        {
            console::truncate_str(&name_and_version, NAME_VERSION_LENGTH_LIMIT, "...").to_string()
        } else {
            name_and_version
        };

        format!("{} ({})", name_and_version, entry.arch()).into()
    }))
}

pub fn space_tips(apt: &OmaApt, sysroot: impl AsRef<Path>) {
    let space = match fs4::available_space(&sysroot) {
        Ok(space) => space,
        Err(e) => {
            warn!("Unable to get available space: {e}");
            return;
        }
    };

    if space >= 5 * 1024 * 1024 * 1024 {
        return;
    }

    let archive_dir_space = match get_dir_size(apt.get_archive_dir()) {
        Ok(size) => size,
        Err(e) => {
            warn!("Unable to get archive dir space: {e}");
            return;
        }
    };

    if archive_dir_space != 0 {
        let human_space = HumanBytes(archive_dir_space).to_string();
        let cmd = color_formatter()
            .color_str("oma clean", Action::Secondary)
            .to_string();

        warn!("{}", fl!("space-warn", size = human_space, cmd = cmd));
    } else {
        warn!("{}", fl!("space-warn-with-zero"));
    }
}

#[allow(dead_code)]
#[inline]
pub fn multiselect<T: Display>(
    msg: &str,
    opts: Vec<T>,
    formatter: &dyn Fn(&[inquire::list_option::ListOption<&T>]) -> String,
    render_config: RenderConfig<'_>,
    page_size: u16,
    default: Vec<usize>,
) -> Result<Vec<T>, anyhow::Error> {
    MultiSelect::new(msg, opts)
        .with_help_message(&fl!("tips"))
        .with_formatter(formatter)
        .with_default(&default)
        .with_page_size(page_size as usize)
        .with_render_config(render_config)
        .prompt()
        .map_err(|e| match e {
            inquire::InquireError::OperationInterrupted => {
                stderr().write_all(b"\n").ok();
                anyhow::anyhow!("")
            }
            e => e.into(),
        })
}

pub fn create_progress_spinner(no_progress: bool, msg: String) -> Option<OmaProgressBar> {
    if !no_progress {
        OmaProgressBar::new_spinner(Some(msg)).into()
    } else {
        None
    }
}

pub(crate) fn no_check_dbus_warn() {
    warn!("{}", fl!("no-check-dbus-tips"));
}

pub fn tui_select_list_size() -> u16 {
    match WRITER.get_height() {
        0 => panic!("Terminal height must be greater than 0"),
        1..=6 => 1,
        x @ 7..=25 => x - 6,
        26.. => 20,
    }
}

pub fn select_tui_display_msg(s: &str, is_inquire: bool) -> Cow<'_, str> {
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
