use std::{io::ErrorKind, process::exit};

use dialoguer::{Confirm, theme::ColorfulTheme};
use oma_utils::{
    dbus::{
        InhibitTypeUnion, create_dbus_connection, get_another_oma_status, is_using_battery,
        session_name, take_wake_lock,
    },
    is_termux,
    zbus::{Connection, zvariant::OwnedFd},
};
use spdlog::{debug, error, info, warn};

use crate::{
    RT,
    config::OmaConfig,
    config_file::{BatteryTristate, TakeWakeLockTristate},
    error::OutputError,
    fl,
};
type Result<T> = std::result::Result<T, OutputError>;

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

pub fn is_ssh_from_loginctl() -> bool {
    let conn = RT.block_on(create_dbus_connection());

    if let Ok(conn) = conn {
        let session_name = RT.block_on(session_name(&conn));
        debug!("session_name: {:?}", session_name);

        return session_name.is_ok_and(|name| name == "system-remote-login");
    }

    false
}

fn no_check_dbus_warn() {
    warn!("{}", fl!("no-check-dbus-tips"));
}

#[inline]
fn check_battery_disabled_warn() {
    warn!("{}", fl!("battery-check-disabled"));
}

#[inline]
fn no_take_wake_lock_warn() {
    warn!("{}", fl!("session-check-disabled"));
}

fn connect_dbus_impl() -> Option<Connection> {
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

fn ask_continue_no_use_battery(conn: &Connection, yes: bool) {
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

fn is_battery(conn: &Connection) -> bool {
    RT.block_on(is_using_battery(conn)).unwrap_or(false)
}

pub fn find_another_oma() -> Result<String> {
    RT.block_on(async { find_another_oma_inner().await })
}

async fn find_another_oma_inner() -> Result<String> {
    let conn = create_dbus_connection().await?;
    let status = get_another_oma_status(&conn).await?;

    let status = match status.as_str() {
        "Pending" => fl!("status-pending"),
        "Downloading" => fl!("status-downloading"),
        pkg => fl!("status-package", pkg = pkg),
    };

    Ok(status)
}
