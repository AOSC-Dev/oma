use std::{path::PathBuf, sync::atomic::Ordering};

use dashmap::DashMap;
use indicatif::{MultiProgress, ProgressBar};
use oma_console::{
    pb::{progress_bar_style, spinner_style},
    writer::Writer,
};
use oma_fetch::{
    checksum::Checksum, DownloadEntry, DownloadError, DownloadManager, DownloadProgressControl,
    DownloadResult, DownloadSource, DownloadSourceType, Summary,
};
use reqwest::ClientBuilder;
use tokio::io::AsyncWriteExt;

#[tokio::main]
async fn main() -> DownloadResult<()> {
    let source_1 = DownloadSource {
        url: "https://mirrors.bfsu.edu.cn/anthon/mascots/zhaxia-stickers-v1.zip".to_string(),
        source_type: DownloadSourceType::Http { auth: None },
    };

    let file_1 = DownloadEntry::builder()
        .source(vec![source_1])
        .filename("zhaxia-stickers-v1.zip".to_string().into())
        .dir(PathBuf::from("./oma-fetcher-test"))
        .hash(
            Checksum::from_sha256_str(
                "de700bdb45a4b8ab322d7eb2d30a7b448f117693b5a4164a3f05345177884134",
            )
            .unwrap(),
        )
        .allow_resume(true)
        .build();

    let source_2 = DownloadSource {
        url: "https://mirrors.bfsu.edu.cn/anthon/mascots/mascots.zip".to_string(),
        source_type: DownloadSourceType::Http { auth: None },
    };

    let file_2 = DownloadEntry::builder()
        .source(vec![source_2])
        .filename("mascots.zip".to_string().into())
        .dir(PathBuf::from("./oma-fetcher-test"))
        .allow_resume(false)
        .build();

    let mut test_local_file = tokio::fs::File::create("test").await.unwrap();
    test_local_file.write_all(b"test").await.unwrap();

    let client = ClientBuilder::new().user_agent("oma").build().unwrap();

    let pm = MyProgressManager::default();

    let download_manager = DownloadManager::builder()
        .client(&client)
        .download_list(vec![file_1, file_2])
        .progress_manager(&pm)
        .build();

    tokio::fs::create_dir_all("./oma-fetcher-test")
        .await
        .unwrap();

    download_manager
        .start_download()
        .await
        .into_iter()
        .collect::<Result<Vec<Summary>, DownloadError>>()?;

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
        let pb = self.mb.insert(0, ProgressBar::new(total_size));
        self.pb_map.insert(0, pb);
    }
}
