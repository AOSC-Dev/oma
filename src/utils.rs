use std::{
    env,
    path::Path,
    process::{exit, Command},
    sync::{atomic::AtomicBool, Arc},
};

use crate::error::OutputError;
use crate::fl;
use anyhow::anyhow;
use dashmap::DashMap;
use dialoguer::{theme::ColorfulTheme, Confirm};
use oma_console::indicatif::{MultiProgress, ProgressBar};
use oma_utils::{
    dbus::{create_dbus_connection, is_using_battery, take_wake_lock, Connection},
    oma::unlock_oma,
    zbus::zvariant::OwnedFd,
};
use rustix::process;
use tokio::runtime::Runtime;
use tracing::warn;

type Result<T> = std::result::Result<T, OutputError>;

// 似乎用函数无法修改 Dashmap 的内容，不知道该怎么办，所以用了宏
/// Control progress bar
#[macro_export]
macro_rules! pb {
    ($event:expr, $mb:expr, $pb_map:expr, $count:expr, $total:expr, $global_is_set:expr) => {{
        tracing::debug!("{}", $event);
        match $event {
            oma_pm::apt::InstallPackageEvent::DownloadEvent(event) => {
                match event {
                    oma_fetch::DownloadEvent::ChecksumMismatchRetry { filename, times } => {
                        // println 返回的错误是无法控制终端的 I/O 错误，这种处理应该直接 panic 返回
                        // 所以这里直接 unwrap
                        // （下同）
                        oma_console::writer::bar_writeln(|s| { $mb.println(s).ok(); },
                            &oma_console::console::style("ERROR")
                                .red()
                                .bold()
                                .to_string(),
                            &fl!("checksum-mismatch-retry", c = filename, retry = times))
                    }
                    oma_fetch::DownloadEvent::GlobalProgressSet(size) => {
                        if let Some(pb) = $pb_map.get(&0) {
                            pb.set_position(size);
                        }
                    }
                    oma_fetch::DownloadEvent::GlobalProgressInc(size) => {
                        if let Some(pb) = $pb_map.get(&0) {
                            pb.inc(size);
                        }
                    }
                    oma_fetch::DownloadEvent::ProgressDone => {
                        if let Some(pb) = $pb_map.get(&($count + 1)) {
                            pb.finish_and_clear();
                        }
                    }
                    oma_fetch::DownloadEvent::NewProgressSpinner(msg) => {
                        let (sty, inv) = oma_console::pb::spinner_style();
                        let pb = $mb.insert(
                            $count + 1,
                            oma_console::indicatif::ProgressBar::new_spinner().with_style(sty),
                        );
                        pb.set_message(msg);
                        pb.enable_steady_tick(inv);
                        $pb_map.insert($count + 1, pb);
                    }
                    oma_fetch::DownloadEvent::NewProgress(size, msg) => {
                        let sty =
                            oma_console::pb::progress_bar_style(&oma_console::WRITER);
                        let pb = $mb.insert(
                            $count + 1,
                            oma_console::indicatif::ProgressBar::new(size).with_style(sty),
                        );
                        pb.set_message(msg);
                        $pb_map.insert($count + 1, pb);
                    }
                    oma_fetch::DownloadEvent::ProgressInc(size) => {
                        let pb = $pb_map.get(&($count + 1)).unwrap();
                        pb.inc(size);
                    }
                    oma_fetch::DownloadEvent::ProgressSet(size) => {
                        let pb = $pb_map.get(&($count + 1)).unwrap();
                        pb.set_position(size);
                    }
                    oma_fetch::DownloadEvent::CanNotGetSourceNextUrl(e) => {
                        oma_console::writer::bar_writeln(|s| { $mb.println(s).ok(); },
                        &oma_console::console::style("ERROR")
                        .red()
                        .bold()
                        .to_string(),
                    &fl!("can-not-get-source-next-url", e = e.to_string()), )
                    }
                    oma_fetch::DownloadEvent::Done(filename) => {
                        tracing::debug!("Downloaded {filename}");
                    }
                    oma_fetch::DownloadEvent::AllDone => {
                        if let Some(gpb) = $pb_map.get(&0) {
                            gpb.finish_and_clear();
                        }
                        $pb_map.remove(&0);
                    }
                }
            }
            oma_pm::apt::InstallPackageEvent::DpkgEvent(event) => {
                let gpb = $pb_map.get(&0);
                if gpb.is_none() {
                    $global_is_set.store(true, std::sync::atomic::Ordering::SeqCst);
                    let sty = oma_console::pb::global_progress_bar_style(&oma_console::WRITER);
                    let gpb = $mb.insert(
                        0,
                        oma_console::indicatif::ProgressBar::new(100).with_style(sty),
                    );
                    gpb.set_prefix("Progress");
                    gpb.set_position(event.percent);
                    $pb_map.insert(0, gpb);
                    return;
                }
                let gpb = gpb.unwrap();
                gpb.set_position(event.percent);
            }
            oma_pm::apt::InstallPackageEvent::DpkgLine(line) => {
                $mb.println(line).ok();
            }
        }

        if let Some(total) = $total {
            if !$global_is_set.load(std::sync::atomic::Ordering::SeqCst) {
                let sty = oma_console::pb::global_progress_bar_style(&oma_console::WRITER);
                let gpb = $mb.insert(
                    0,
                    oma_console::indicatif::ProgressBar::new(total).with_style(sty),
                );
                gpb.set_prefix("Progress");
                $pb_map.insert(0, gpb);
                $global_is_set.store(true, std::sync::atomic::Ordering::SeqCst);
            }
        }
    }};
}

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

pub fn multibar() -> (
    Arc<MultiProgress>,
    DashMap<usize, ProgressBar>,
    Arc<AtomicBool>,
) {
    let mb = Arc::new(MultiProgress::new());
    let pb_map: DashMap<usize, ProgressBar> = DashMap::new();

    let global_is_set = Arc::new(AtomicBool::new(false));

    (mb, pb_map, global_is_set)
}
