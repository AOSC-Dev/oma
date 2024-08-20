use std::sync::Arc;

use dashmap::DashMap;
use dialoguer::console::style;
use oma_console::{
    indicatif::{MultiProgress, ProgressBar},
    pb::{global_progress_bar_style, progress_bar_style, spinner_style},
    writer::bar_writeln,
    WRITER,
};
use oma_fetch::DownloadEvent;
use oma_refresh::db::RefreshEvent;
use tracing::{error, info, warn};

use crate::fl;

pub trait OmaProgress {
    fn change(&self, action: ProgressEvent, index: usize, total: Option<u64>);
}

pub struct OmaProgressBar {
    mb: MultiProgress,
    pub pb_map: Arc<DashMap<usize, ProgressBar>>,
}

pub enum ProgressEvent {
    DownloadEvent(DownloadEvent),
    RefreshEvent(RefreshEvent),
}

impl From<DownloadEvent> for ProgressEvent {
    fn from(value: DownloadEvent) -> Self {
        Self::DownloadEvent(value)
    }
}

impl From<RefreshEvent> for ProgressEvent {
    fn from(value: RefreshEvent) -> Self {
        Self::RefreshEvent(value)
    }
}

impl OmaProgress for OmaProgressBar {
    fn change(&self, action: ProgressEvent, index: usize, total: Option<u64>) {
        if let Some(total) = total {
            if self.pb_map.get(&0).is_none() {
                let sty = global_progress_bar_style(&WRITER);
                self.pb_map.insert(
                    0,
                    self.mb.insert(0, ProgressBar::new(total).with_style(sty)),
                );
            }
        }

        match action {
            ProgressEvent::DownloadEvent(action) => {
                self.handle_download_event(action, index);
            }
            ProgressEvent::RefreshEvent(action) => match action {
                RefreshEvent::ClosingTopic(topic_name) => bar_writeln(
                    |s| {
                        self.mb.println(s).ok();
                    },
                    &style("INFO").blue().bold().to_string(),
                    &fl!("scan-topic-is-removed", name = topic_name),
                ),
                RefreshEvent::DownloadEvent(event) => {
                    self.handle_download_event(event, index);
                }
                RefreshEvent::ScanningTopic => {
                    let (sty, inv) = spinner_style();
                    let pb = self
                        .mb
                        .insert(index + 1, ProgressBar::new_spinner().with_style(sty));
                    pb.set_message(fl!("refreshing-topic-metadata"));
                    pb.enable_steady_tick(inv);
                    self.pb_map.insert(index + 1, pb);
                }
                RefreshEvent::TopicNotInMirror(topic, mirror) => {
                    bar_writeln(
                        |s| {
                            self.mb.println(s).ok();
                        },
                        &style("WARNING").yellow().bold().to_string(),
                        &fl!("topic-not-in-mirror", topic = topic, mirror = mirror),
                    );
                    bar_writeln(
                        |s| {
                            self.mb.println(s).ok();
                        },
                        &style("WARNING").yellow().bold().to_string(),
                        &fl!("skip-write-mirror"),
                    );
                }
            },
        }
    }
}

impl OmaProgressBar {
    pub fn new() -> Self {
        let mb = MultiProgress::new();
        let pb_map = Arc::new(DashMap::new());

        Self { mb, pb_map }
    }

    fn handle_download_event(&self, action: DownloadEvent, index: usize) {
        match action {
            DownloadEvent::ChecksumMismatchRetry { filename, times } => {
                oma_console::writer::bar_writeln(
                    |s| {
                        self.mb.println(s).ok();
                    },
                    &oma_console::console::style("ERROR")
                        .red()
                        .bold()
                        .to_string(),
                    &fl!("checksum-mismatch-retry", c = filename, retry = times),
                )
            }
            DownloadEvent::GlobalProgressSet(n) => {
                if let Some(gpb) = &self.pb_map.get(&0) {
                    gpb.set_position(n);
                }
            }
            DownloadEvent::GlobalProgressInc(n) => {
                if let Some(gpb) = &self.pb_map.get(&0) {
                    gpb.inc(n);
                }
            }
            DownloadEvent::ProgressDone => {
                if let Some(pb) = self.pb_map.get(&(index + 1)) {
                    pb.finish_and_clear();
                }
            }
            DownloadEvent::NewProgressSpinner(msg) => {
                let (sty, inv) = spinner_style();
                let pb = self
                    .mb
                    .insert(index + 1, ProgressBar::new_spinner().with_style(sty));
                pb.set_message(msg);
                pb.enable_steady_tick(inv);
                self.pb_map.insert(index + 1, pb);
            }
            DownloadEvent::NewProgress(size, msg) => {
                let sty = progress_bar_style(&WRITER);
                let pb = self
                    .mb
                    .insert(index + 1, ProgressBar::new(size).with_style(sty));
                pb.set_message(msg);
                self.pb_map.insert(index + 1, pb);
            }
            DownloadEvent::ProgressInc(n) => {
                if let Some(pb) = self.pb_map.get(&(index + 1)) {
                    pb.inc(n);
                }
            }
            DownloadEvent::ProgressSet(n) => {
                if let Some(pb) = self.pb_map.get(&(index + 1)) {
                    pb.set_position(n);
                }
            }
            DownloadEvent::CanNotGetSourceNextUrl(e) => oma_console::writer::bar_writeln(
                |s| {
                    self.mb.println(s).ok();
                },
                &oma_console::console::style("ERROR")
                    .red()
                    .bold()
                    .to_string(),
                &fl!("can-not-get-source-next-url", e = e.to_string()),
            ),
            DownloadEvent::Done(filename) => {
                tracing::debug!("Downloaded {filename}");
            }
            DownloadEvent::AllDone => {
                if let Some(gpb) = &self.pb_map.get(&0) {
                    gpb.finish_and_clear();
                }
            }
        }
    }
}

pub struct NoProgressBar;

impl OmaProgress for NoProgressBar {
    fn change(&self, action: ProgressEvent, _index: usize, _total: Option<u64>) {
        match action {
            ProgressEvent::DownloadEvent(event) => match event {
                DownloadEvent::ChecksumMismatchRetry { filename, times } => {
                    error!(
                        "{}",
                        fl!("checksum-mismatch-retry", c = filename, retry = times)
                    );
                }
                DownloadEvent::CanNotGetSourceNextUrl(e) => {
                    error!("{}", fl!("can-not-get-source-next-url", e = e.to_string()));
                }
                DownloadEvent::Done(msg) => {
                    WRITER.writeln("DONE", &msg).ok();
                }
                _ => {}
            },
            ProgressEvent::RefreshEvent(event) => match event {
                RefreshEvent::DownloadEvent(event) => Self::handle_download_event(event),
                RefreshEvent::ClosingTopic(topic_name) => {
                    info!("{}", fl!("scan-topic-is-removed", name = topic_name));
                }
                RefreshEvent::ScanningTopic => {
                    info!("{}", fl!("refreshing-topic-metadata"));
                }
                RefreshEvent::TopicNotInMirror(topic, mirror) => {
                    warn!(
                        "{}",
                        fl!("topic-not-in-mirror", topic = topic, mirror = mirror)
                    );
                    warn!("{}", fl!("skip-write-mirror"));
                }
            },
        }
    }
}

impl NoProgressBar {
    fn handle_download_event(event: DownloadEvent) {
        match event {
            DownloadEvent::ChecksumMismatchRetry { filename, times } => {
                error!(
                    "{}",
                    fl!("checksum-mismatch-retry", c = filename, retry = times)
                );
            }
            DownloadEvent::CanNotGetSourceNextUrl(e) => {
                error!("{}", fl!("can-not-get-source-next-url", e = e.to_string()));
            }
            DownloadEvent::Done(msg) => {
                WRITER.writeln("DONE", &msg).ok();
            }
            _ => {}
        }
    }
}
