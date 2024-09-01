use std::{
    env,
    path::Path,
    process::{exit, Command},
};

use crate::error::OutputError;
use crate::fl;
use anyhow::anyhow;
use dialoguer::{theme::ColorfulTheme, Confirm};
use oma_utils::{
    dbus::{create_dbus_connection, is_using_battery, take_wake_lock, Connection},
    oma::unlock_oma,
    zbus::zvariant::OwnedFd,
};
use rustix::process;
use tokio::runtime::Runtime;
use tracing::{info, warn};


type Result<T> = std::result::Result<T, OutputError>;

pub fn root() -> Result<()> {
    if process::geteuid().is_root() {
        return Ok(());
    }

    let args = std::env::args().collect::<Vec<_>>();
    let mut handled_args = vec![];

    if (env::var("DISPLAY").is_ok() || env::var("WAYLAND_DISPLAY").is_ok()) && !is_wsl() {
        // Workaround: 使用 pkexec 执行其它程序时，若你指定了相对路径
        // pkexec 并不会以当前路径作为起点寻求这个位置
        // 所以需要转换成绝对路径，再喂给 pkexec
        for arg in args {
            let mut arg = arg.to_string();
            if arg.ends_with(".deb") {
                let path = Path::new(&arg);
                let path = path.canonicalize().unwrap_or(path.to_path_buf());
                arg = path.display().to_string();
            }
            handled_args.push(arg);
        }

        info!("{}", fl!("pkexec-tips-1"));
        info!("{}", fl!("pkexec-tips-2"));

        let out = Command::new("pkexec")
            .args(handled_args)
            .spawn()
            .and_then(|x| x.wait_with_output())
            .map_err(|e| anyhow!(fl!("execute-pkexec-fail", e = e.to_string())))?;

        unlock_oma().ok();
        exit(out.status.code().unwrap_or(1));
    }

    Err(OutputError {
        description: fl!("please-run-me-as-root"),
        source: None,
    })
}

pub fn create_async_runtime() -> Result<Runtime> {
    let tokio = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .map_err(|e| OutputError {
            description: "Failed to create async runtime".to_string(),
            source: Some(Box::new(e)),
        })?;

    Ok(tokio)
}

pub fn dbus_check(rt: &Runtime, yes: bool) -> Result<Vec<OwnedFd>> {
    let conn = rt.block_on(create_dbus_connection())?;
    rt.block_on(check_battery(&conn, yes));

    // 需要保留 fd
    // login1 根据 fd 来判断是否关闭 inhibit
    let fds = rt.block_on(take_wake_lock(&conn, &fl!("changing-system"), "oma"))?;

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

pub async fn check_battery(conn: &Connection, yes: bool) {
    let is_battery = is_using_battery(conn).await.unwrap_or(false);

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
