use std::{
    env,
    path::Path,
    process::{Command, exit},
    sync::atomic::Ordering,
};

use anyhow::anyhow;
use oma_logger::{debug, info};
use oma_utils::is_termux;
use rustix::fs::statfs;
use rustix::process;
use zbus::{Connection, fdo::DBusProxy, names::BusName};

use crate::{NOT_ALLOW_CTRLC, RT, error::OutputError, fl};

type Result<T> = std::result::Result<T, OutputError>;

pub fn root(no_check_dbus: bool) -> Result<()> {
    if is_termux() {
        return Ok(());
    }

    if is_root() {
        return Ok(());
    }

    NOT_ALLOW_CTRLC.store(true, Ordering::Relaxed);

    // FUSE 挂载默认不允许 root 进入，提权时不能保留 cwd；
    // 此时 cwd 会被重新设置为 `/`，相对 deb 路径没有意义，直接拒绝。
    let keep_cwd = !cwd_is_fuse();
    if !keep_cwd && has_relative_deb_arg() {
        return Err(OutputError::msg(fl!("fuse-cwd-relative-deb")));
    }

    match env::var("OMA_PERM_TOOL").as_deref() {
        Ok("sdr") => systemd_run_oma(keep_cwd)?,
        Ok("pkexec") => pkexec_oma()?,
        Ok(x) => other_permission_tools_run_oma(x)?,
        Err(_) => {}
    }

    let is_wsl = is_wsl();

    // Fix issue https://github.com/AOSC-Dev/oma/issues/609
    // `systemd-run --pty` needs a running systemd-logind to attach the
    // command to a login session; the binary may exist even when logind
    // isn't running (e.g. a systemd container with logind disabled), so
    // require logind before trying it and let the pkexec/sudo/doas branches
    // take over otherwise.
    if which::which("systemd-run").is_ok() && !is_wsl && !no_check_dbus && is_logind_running() {
        systemd_run_oma(keep_cwd)?;
    } else if is_desktop_env() && !is_wsl && which::which("pkexec").is_ok() {
        // 检测是否有 DISPLAY，如果有，则在提权时使用 pkexec
        // 通常情况下 SSH 连接不会有 DISPLAY 环境变量，除非开启 X11 Forwarding
        pkexec_oma()?;
    } else if which::which("sudo").is_ok() {
        other_permission_tools_run_oma("sudo")?;
    } else if which::which("doas").is_ok() {
        other_permission_tools_run_oma("doas")?;
    }

    Err(OutputError::msg(fl!("please-run-me-as-root")))
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
        .map_err(|e| anyhow!(fl!("execute-cmd-fail", cmd = cmd, e = e)))?;

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
        .map_err(|e| anyhow!(fl!("execute-cmd-fail", cmd = "pkexec", e = e)))?;

    exit(out.status.code().unwrap_or(1));
}

/// Environment variables that oma reads and should be forwarded to the
/// elevated process when using `systemd-run` (which starts services with a
/// minimal environment). All `OMA_*` variables are forwarded automatically.
const OMA_KNOWN_ENVS: &[&str] = &[
    // logging
    "RUST_LOG",
    // history "Requested-By" logging
    "SUDO_USER",
    "SUDO_UID",
    // platform detection
    "TERMUX_VERSION",
    "CI",
    // color / terminal detection
    "NO_COLOR",
    "SSH_CONNECTION",
    "TERM",
    "DISPLAY",
    "WAYLAND_DISPLAY",
    // i18n (sys-locale)
    "LANGUAGE",
    "LC_ALL",
    "LC_MESSAGES",
    "LANG",
];

fn systemd_run_oma(keep_cwd: bool) -> Result<()> {
    let envs_args = std::env::vars()
        .filter(|(k, _)| k.starts_with("OMA_") || OMA_KNOWN_ENVS.contains(&k.as_str()))
        .map(|(k, v)| format!("--setenv={k}={v}"));

    let mut cmd = Command::new("systemd-run");
    cmd.env("SYSTEMD_ADJUST_TERMINAL_TITLE", "0")
        .arg("--pty")
        .arg("--quiet")
        .arg("--unit=oma")
        .args(envs_args)
        .arg("--collect");

    if keep_cwd {
        cmd.arg("--same-dir");
    } else {
        // systemd-run 的 system service 默认在 `/` 下运行，这里显式写出来
        cmd.arg("--working-directory=/");
    }

    let out = cmd
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

/// FUSE_SUPER_MAGIC
const FUSE_SUPER_MAGIC: u64 = 0x6573_5546;

/// 当前工作目录是否位于 FUSE 挂载上。
///
/// FUSE 默认只允许挂载者本人访问，root 也会被拒绝；提权时若保留 cwd，
/// systemd 会以 root 身份 chdir 进该目录并失败。
fn cwd_is_fuse() -> bool {
    matches!(statfs("."), Ok(s) if s.f_type as u64 == FUSE_SUPER_MAGIC)
}

/// 提权后 cwd 可能被换成 `/`，相对 `.deb` 路径会被重新解析，必须拒绝。
fn has_relative_deb_arg() -> bool {
    env::args()
        .skip(1)
        .any(|a| a.ends_with(".deb") && Path::new(&a).is_relative())
}

// From `is_wsl` crate
fn is_wsl() -> bool {
    let Ok(kernel_info) = std::fs::read_to_string("/proc/sys/kernel/osrelease") else {
        return false;
    };

    let kernel_info = kernel_info.to_ascii_lowercase();

    kernel_info.contains("microsoft") || kernel_info.contains("wsl")
}

fn is_logind_running() -> bool {
    let running = RT.block_on(async {
        let Ok(conn) = Connection::system().await else {
            return false;
        };

        let Ok(dbus) = DBusProxy::new(&conn).await else {
            return false;
        };

        let Ok(name) = BusName::try_from("org.freedesktop.login1") else {
            return false;
        };

        dbus.name_has_owner(name).await.unwrap_or(false)
    });

    if !running {
        debug!("systemd-logind is not running, falling back to other privilege tools");
    }

    running
}
