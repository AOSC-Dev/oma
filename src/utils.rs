use std::{
    env,
    path::Path,
    process::{Command, exit},
    sync::atomic::Ordering,
};

use crate::{NOT_ALLOW_CTRLC, error::OutputError, path_completions::PathCompleter};
use crate::{RT, fl};

use anyhow::anyhow;
use clap_complete::{CompletionCandidate, engine::ValueCompleter};
use dialoguer::{Confirm, theme::ColorfulTheme};
use oma_pm::apt::{AptConfig, FilterMode, OmaApt, OmaAptArgs};
use oma_utils::{
    dbus::{Connection, create_dbus_connection, is_using_battery, session_name, take_wake_lock},
    oma::unlock_oma,
    zbus::zvariant::OwnedFd,
};
use rustix::process;
use tracing::{debug, info, warn};

type Result<T> = std::result::Result<T, OutputError>;

pub fn root() -> Result<()> {
    if is_root() {
        return Ok(());
    }

    let mut args = std::env::args().collect::<Vec<_>>();

    if (env::var("DISPLAY").is_ok() || env::var("WAYLAND_DISPLAY").is_ok()) && !is_wsl() {
        // Workaround: 使用 pkexec 执行其它程序时，若你指定了相对路径
        // pkexec 并不会以当前路径作为起点寻求这个位置
        // 所以需要转换成绝对路径，再喂给 pkexec
        file_path_canonicalize(&mut args);

        info!("{}", fl!("pkexec-tips-1"));
        info!("{}", fl!("pkexec-tips-2"));

        NOT_ALLOW_CTRLC.store(true, Ordering::Relaxed);

        let out = Command::new("pkexec")
            .args(args)
            .spawn()
            .and_then(|x| x.wait_with_output())
            .map_err(|e| anyhow!(fl!("execute-pkexec-fail", e = e.to_string())))?;

        exit(out.status.code().unwrap_or(1));
    }

    Err(OutputError {
        description: fl!("please-run-me-as-root"),
        source: None,
    })
}

#[inline]
pub fn is_root() -> bool {
    process::geteuid().is_root()
}

fn file_path_canonicalize(args: &mut Vec<String>) {
    for arg in args {
        if !arg.ends_with(".deb") {
            continue;
        }

        let path = Path::new(&arg);
        let path = path.canonicalize().unwrap_or(path.to_path_buf());
        *arg = path.display().to_string();
    }
}

pub fn dbus_check(yes: bool) -> Result<Vec<OwnedFd>> {
    let conn = RT.block_on(create_dbus_connection())?;
    check_battery(&conn, yes);

    // 需要保留 fd
    // login1 根据 fd 来判断是否关闭 inhibit
    let fds = RT.block_on(take_wake_lock(&conn, &fl!("changing-system"), "oma"))?;

    Ok(fds)
}

// From `is_wsl` crate
fn is_wsl() -> bool {
    if let Ok(b) = std::fs::read("/proc/sys/kernel/osrelease") {
        if let Ok(s) = std::str::from_utf8(&b) {
            let a = s.to_ascii_lowercase();
            return a.contains("microsoft") || a.contains("wsl");
        }
    }

    false
}

pub fn check_battery(conn: &Connection, yes: bool) {
    let is_battery = RT.block_on(is_using_battery(conn)).unwrap_or(false);

    if is_battery {
        if yes {
            return;
        }
        let theme = ColorfulTheme::default();
        warn!("{}", fl!("battery"));
        let cont = Confirm::with_theme(&theme)
            .with_prompt(fl!("continue"))
            .default(false)
            .interact();

        if !cont.unwrap_or(false) {
            unlock_oma().ok();
            exit(0);
        }
    }
}

pub fn is_ssh_from_loginctl() -> bool {
    let conn = RT.block_on(create_dbus_connection());

    if let Ok(conn) = conn {
        let session_name = RT.block_on(session_name(&conn));
        debug!("session_name: {:?}", session_name);

        return session_name.is_ok_and(|name| name == "system-remote-login");
    }

    false
}

/// oma display normal message
#[macro_export]
macro_rules! msg {
    ($($arg:tt)+) => {
        use oma_console::writer::Writeln as _;
        let s = format!($($arg)+);
        tracing::debug!("{s}");
        $crate::WRITER.writeln("", &s).ok();
    };
}

/// oma display success message
#[macro_export]
macro_rules! success {
    ($($arg:tt)+) => {
        use oma_console::writer::Writeln as _;
        let s = format!($($arg)+);
        tracing::debug!("{s}");
        $crate::WRITER.writeln(&oma_console::console::style("SUCCESS").green().bold().to_string(), &s).ok();
    };
}

/// oma display due_to message
#[macro_export]
macro_rules! due_to {
    ($($arg:tt)+) => {
        use oma_console::writer::Writeln as _;
        let s = format!($($arg)+);
        tracing::debug!("{s}");
        $crate::WRITER.writeln(&oma_console::console::style("DUE TO").yellow().bold().to_string(), &s).ok();
    };
}

pub fn pkgnames_completions(current: &std::ffi::OsStr) -> Vec<CompletionCandidate> {
    let mut completions = PathCompleter::file().complete(current);

    let Some(current) = current.to_str() else {
        return completions;
    };

    pkgnames_complete_impl(&mut completions, current, &[FilterMode::Names]);

    completions
}

fn pkgnames_complete_impl(
    completions: &mut Vec<CompletionCandidate>,
    current: &str,
    filter_mode: &[FilterMode],
) {
    let Ok(apt) = OmaApt::new(
        vec![],
        OmaAptArgs::builder().build(),
        false,
        AptConfig::new(),
    ) else {
        return;
    };

    let Ok(pkgs) = apt.filter_pkgs(filter_mode) else {
        return;
    };

    if current.is_empty() {
        for i in pkgs {
            completions.push(i.name().into());
        }
    } else {
        for i in pkgs.filter(|i| i.name().starts_with(current)) {
            completions.push(i.name().into());
        }
    }
}

pub fn pkgnames_remove_completions(current: &std::ffi::OsStr) -> Vec<CompletionCandidate> {
    let mut completions = vec![];
    let Some(current) = current.to_str() else {
        return completions;
    };

    pkgnames_complete_impl(
        &mut completions,
        current,
        &[FilterMode::Names, FilterMode::Installed],
    );

    completions
}
