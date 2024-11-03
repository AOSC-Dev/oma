use std::sync::atomic::Ordering;

use dashmap::DashMap;
use indicatif::{MultiProgress, ProgressBar};
use oma_console::{
    pb::{global_progress_bar_style, progress_bar_style, spinner_style},
    writer::Writer,
};
use oma_fetch::{reqwest::ClientBuilder, DownloadProgressControl};
use oma_pm::apt::{AptConfig, OmaApt, OmaAptArgs, OmaAptError, SummarySort};

fn main() -> Result<(), OmaAptError> {
    let oma_apt_args = OmaAptArgs::builder().yes(true).build();
    let mut apt = OmaApt::new(vec![], oma_apt_args, false, AptConfig::new())?;
    let pkgs = apt.select_pkg(&vec!["fish"], false, true, true)?;

    apt.install(&pkgs.0, false)?;

    let client = ClientBuilder::new().user_agent("oma").build().unwrap();

    apt.resolve(false, true, false)?;

    let op = apt.summary(SummarySort::Operation, |_| false, |_| false)?;

    let pm = MyProgressManager::default();

    apt.commit(&client, None, &pm, op)?;

    Ok(())
}

struct MyProgressManager {
    mb: MultiProgress,
    pb_map: DashMap<usize, ProgressBar>,
}

impl Default for MyProgressManager {
    fn default() -> Self {
        Self {
            mb: MultiProgress::new(),
            pb_map: DashMap::new(),
        }
    }
}

impl DownloadProgressControl for MyProgressManager {
    fn checksum_mismatch_retry(&self, _index: usize, filename: &str, times: usize) {
        self.mb
            .println(format!(
                "{filename} checksum failed, retrying {times} times"
            ))
            .unwrap();
    }

    fn global_progress_set(&self, num: &std::sync::atomic::AtomicU64) {
        if let Some(pb) = self.pb_map.get(&0) {
            pb.set_position(num.load(Ordering::SeqCst));
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
        let writer = Writer::default();
        let sty = progress_bar_style(&writer);
        let pb = self
            .mb
            .insert(index + 1, ProgressBar::new(size).with_style(sty));
        pb.set_message(msg.to_string());
        self.pb_map.insert(index + 1, pb);
    }

    fn progress_inc(&self, index: usize, num: u64) {
        let pb = self.pb_map.get(&(index + 1)).unwrap();
        pb.inc(num);
    }

    fn progress_set(&self, index: usize, num: u64) {
        let pb = self.pb_map.get(&(index + 1)).unwrap();
        pb.set_position(num);
    }

    fn failed_to_get_source_next_url(&self, _index: usize, err: &str) {
        self.mb.println(format!("Error: {err}")).unwrap();
    }

    fn download_done(&self, _index: usize, _msg: &str) {
        return;
    }

    fn all_done(&self) {
        return;
    }

    fn new_global_progress_bar(&self, total_size: u64) {
        let writer = Writer::default();
        let sty = global_progress_bar_style(&writer);
        let pb = self
            .mb
            .insert(0, ProgressBar::new(total_size).with_style(sty));
        self.pb_map.insert(0, pb);
    }
}
