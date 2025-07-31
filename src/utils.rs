use std::{
    env,
    path::Path,
    process::{Command, exit},
    sync::atomic::Ordering,
};

use crate::{
    NOT_ALLOW_CTRLC,
    config::{BatteryTristate, Config, TakeWakeLockTristate},
    error::OutputError,
    path_completions::PathCompleter,
    subcommand::utils::no_check_dbus_warn,
};
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

    // 检测是否有 DISPLAY，如果有，则在提权时使用 pkexec
    // 通常情况下 SSH 连接不会有 DISPLAY 环境变量，除非开启 X11 Forwarding
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

pub fn dbus_check(
    yes: bool,
    config: &Config,
    no_check_dbus: bool,
    dry_run: bool,
    no_take_wake_lock: bool,
    no_check_battery: bool,
) -> Result<Vec<OwnedFd>> {
    if config.no_check_dbus() || no_check_dbus || dry_run {
        no_check_dbus_warn();
        return Ok(vec![]);
    }

    let Some(conn) = connect_dbus_impl() else {
        return Ok(vec![]);
    };

    if no_check_battery {
        check_battery_disabled_warn();
    } else {
        match config.check_battery() {
            BatteryTristate::Ask => {
                ask_continue_no_use_battery(&conn, yes);
            }
            BatteryTristate::Warn => {
                if is_battery(&conn) {
                    check_battery_disabled_warn();
                }
            }
            BatteryTristate::Ignore => {}
        }
    }

    // 需要保留 fd
    // login1 根据 fd 来判断是否关闭 inhibit
    if no_take_wake_lock {
        no_take_wake_lock_warn();
        Ok(vec![])
    } else {
        match config.take_wake_lock() {
            TakeWakeLockTristate::Yes => {
                Ok(RT.block_on(take_wake_lock(&conn, &fl!("changing-system"), "oma"))?)
            }
            TakeWakeLockTristate::Warn => {
                no_take_wake_lock_warn();
                Ok(vec![])
            }
            TakeWakeLockTristate::Ignore => Ok(vec![]),
        }
    }
}

#[inline]
pub(crate) fn check_battery_disabled_warn() {
    warn!("{}", fl!("battery-check-disabled"));
}

#[inline]
pub(crate) fn no_take_wake_lock_warn() {
    warn!("{}", fl!("session-check-disabled"));
}

pub fn connect_dbus_impl() -> Option<Connection> {
    let Ok(conn) = RT.block_on(create_dbus_connection()) else {
        warn!("{}", fl!("failed-check-dbus"));
        warn!("{}", fl!("failed-check-dbus-tips-1"));
        info!("{}", fl!("failed-check-dbus-tips-2"));
        info!("{}", fl!("failed-check-dbus-tips-3"));

        let theme = ColorfulTheme::default();
        let ans = Confirm::with_theme(&theme)
            .with_prompt(fl!("continue"))
            .default(false)
            .interact()
            .unwrap_or(false);

        if !ans {
            exit(0);
        }

        return None;
    };

    Some(conn)
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

pub fn ask_continue_no_use_battery(conn: &Connection, yes: bool) {
    let is_battery = is_battery(conn);

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

pub fn is_battery(conn: &Connection) -> bool {
    RT.block_on(is_using_battery(conn)).unwrap_or(false)
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

pub fn pkgnames_and_path_completions(current: &std::ffi::OsStr) -> Vec<CompletionCandidate> {
    let path_completions = PathCompleter::file()
        .filter(|x| x.extension().is_some_and(|y| y == "deb"))
        .complete(current);

    let mut completions = vec![];
    let current = &current.to_string_lossy();
    pkgnames_complete_impl(&mut completions, current, &[FilterMode::Names]);

    completions.extend(path_completions);

    completions
}

pub fn pkgnames_completions(current: &std::ffi::OsStr) -> Vec<CompletionCandidate> {
    let mut completions = vec![];
    let current = &current.to_string_lossy();
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
    let current = current.to_string_lossy();

    pkgnames_complete_impl(
        &mut completions,
        &current,
        &[FilterMode::Names, FilterMode::Installed],
    );

    completions
}
