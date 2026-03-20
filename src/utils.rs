use std::{io::ErrorKind, path::PathBuf, process::exit};

use crate::{RT, fl};
use crate::{
    config::OmaConfig,
    config_file::{BatteryTristate, TakeWakeLockTristate},
    error::OutputError,
    path_completions::PathCompleter,
    subcommand::utils::no_check_dbus_warn,
};

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
use spdlog::{debug, error, info, warn};

type Result<T> = std::result::Result<T, OutputError>;

#[inline]
pub fn get_lists_dir(config: &AptConfig) -> PathBuf {
    PathBuf::from(config.dir("Dir::State::lists", "lists/"))
}

pub fn dbus_check(yes: bool, config: &OmaConfig) -> Result<Option<OwnedFd>> {
    if config.no_check_dbus {
        no_check_dbus_warn();
        return Ok(None);
    }

    if is_termux() {
        return Ok(None);
    }

    let Some(conn) = connect_dbus_impl() else {
        return Ok(None);
    };

    match config.check_battery {
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

    // 需要保留 fd
    // login1 根据 fd 来判断是否关闭 inhibit
    match config.take_wake_lock {
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
                exit(0);
            }
        }
        Err(e) => {
            let dialoguer::Error::IO(e) = e;
            if e.kind() != ErrorKind::Interrupted {
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
