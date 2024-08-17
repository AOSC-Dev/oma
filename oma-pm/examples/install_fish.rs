use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
};

use dashmap::DashMap;
use indicatif::{MultiProgress, ProgressBar};
use oma_console::{
    pb::{oma_spinner, oma_style_pb},
    writer::Writer,
};
use oma_fetch::{reqwest::ClientBuilder, DownloadEvent};
use oma_pm::apt::{AptArgs, OmaApt, OmaAptArgsBuilder, OmaAptError};

fn main() -> Result<(), OmaAptError> {
    let oma_apt_args = OmaAptArgsBuilder::default().build().unwrap();
    let mut apt = OmaApt::new(vec![], oma_apt_args, false)?;
    let pkgs = apt.select_pkg(&vec!["fish"], false, true, true)?;

    let mb = Arc::new(MultiProgress::new());
    let pb_map: DashMap<usize, ProgressBar> = DashMap::new();

    let global_is_set = Arc::new(AtomicBool::new(false));

    apt.install(&pkgs.0, false)?;

    let client = ClientBuilder::new().user_agent("oma").build().unwrap();

    apt.resolve(false, true)?;

    let op = apt.summary(|_| false)?;
    apt.commit(
        &client,
        None,
        &AptArgs::default(),
        |count, event, total| {
            match event {
                oma_pm::apt::InstallPackageEvent::DownloadEvent(event) => match event {
                    DownloadEvent::ChecksumMismatchRetry { filename, times } => {
                        mb.println(format!(
                            "{filename} checksum failed, retrying {times} times"
                        ))
                        .unwrap();
                    }
                    DownloadEvent::GlobalProgressSet(size) => {
                        if let Some(pb) = pb_map.get(&0) {
                            pb.set_position(size);
                        }
                    }
                    DownloadEvent::GlobalProgressInc(size) => {
                        if let Some(pb) = pb_map.get(&0) {
                            pb.inc(size);
                        }
                    }
                    DownloadEvent::ProgressDone => {
                        if let Some(pb) = pb_map.get(&(count + 1)) {
                            pb.finish_and_clear();
                        }
                    }
                    DownloadEvent::NewProgressSpinner(msg) => {
                        let (sty, inv) = oma_spinner(false);
                        let pb = mb.insert(count + 1, ProgressBar::new_spinner().with_style(sty));
                        pb.set_message(msg);
                        pb.enable_steady_tick(inv);
                        pb_map.insert(count + 1, pb);
                    }
                    DownloadEvent::NewProgress(size, msg) => {
                        let sty = oma_style_pb(Writer::default(), false);
                        let pb = mb.insert(count + 1, ProgressBar::new(size).with_style(sty));
                        pb.set_message(msg);
                        pb_map.insert(count + 1, pb);
                    }
                    DownloadEvent::ProgressInc(size) => {
                        let pb = pb_map.get(&(count + 1)).unwrap();
                        pb.inc(size);
                    }
                    DownloadEvent::ProgressSet(size) => {
                        let pb = pb_map.get(&(count + 1)).unwrap();
                        pb.set_position(size);
                    }
                    DownloadEvent::CanNotGetSourceNextUrl(e) => {
                        mb.println(format!("Error: {e}")).unwrap();
                    }
                    DownloadEvent::Done(_) => {
                        return;
                    }
                    DownloadEvent::AllDone => {
                        pb_map.get(&0).unwrap().finish_and_clear();
                    }
                },
                oma_pm::apt::InstallPackageEvent::DpkgEvent(event) => {
                    dbg!(event);
                }
                oma_pm::apt::InstallPackageEvent::DpkgLine(line) => {
                    dbg!(line);
                }
            }
            if let Some(total) = total {
                if !global_is_set.load(Ordering::SeqCst) {
                    let sty = oma_style_pb(Writer::default(), true);
                    let gpb = mb.insert(0, ProgressBar::new(total).with_style(sty));
                    pb_map.insert(0, gpb);
                    global_is_set.store(true, Ordering::SeqCst);
                }
            }
        },
        op,
    )?;

    Ok(())
}
