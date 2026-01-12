use std::{
    env,
    io::ErrorKind,
    path::PathBuf,
    process::{Command, exit},
    sync::atomic::Ordering,
};

use crate::{
    NOT_ALLOW_CTRLC,
    config::{BatteryTristate, Config, TakeWakeLockTristate},
    error::OutputError,
    path_completions::PathCompleter,
    subcommand::utils::no_check_dbus_warn,
    unlock_oma,
};
use crate::{RT, fl};

use anyhow::anyhow;
use clap_complete::{CompletionCandidate, engine::ValueCompleter};
use dialoguer::{Confirm, theme::ColorfulTheme};
use oma_pm::{
    apt::{AptConfig, OmaApt, OmaAptArgs},
    oma_apt::PackageSort,
};
use oma_utils::{
    dbus::{
        Connection, InhibitTypeUnion, create_dbus_connection, is_using_battery, session_name,
        take_wake_lock,
    },
    is_termux,
    zbus::zvariant::OwnedFd,
};
use rustix::process;
use spdlog::{debug, error, info, warn};

type Result<T> = std::result::Result<T, OutputError>;

pub fn root() -> Result<()> {
    if is_termux() {
        return Ok(());
    }

    if is_root() {
        return Ok(());
    }

    NOT_ALLOW_CTRLC.store(true, Ordering::Relaxed);

    match env::var("OMA_PERM_TOOL").as_deref() {
        Ok("sdr") => systemd_run_oma()?,
        Ok("pkexec") => pkexec_oma()?,
        Ok(x) => other_permission_tools_run_oma(x)?,
        Err(_) => {}
    }

    // Fix issue https://github.com/AOSC-Dev/oma/issues/609
    if which::which("systemd-run").is_ok() {
        systemd_run_oma()?;
    } else if is_desktop_env() && !is_wsl() && which::which("pkexec").is_ok() {
        // 检测是否有 DISPLAY，如果有，则在提权时使用 pkexec
        // 通常情况下 SSH 连接不会有 DISPLAY 环境变量，除非开启 X11 Forwarding
        pkexec_oma()?;
    } else if which::which("sudo").is_ok() {
        other_permission_tools_run_oma("sudo")?;
    } else if which::which("doas").is_ok() {
        other_permission_tools_run_oma("doas")?;
    }

    Err(OutputError {
        description: fl!("please-run-me-as-root"),
        source: None,
    })
}

#[inline]
fn is_desktop_env() -> bool {
    env::var("DISPLAY").is_ok() || env::var("WAYLAND_DISPLAY").is_ok()
}

fn other_permission_tools_run_oma(cmd: &str) -> Result<()> {
    let out = Command::new(cmd)
        .args(std::env::args())
        .spawn()
        .and_then(|x| x.wait_with_output())
        .map_err(|e| anyhow!(fl!("execute-cmd-fail", cmd = cmd, e = e.to_string())))?;

    exit(out.status.code().unwrap_or(1));
}

fn pkexec_oma() -> Result<()> {
    info!("{}", fl!("pkexec-tips-1"));
    info!("{}", fl!("pkexec-tips-2"));
    let out = Command::new("pkexec")
        .arg("--keep-cwd")
        .args(std::env::args())
        .spawn()
        .and_then(|x| x.wait_with_output())
        .map_err(|e| anyhow!(fl!("execute-cmd-fail", cmd = "pkexec", e = e.to_string())))?;

    exit(out.status.code().unwrap_or(1));
}

fn systemd_run_oma() -> Result<()> {
    let out = Command::new("systemd-run")
        .env("SYSTEMD_ADJUST_TERMINAL_TITLE", "0")
        .arg("--same-dir")
        .arg("--pty")
        .arg("--quiet")
        .arg("--unit=oma")
        .arg("--collect")
        .args(std::env::args())
        .spawn()
        .and_then(|x| x.wait_with_output())
        .map_err(|e| {
            anyhow!(fl!(
                "execute-cmd-fail",
                cmd = "systemd-run",
                e = e.to_string()
            ))
        })?;

    exit(out.status.code().unwrap_or(1));
}

#[inline]
pub fn is_root() -> bool {
    process::geteuid().is_root()
}

#[inline]
pub fn get_lists_dir(config: &AptConfig) -> PathBuf {
    PathBuf::from(config.dir("Dir::State::lists", "lists/"))
}

pub fn dbus_check(
    yes: bool,
    config: &Config,
    no_check_dbus: bool,
    dry_run: bool,
    no_take_wake_lock: bool,
    no_check_battery: bool,
) -> Result<Option<OwnedFd>> {
    if config.no_check_dbus() || no_check_dbus || dry_run {
        no_check_dbus_warn();
        return Ok(None);
    }

    if is_termux() {
        return Ok(None);
    }

    let Some(conn) = connect_dbus_impl() else {
        return Ok(None);
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
        Ok(None)
    } else {
        match config.take_wake_lock() {
            TakeWakeLockTristate::Yes => Ok(Some(RT.block_on(take_wake_lock(
                &conn,
                InhibitTypeUnion::all(),
                &fl!("changing-system"),
                "oma",
            ))?)),
            TakeWakeLockTristate::Warn => {
                no_take_wake_lock_warn();
                Ok(None)
            }
            TakeWakeLockTristate::Ignore => Ok(None),
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
        if is_termux() {
            return None;
        }

        warn!("{}", fl!("failed-check-dbus"));
        warn!("{}", fl!("failed-check-dbus-tips-1"));
        info!("{}", fl!("failed-check-dbus-tips-2"));
        info!("{}", fl!("failed-check-dbus-tips-3"));

        let theme = ColorfulTheme::default();
        let ans = Confirm::with_theme(&theme)
            .with_prompt(fl!("continue"))
            .default(false)
            .interact();

        handle_dialoguer_question_result(ans);

        return None;
    };

    Some(conn)
}

// From `is_wsl` crate
fn is_wsl() -> bool {
    if let Ok(b) = std::fs::read("/proc/sys/kernel/osrelease")
        && let Ok(s) = std::str::from_utf8(&b)
    {
        let a = s.to_ascii_lowercase();
        return a.contains("microsoft") || a.contains("wsl");
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

        handle_dialoguer_question_result(cont);
    }
}

fn handle_dialoguer_question_result(res: std::result::Result<bool, dialoguer::Error>) {
    match res {
        Ok(b) => {
            if !b {
                unlock_oma().ok();
                exit(0);
            }
        }
        Err(e) => {
            let dialoguer::Error::IO(e) = e;
            if e.kind() != ErrorKind::Interrupted {
                unlock_oma().ok();
                error!("{e}");
                exit(1);
            } else {
                debug!("Interrupted by user.");
            }
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
        spdlog::debug!("{s}");
        $crate::WRITER.writeln("", &s).ok();
    };
}

/// oma display success message
#[macro_export]
macro_rules! success {
    ($($arg:tt)+) => {
        use oma_console::writer::Writeln as _;
        let s = format!($($arg)+);
        spdlog::debug!("{s}");
        $crate::WRITER.writeln(&oma_console::console::style("SUCCESS").green().bold().to_string(), &s).ok();
    };
}

/// oma display due_to message
#[macro_export]
macro_rules! due_to {
    ($($arg:tt)+) => {
        use oma_console::writer::Writeln as _;
        let s = format!($($arg)+);
        spdlog::debug!("{s}");
        $crate::WRITER.writeln(&oma_console::console::style("DUE TO").yellow().bold().to_string(), &s).ok();
    };
}

pub fn pkgnames_and_path_completions(current: &std::ffi::OsStr) -> Vec<CompletionCandidate> {
    let path_completions = PathCompleter::file()
        .filter(|x| x.extension().is_some_and(|y| y == "deb"))
        .complete(current);

    let mut completions = vec![];
    let current = &current.to_string_lossy();
    pkgnames_complete_impl(&mut completions, current, PackageSort::default().names());

    completions.extend(path_completions);

    completions
}

pub fn pkgnames_completions(current: &std::ffi::OsStr) -> Vec<CompletionCandidate> {
    let mut completions = vec![];
    let current = &current.to_string_lossy();
    pkgnames_complete_impl(&mut completions, current, PackageSort::default().names());

    completions
}

fn pkgnames_complete_impl(
    completions: &mut Vec<CompletionCandidate>,
    current: &str,
    sort: PackageSort,
) {
    let Ok(apt) = OmaApt::new(
        vec![],
        OmaAptArgs::builder().build(),
        false,
        AptConfig::new(),
    ) else {
        return;
    };

    let pkgs = apt.cache.packages(&sort);

    if current.is_empty() {
        for pkg in pkgs {
            completions.push(pkg.fullname(true).into());
        }
    } else {
        for pkg in pkgs {
            let pkgname = pkg.fullname(true);
            if !pkgname.starts_with(current) {
                continue;
            }
            completions.push(pkgname.into());
        }
    }
}

pub fn pkgnames_remove_completions(current: &std::ffi::OsStr) -> Vec<CompletionCandidate> {
    let mut completions = vec![];
    let current = current.to_string_lossy();

    pkgnames_complete_impl(
        &mut completions,
        &current,
        PackageSort::default().names().installed(),
    );

    completions
}
