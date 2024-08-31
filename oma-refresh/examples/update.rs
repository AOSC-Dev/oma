use std::{
    path::{Path, PathBuf},
    result::Result,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
};

use dashmap::DashMap;
use indicatif::{MultiProgress, ProgressBar};
use oma_apt::config::Config;
use oma_console::{
    pb::{global_progress_bar_style, progress_bar_style, spinner_style},
    writer::Writer,
};
use oma_fetch::{reqwest::ClientBuilder, DownloadEvent};
use oma_refresh::db::{OmaRefresh, OmaRefreshBuilder, RefreshError, RefreshEvent};
use oma_utils::dpkg::dpkg_arch;

#[tokio::main]
async fn main() -> Result<(), RefreshError> {
    let p = Path::new("./oma-fetcher-test");
    tokio::fs::create_dir_all(p).await.unwrap();
    let client = ClientBuilder::new().user_agent("oma").build().unwrap();

    let apt_config = Config::new();

    let refresher: OmaRefresh = OmaRefreshBuilder {
        client: &client,
        source: PathBuf::from("/"),
        limit: Some(4),
        arch: dpkg_arch("/").unwrap(),
        download_dir: p.to_path_buf(),
        refresh_topics: false,
        apt_config: &apt_config,
    }
    .into();

    let mb = Arc::new(MultiProgress::new());
    let pb_map: DashMap<usize, ProgressBar> = DashMap::new();

    let global_is_set = Arc::new(AtomicBool::new(false));

    refresher
        .start(
            move |count, event, total| {
                match event {
                    RefreshEvent::ClosingTopic(topic_name) => {
                        mb.println(format!("Closing topic {topic_name}")).unwrap();
                    }
                    RefreshEvent::DownloadEvent(event) => match event {
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
                            let (sty, inv) = spinner_style();
                            let pb =
                                mb.insert(count + 1, ProgressBar::new_spinner().with_style(sty));
                            pb.set_message(msg);
                            pb.enable_steady_tick(inv);
                            pb_map.insert(count + 1, pb);
                        }
                        DownloadEvent::NewProgress(size, msg) => {
                            let writer = Writer::default();
                            let sty = progress_bar_style(&writer);
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
                            if let Some(gpb) = pb_map.get(&0) {
                                gpb.finish_and_clear();
                            }
                        }
                    },
                    RefreshEvent::ScanningTopic => {
                        mb.println("Scanning topic...").unwrap();
                    }
                    RefreshEvent::TopicNotInMirror(topic, mirror) => {
                        mb.println(format!("{topic} not in {mirror}")).unwrap();
                    }
                }

                if let Some(total) = total {
                    if !global_is_set.load(Ordering::SeqCst) {
                        let writer = Writer::default();
                        let sty = global_progress_bar_style(&writer);
                        let gpb = mb.insert(0, ProgressBar::new(total).with_style(sty));
                        pb_map.insert(0, gpb);
                        global_is_set.store(true, Ordering::SeqCst);
                    }
                }
            },
            || "test".to_string(),
        )
        .await
        .unwrap();

    Ok(())
}
