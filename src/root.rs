use std::{
    env,
    process::{Command, exit},
    sync::atomic::Ordering,
};

use anyhow::anyhow;
use oma_utils::is_termux;
use rustix::process;
use spdlog::info;

use crate::{NOT_ALLOW_CTRLC, error::OutputError, fl};

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
    let oma_envs_args = std::env::vars()
        .filter(|(k, _)| k.starts_with("OMA_"))
        .map(|(k, v)| format!("--setenv={k}={v}"));

    let out = Command::new("systemd-run")
        .env("SYSTEMD_ADJUST_TERMINAL_TITLE", "0")
        .arg("--same-dir")
        .arg("--pty")
        .arg("--quiet")
        .arg("--unit=oma")
        .args(oma_envs_args)
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
