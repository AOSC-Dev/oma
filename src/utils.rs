use std::{
    env,
    fmt::Display,
    path::Path,
    process::{exit, Command},
    sync::atomic::Ordering,
};

use crate::{color_formatter, fl, RT, WRITER};
use crate::{error::OutputError, SPAWN_NEW_OMA};
use anyhow::anyhow;
use dialoguer::{console::style, theme::ColorfulTheme, Confirm};
use oma_console::{
    print::Action,
    writer::{gen_prefix, writeln_inner, MessageType},
};
use oma_pm::{search::SearchResult, PackageStatus};
use oma_utils::{
    dbus::{create_dbus_connection, is_using_battery, session_name, take_wake_lock, Connection},
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

        SPAWN_NEW_OMA.store(true, Ordering::Relaxed);

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

pub struct SearchResultDisplay<'a>(pub &'a SearchResult);

impl Display for SearchResultDisplay<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let i = self.0;
        #[cfg(feature = "aosc")]
        let mut pkg_info_line = if i.is_base {
            color_formatter()
                .color_str(&i.name, Action::Purple)
                .bold()
                .to_string()
        } else {
            color_formatter()
                .color_str(&i.name, Action::Emphasis)
                .bold()
                .to_string()
        };

        #[cfg(not(feature = "aosc"))]
        let mut pkg_info_line = color_formatter()
            .color_str(&i.name, Action::Emphasis)
            .bold()
            .to_string();

        pkg_info_line.push(' ');

        if i.status == PackageStatus::Upgrade {
            pkg_info_line.push_str(&format!(
                "{} -> {}",
                color_formatter().color_str(i.old_version.as_ref().unwrap(), Action::WARN),
                color_formatter().color_str(&i.new_version, Action::EmphasisSecondary)
            ));
        } else {
            pkg_info_line.push_str(
                &color_formatter()
                    .color_str(&i.new_version, Action::EmphasisSecondary)
                    .to_string(),
            );
        }

        let mut pkg_tags = vec![];

        if i.dbg_package {
            pkg_tags.push(fl!("debug-symbol-available"));
        }

        if i.full_match {
            pkg_tags.push(fl!("full-match"))
        }

        if !pkg_tags.is_empty() {
            pkg_info_line.push(' ');
            pkg_info_line.push_str(
                &color_formatter()
                    .color_str(format!("[{}]", pkg_tags.join(",")), Action::Note)
                    .to_string(),
            );
        }

        let prefix = match i.status {
            PackageStatus::Avail => style("AVAIL").dim(),
            PackageStatus::Installed => {
                color_formatter().color_str("INSTALLED", Action::Foreground)
            }
            PackageStatus::Upgrade => color_formatter().color_str("UPGRADE", Action::WARN),
        }
        .to_string();

        writeln!(f, "{}{}", gen_prefix(&prefix, 10), pkg_info_line)?;

        writeln_inner(
            &i.desc,
            "",
            WRITER.get_max_len().into(),
            WRITER.get_prefix_len(),
            |t, s| {
                match t {
                    MessageType::Msg => {
                        writeln!(
                            f,
                            "{}",
                            color_formatter().color_str(s.trim(), Action::Secondary)
                        )
                    }
                    MessageType::Prefix => write!(f, "{}", gen_prefix(s, 10)),
                }
                .ok();
            },
        );

        Ok(())
    }
}

/// oma display normal message
#[macro_export]
macro_rules! msg {
    ($($arg:tt)+) => {
        use oma_console::writer::Writeln as _;
        $crate::WRITER.writeln("", &format!($($arg)+)).ok();
    };
}

/// oma display success message
#[macro_export]
macro_rules! success {
    ($($arg:tt)+) => {
        use oma_console::writer::Writeln as _;
        $crate::WRITER.writeln(&oma_console::console::style("SUCCESS").green().bold().to_string(), &format!($($arg)+)).ok();
    };
}

/// oma display due_to message
#[macro_export]
macro_rules! due_to {
    ($($arg:tt)+) => {
        use oma_console::writer::Writeln as _;
        $crate::WRITER.writeln(&oma_console::console::style("DUE TO").yellow().bold().to_string(), &format!($($arg)+)).ok();
    };
}
