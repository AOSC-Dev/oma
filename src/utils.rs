use std::{
    path::Path,
    process::{exit, Command},
    sync::{atomic::AtomicBool, Arc},
};

use crate::error::OutputError;
use crate::fl;
use anyhow::anyhow;
use dashmap::DashMap;
use dialoguer::{theme::ColorfulTheme, Confirm};
use oma_console::{
    indicatif::{MultiProgress, ProgressBar},
    warn,
};
use oma_utils::dbus::{create_dbus_connection, is_using_battery, take_wake_lock, Connection};
use tokio::runtime::Runtime;

type Result<T> = std::result::Result<T, OutputError>;

// 似乎用函数无法修改 Dashmap 的内容，不知道该怎么办，所以用了宏
/// Control progress bar
#[macro_export]
macro_rules! pb {
    ($event:expr, $mb:expr, $pb_map:expr, $count:expr, $total:expr, $global_is_set:expr) => {{
        match $event {
            oma_fetch::DownloadEvent::ChecksumMismatchRetry { filename, times } => {
                oma_console::WRITER
                    .writeln_with_mb(
                        &*$mb,
                        &oma_console::console::style("ERROR")
                            .red()
                            .bold()
                            .to_string(),
                        &fl!("checksum-mismatch-retry", c = filename, retry = times),
                    )
                    .unwrap();
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
                let (sty, inv) = oma_console::pb::oma_spinner(false).unwrap();
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
                    oma_console::pb::oma_style_pb(oma_console::writer::Writer::default(), false)
                        .unwrap();
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
            oma_fetch::DownloadEvent::CanNotGetSourceNextUrl(e) => {
                oma_console::WRITER
                    .writeln_with_mb(
                        &*$mb,
                        &oma_console::console::style("ERROR")
                            .red()
                            .bold()
                            .to_string(),
                        &fl!("can-not-get-source-next-url", e = e.to_string()),
                    )
                    .unwrap();
            }
        }

        if let Some(total) = $total {
            if !$global_is_set.load(std::sync::atomic::Ordering::SeqCst) {
                let sty =
                    oma_console::pb::oma_style_pb(oma_console::writer::Writer::default(), true)
                        .unwrap();
                let gpb = $mb.insert(
                    0,
                    oma_console::indicatif::ProgressBar::new(total).with_style(sty),
                );
                gpb.set_message("Progress");
                $pb_map.insert(0, gpb);
                $global_is_set.store(true, std::sync::atomic::Ordering::SeqCst);
            }
        }
    }};
}

pub fn root() -> Result<()> {
    if nix::unistd::geteuid().is_root() {
        return Ok(());
    }

    let args = std::env::args().collect::<Vec<_>>();
    let mut handled_args = vec![];

    // Handle pkexec user input path
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

    exit(
        out.status
            .code()
            .expect("Can not get pkexec oma exit status"),
    );
}

pub fn create_async_runtime() -> Result<Runtime> {
    let tokio = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()?;

    Ok(tokio)
}

pub fn dbus_check(rt: &Runtime) -> Result<()> {
    let conn = rt.block_on(create_dbus_connection())?;
    rt.block_on(check_battery(&conn))?;
    rt.block_on(take_wake_lock(&conn, &fl!("changing-system"), "oma"))?;

    Ok(())
}

pub async fn check_battery(conn: &Connection) -> Result<()> {
    let is_battery = is_using_battery(conn).await.unwrap_or(false);

    if is_battery {
        let theme = ColorfulTheme::default();
        warn!("{}", fl!("battery"));
        let cont = Confirm::with_theme(&theme)
            .with_prompt(fl!("continue"))
            .default(false)
            .interact()?;

        if !cont {
            exit(0);
        }
    }

    Ok(())
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
