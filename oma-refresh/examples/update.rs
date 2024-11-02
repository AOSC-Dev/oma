use std::{path::Path, result::Result, sync::atomic::Ordering};

use apt_auth_config::AuthConfig;
use dashmap::DashMap;
use indicatif::{MultiProgress, ProgressBar};
use oma_apt::config::Config;
use oma_console::{
    pb::{global_progress_bar_style, progress_bar_style, spinner_style},
    writer::Writer,
};
use oma_fetch::{reqwest::ClientBuilder, DownloadProgressControl};
use oma_refresh::db::{HandleRefresh, HandleTopicsControl, OmaRefresh, RefreshError};
use oma_utils::dpkg::dpkg_arch;

#[tokio::main]
async fn main() -> Result<(), RefreshError> {
    let p = Path::new("./oma-fetcher-test");
    tokio::fs::create_dir_all(p).await.unwrap();
    let client = ClientBuilder::new().user_agent("oma").build().unwrap();

    let apt_config = Config::new();

    let pm: Box<dyn HandleRefresh> = Box::new(MyProgressManager::default());

    OmaRefresh::builder()
        .client(&client)
        .apt_config(&apt_config)
        .arch(dpkg_arch("/")?)
        .download_dir(p.to_path_buf())
        .source("/".into())
        .topic_msg("test")
        .refresh_topics(false)
        .progress_manager(pm.as_ref())
        .auth_config(&AuthConfig::from_path("/").unwrap())
        .build()
        .start()
        .await?;

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

impl HandleTopicsControl for MyProgressManager {
    fn scanning_topic(&self) {
        self.mb.println("Scanning topics ...").unwrap();
    }

    fn closing_topic(&self, topic: &str) {
        self.mb.println(format!("Closing topic {}", topic)).unwrap();
    }

    fn topic_not_in_mirror(&self, topic: &str, mirror: &str) {
        self.mb
            .println(format!("Topic {} not in mirror {}", topic, mirror))
            .unwrap();
    }
}

impl HandleRefresh for MyProgressManager {
    fn run_invoke_script(&self) {
        self.mb
            .println("Executing Post-refresh configuration script (Post-Invoke-Success) ...")
            .unwrap();
    }
}
