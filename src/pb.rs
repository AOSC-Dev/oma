use std::{
    borrow::Cow,
    cell::OnceCell,
    sync::{
        atomic::{AtomicU64, Ordering},
        RwLock,
    },
    time::{Duration, Instant},
};

use ahash::{HashMap, RandomState};
use oma_console::{
    console::style,
    indicatif::{MultiProgress, ProgressBar},
    msg,
    pb::{global_progress_bar_style, progress_bar_style, spinner_style},
    writer::{gen_prefix, writeln_inner, MessageType, Writeln},
    WRITER,
};
use oma_fetch::DownloadProgressControl;
use oma_refresh::db::{HandleRefresh, HandleTopicsControl};
use oma_utils::human_bytes::HumanBytes;
use tracing::{error, info, warn};

use crate::fl;

pub struct OmaProgressBar {
    pub inner: ProgressBar,
}

impl OmaProgressBar {
    pub fn new_spinner(spinner_message: Option<impl Into<Cow<'static, str>>>) -> Self {
        let pb = ProgressBar::new_spinner();
        let (sty, inv) = spinner_style();
        pb.set_style(sty);
        pb.enable_steady_tick(inv);

        if let Some(msg) = spinner_message {
            pb.set_message(msg);
        }

        Self { inner: pb }
    }

    #[allow(dead_code)]
    pub fn new(pb: ProgressBar) -> Self {
        Self { inner: pb }
    }
}

impl Writeln for OmaProgressBar {
    fn writeln(&self, prefix: &str, msg: &str) -> std::io::Result<()> {
        let max_len = WRITER.get_max_len();
        let mut output = (None, None);

        writeln_inner(msg, prefix, max_len as usize, WRITER.prefix_len, |t, s| {
            match t {
                MessageType::Msg => {
                    let s: Box<str> = Box::from(s);
                    output.1 = Some(s)
                }
                MessageType::Prefix => output.0 = Some(gen_prefix(s, 10)),
            }

            if let (Some(prefix), Some(msg)) = &output {
                self.inner.println(format!("{prefix}{msg}"));
                output = (None, None);
            }
        });

        Ok(())
    }
}

pub struct OmaMultiProgressBar {
    mb: MultiProgress,
    pb_map: RwLock<HashMap<usize, ProgressBar>>,
}

impl Default for OmaMultiProgressBar {
    fn default() -> Self {
        Self {
            mb: MultiProgress::new(),
            pb_map: RwLock::new(HashMap::with_hasher(RandomState::new())),
        }
    }
}

impl Writeln for OmaMultiProgressBar {
    fn writeln(&self, prefix: &str, msg: &str) -> std::io::Result<()> {
        let max_len = WRITER.get_max_len();
        let mut output = (None, None);

        let mut result = Ok(());

        writeln_inner(msg, prefix, max_len as usize, WRITER.prefix_len, |t, s| {
            match t {
                MessageType::Msg => {
                    let s: Box<str> = Box::from(s);
                    output.1 = Some(s)
                }
                MessageType::Prefix => output.0 = Some(gen_prefix(s, 10)),
            }

            if let (Some(prefix), Some(msg)) = &output {
                result = self.mb.println(format!("{prefix}{msg}"));
                output = (None, None);
            }
        });

        result
    }
}

impl DownloadProgressControl for OmaMultiProgressBar {
    fn checksum_mismatch_retry(&self, _index: usize, filename: &str, times: usize) {
        self.writeln(
            &style("ERROR").red().bold().to_string(),
            &fl!("checksum-mismatch-retry", c = filename, retry = times),
        )
        .ok();
    }

    fn global_progress_set(&self, num: &AtomicU64) {
        if let Some(gpb) = self.pb_map.read().unwrap().get(&0) {
            gpb.set_position(num.load(Ordering::SeqCst));
        }
    }

    fn progress_done(&self, index: usize) {
        if let Some(pb) = self.pb_map.read().unwrap().get(&(index + 1)) {
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
        self.pb_map.write().unwrap().insert(index + 1, pb);
    }

    fn new_progress_bar(&self, index: usize, msg: &str, size: u64) {
        let sty = progress_bar_style(&WRITER);
        let pb = self
            .mb
            .insert(index + 1, ProgressBar::new(size).with_style(sty));
        pb.set_message(msg.to_string());
        self.pb_map.write().unwrap().insert(index + 1, pb);
    }

    fn progress_inc(&self, index: usize, num: u64) {
        if let Some(pb) = self.pb_map.read().unwrap().get(&(index + 1)) {
            pb.inc(num);
        }
    }

    fn progress_set(&self, index: usize, num: u64) {
        if let Some(pb) = self.pb_map.read().unwrap().get(&(index + 1)) {
            pb.set_position(num);
        }
    }

    fn failed_to_get_source_next_url(&self, _index: usize, err: &str) {
        self.writeln(
            &style("ERROR").red().bold().to_string(),
            &fl!("can-not-get-source-next-url", e = err.to_string()),
        )
        .ok();
    }

    fn download_done(&self, _index: usize, msg: &str) {
        tracing::debug!("Downloaded {msg}");
    }

    fn all_done(&self) {
        if let Some(gpb) = &self.pb_map.read().unwrap().get(&0) {
            gpb.finish_and_clear();
        }
    }

    fn new_global_progress_bar(&self, total_size: u64) {
        let sty = global_progress_bar_style(&WRITER);
        let pb = self
            .mb
            .insert(0, ProgressBar::new(total_size).with_style(sty));
        self.pb_map.write().unwrap().insert(0, pb);
    }
}

impl HandleTopicsControl for OmaMultiProgressBar {
    fn scanning_topic(&self) {
        let (sty, inv) = spinner_style();
        let pb = self
            .mb
            .insert(1, ProgressBar::new_spinner().with_style(sty));
        pb.set_message(fl!("refreshing-topic-metadata"));
        pb.enable_steady_tick(inv);
        self.pb_map.write().unwrap().insert(1, pb);
    }

    fn closing_topic(&self, topic: &str) {
        self.writeln(
            &style("INFO").blue().bold().to_string(),
            &fl!("scan-topic-is-removed", name = topic),
        )
        .ok();
    }

    fn topic_not_in_mirror(&self, topic: &str, mirror: &str) {
        self.writeln(
            &style("WARNING").yellow().bold().to_string(),
            &fl!("topic-not-in-mirror", topic = topic, mirror = mirror),
        )
        .ok();
        self.writeln(
            &style("WARNING").yellow().bold().to_string(),
            &fl!("skip-write-mirror"),
        )
        .ok();
    }
}

impl HandleRefresh for OmaMultiProgressBar {
    fn run_invoke_script(&self) {
        let (sty, inv) = spinner_style();
        let pb = self
            .mb
            .insert(1, ProgressBar::new_spinner().with_style(sty));
        pb.set_message(fl!("oma-refresh-success-invoke"));
        pb.enable_steady_tick(inv);
        self.pb_map.write().unwrap().insert(1, pb);
    }
}

impl Default for NoProgressBar {
    fn default() -> Self {
        Self {
            timer: RwLock::new(Instant::now()),
            total_size: OnceCell::new(),
            old_downloaded: AtomicU64::new(0),
        }
    }
}

pub struct NoProgressBar {
    timer: RwLock<Instant>,
    total_size: OnceCell<u64>,
    old_downloaded: AtomicU64,
}

impl DownloadProgressControl for NoProgressBar {
    fn checksum_mismatch_retry(&self, _index: usize, filename: &str, times: usize) {
        error!(
            "{}",
            fl!("checksum-mismatch-retry", c = filename, retry = times)
        );
    }

    fn global_progress_set(&self, num: &AtomicU64) {
        let elapsed = self.timer.read().unwrap().elapsed();
        if elapsed >= Duration::from_secs(3) {
            let downloaded = num.load(Ordering::SeqCst);
            if let Some(total_size) = self.total_size.get() {
                msg!(
                    "{} / {} ({}/s)",
                    HumanBytes(downloaded),
                    HumanBytes(*total_size),
                    HumanBytes(
                        (downloaded - self.old_downloaded.load(Ordering::SeqCst))
                            / elapsed.as_secs()
                    )
                );
                self.old_downloaded.store(downloaded, Ordering::SeqCst);
            } else {
                msg!("Downloaded {}", HumanBytes(downloaded));
            }
            *self.timer.write().unwrap() = Instant::now();
        }
    }

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

    fn new_global_progress_bar(&self, total_size: u64) {
        self.total_size.get_or_init(|| total_size);
    }
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
