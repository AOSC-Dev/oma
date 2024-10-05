use std::sync::atomic::{AtomicU64, Ordering};

use ahash::RandomState;
use dialoguer::console::style;
use oma_console::{
    indicatif::{MultiProgress, ProgressBar},
    pb::{global_progress_bar_style, progress_bar_style, spinner_style},
    writer::bar_writeln,
    WRITER,
};
use oma_fetch::DownloadProgressControl;
use oma_refresh::db::{HandleRefresh, HandleTopicsControl};
use tracing::{error, info, warn};

use crate::fl;

type DashMap<K, V> = dashmap::DashMap<K, V, ahash::random_state::RandomState>;

pub struct OmaProgressBar {
    mb: MultiProgress,
    pb_map: DashMap<usize, ProgressBar>,
}

impl Default for OmaProgressBar {
    fn default() -> Self {
        Self {
            mb: MultiProgress::new(),
            pb_map: DashMap::with_hasher(RandomState::new()),
        }
    }
}

impl DownloadProgressControl for OmaProgressBar {
    fn checksum_mismatch_retry(&self, _index: usize, filename: &str, times: usize) {
        oma_console::writer::bar_writeln(
            |s| {
                self.mb.println(s).ok();
            },
            &oma_console::console::style("ERROR")
                .red()
                .bold()
                .to_string(),
            &fl!("checksum-mismatch-retry", c = filename, retry = times),
        );
    }

    fn global_progress_set(&self, num: &AtomicU64) {
        if let Some(gpb) = &self.pb_map.get(&0) {
            gpb.set_position(num.load(Ordering::SeqCst));
        }
    }

    fn progress_done(&self, index: usize) {
        if let Some(pb) = self.pb_map.get(&(index + 1)) {
            pb.finish_and_clear();
        }
    }

    fn new_progress_spinner(&self, index: usize, msg: &str) {
        let (sty, inv) = spinner_style();
        let pb = self
            .mb
            .insert(index + 1, ProgressBar::new_spinner().with_style(sty));
        pb.set_message(msg.to_string());
        pb.enable_steady_tick(inv);
        self.pb_map.insert(index + 1, pb);
    }

    fn new_progress_bar(&self, index: usize, msg: &str, size: u64) {
        let sty = progress_bar_style(&WRITER);
        let pb = self
            .mb
            .insert(index + 1, ProgressBar::new(size).with_style(sty));
        pb.set_message(msg.to_string());
        self.pb_map.insert(index + 1, pb);
    }

    fn progress_inc(&self, index: usize, num: u64) {
        if let Some(pb) = self.pb_map.get(&(index + 1)) {
            pb.inc(num);
        }
    }

    fn progress_set(&self, index: usize, num: u64) {
        if let Some(pb) = self.pb_map.get(&(index + 1)) {
            pb.set_position(num);
        }
    }

    fn failed_to_get_source_next_url(&self, _index: usize, err: &str) {
        oma_console::writer::bar_writeln(
            |s| {
                self.mb.println(s).ok();
            },
            &oma_console::console::style("ERROR")
                .red()
                .bold()
                .to_string(),
            &fl!("can-not-get-source-next-url", e = err.to_string()),
        );
    }

    fn download_done(&self, _index: usize, msg: &str) {
        tracing::debug!("Downloaded {msg}");
    }

    fn all_done(&self) {
        if let Some(gpb) = &self.pb_map.get(&0) {
            gpb.finish_and_clear();
        }
    }

    fn new_global_progress_bar(&self, total_size: u64) {
        let sty = global_progress_bar_style(&WRITER);
        let pb = self
            .mb
            .insert(0, ProgressBar::new(total_size).with_style(sty));
        self.pb_map.insert(0, pb);
    }
}

impl HandleTopicsControl for OmaProgressBar {
    fn scanning_topic(&self) {
        let (sty, inv) = spinner_style();
        let pb = self
            .mb
            .insert(1, ProgressBar::new_spinner().with_style(sty));
        pb.set_message(fl!("refreshing-topic-metadata"));
        pb.enable_steady_tick(inv);
        self.pb_map.insert(1, pb);
    }

    fn closing_topic(&self, topic: &str) {
        bar_writeln(
            |s| {
                self.mb.println(s).ok();
            },
            &style("INFO").blue().bold().to_string(),
            &fl!("scan-topic-is-removed", name = topic),
        );
    }

    fn topic_not_in_mirror(&self, topic: &str, mirror: &str) {
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
}

impl HandleRefresh for OmaProgressBar {
    fn run_invoke_script(&self) {
        let (sty, inv) = spinner_style();
        let pb = self
            .mb
            .insert(1, ProgressBar::new_spinner().with_style(sty));
        pb.set_message(fl!("oma-refresh-success-invoke"));
        pb.enable_steady_tick(inv);
        self.pb_map.insert(1, pb);
    }
}

pub struct NoProgressBar;

impl DownloadProgressControl for NoProgressBar {
    fn checksum_mismatch_retry(&self, _index: usize, filename: &str, times: usize) {
        error!(
            "{}",
            fl!("checksum-mismatch-retry", c = filename, retry = times)
        );
    }

    fn global_progress_set(&self, _num: &AtomicU64) {}

    fn progress_done(&self, _index: usize) {}

    fn new_progress_spinner(&self, _index: usize, _msg: &str) {}

    fn new_progress_bar(&self, _index: usize, _msg: &str, _size: u64) {}

    fn progress_inc(&self, _index: usize, _num: u64) {}

    fn progress_set(&self, _index: usize, _num: u64) {}

    fn failed_to_get_source_next_url(&self, _index: usize, err: &str) {
        error!(
            "{}",
            fl!("can-not-get-source-next-url", e = err.to_string())
        );
    }

    fn download_done(&self, _index: usize, msg: &str) {
        WRITER.writeln("DONE", msg).ok();
    }

    fn all_done(&self) {}

    fn new_global_progress_bar(&self, _total_size: u64) {}
}

impl HandleTopicsControl for NoProgressBar {
    fn scanning_topic(&self) {}

    fn closing_topic(&self, topic: &str) {
        info!("{}", fl!("scan-topic-is-removed", name = topic));
    }

    fn topic_not_in_mirror(&self, topic: &str, mirror: &str) {
        warn!(
            "{}",
            fl!("topic-not-in-mirror", topic = topic, mirror = mirror)
        );
        warn!("{}", fl!("skip-write-mirror"));
    }
}

impl HandleRefresh for NoProgressBar {
    fn run_invoke_script(&self) {
        info!("{}", fl!("oma-refresh-success-invoke"));
    }
}
