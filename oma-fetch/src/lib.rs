use std::{
    path::PathBuf,
    sync::{atomic::AtomicU64, Arc},
};

use derive_builder::Builder;
use download::SingleDownloaderBuilder;
use futures::StreamExt;

use reqwest::{Client, ClientBuilder};

pub mod checksum;
mod download;

#[derive(thiserror::Error, Debug)]
pub enum DownloadError {
    #[error("checksum mismatch {0} at dir {1}")]
    ChecksumMisMatch(String, String),
    #[error("404 not found: {0}")]
    NotFound(String),
    #[error(transparent)]
    IOError(#[from] tokio::io::Error),
    #[error(transparent)]
    ReqwestError(#[from] reqwest::Error),
    #[error(transparent)]
    ChecksumError(#[from] crate::checksum::ChecksumError),
    #[error("Failed to open local source file {0}: {1}")]
    FailedOpenLocalSourceFile(Arc<String>, String),
    #[error("Download all file failed: {0}: {1}")]
    DownloadAllFailed(String, String),
    #[error(transparent)]
    DownloadSourceBuilderError(#[from] DownloadEntryBuilderError),
    #[error("Invaild URL: {0}")]
    InvaildURL(String),
}

pub type DownloadResult<T> = std::result::Result<T, DownloadError>;

#[derive(Debug, Clone, Builder, Default)]
#[builder(default)]
pub struct DownloadEntry {
    source: Vec<DownloadSource>,
    filename: Arc<String>,
    dir: PathBuf,
    #[builder(setter(into, strip_option))]
    hash: Option<String>,
    allow_resume: bool,
    #[builder(setter(into, strip_option))]
    msg: Option<String>,
    extract: bool,
}

#[derive(Debug, Clone)]
pub struct DownloadSource {
    url: String,
    source_type: DownloadSourceType,
}

impl DownloadSource {
    pub fn new(url: String, source_type: DownloadSourceType) -> Self {
        Self { url, source_type }
    }
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub enum DownloadSourceType {
    Http,
    Local,
}

impl PartialOrd for DownloadSourceType {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for DownloadSourceType {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        match self {
            DownloadSourceType::Http => match other {
                DownloadSourceType::Http => std::cmp::Ordering::Equal,
                DownloadSourceType::Local => std::cmp::Ordering::Less,
            },
            DownloadSourceType::Local => match other {
                DownloadSourceType::Http => std::cmp::Ordering::Greater,
                DownloadSourceType::Local => std::cmp::Ordering::Equal,
            },
        }
    }
}

pub struct OmaFetcher {
    client: Client,
    download_list: Vec<DownloadEntry>,
    limit_thread: usize,
    retry_times: usize,
    global_progress: Arc<AtomicU64>,
}

#[derive(Debug)]
pub struct Summary {
    pub filename: Arc<String>,
    pub writed: bool,
    pub count: usize,
    pub context: Arc<Option<String>>,
}

#[derive(Debug)]
pub enum DownloadEvent {
    ChecksumMismatchRetry { filename: String, times: usize },
    GlobalProgressSet(u64),
    GlobalProgressInc(u64),
    ProgressDone,
    NewProgressSpinner(String),
    NewProgress(u64, String),
    ProgressInc(u64),
    CanNotGetSourceNextUrl(String),
    Done(String),
    AllDone,
}

/// Summary struct to save download result
impl Summary {
    fn new(
        filename: Arc<String>,
        writed: bool,
        count: usize,
        context: Arc<Option<String>>,
    ) -> Self {
        Self {
            filename,
            writed,
            count,
            context,
        }
    }
}

/// OmaFetcher is a Download Manager
impl OmaFetcher {
    pub fn new(
        client: Option<Client>,
        download_list: Vec<DownloadEntry>,
        limit_thread: Option<usize>,
    ) -> DownloadResult<Self> {
        let client = client.unwrap_or(ClientBuilder::new().user_agent("oma").build()?);

        Ok(Self {
            client,
            download_list,
            limit_thread: limit_thread.unwrap_or(4),
            retry_times: 3,
            global_progress: Arc::new(AtomicU64::new(0)),
        })
    }

    /// Set retry times
    pub fn retry_times(&mut self, retry_times: usize) -> &mut Self {
        self.retry_times = retry_times;
        self
    }

    /// Start download
    pub async fn start_download<F>(&self, callback: F) -> Vec<DownloadResult<Summary>>
    where
        F: Fn(usize, DownloadEvent) + Clone + Send + Sync,
    {
        let callback = Arc::new(callback);
        let mut tasks = Vec::new();
        let mut list = vec![];
        for (i, c) in self.download_list.iter().enumerate() {
            let msg = Arc::new(c.msg.clone());
            // 因为数据的来源是确定的，所以这里能够确定肯定不崩溃，因此直接 unwrap
            let single = SingleDownloaderBuilder::default()
                .client(&self.client)
                .context(msg.clone())
                .download_list_index(i)
                .entry(c)
                .progress((i + 1, self.download_list.len(), msg))
                .retry_times(self.retry_times)
                .build()
                .unwrap();

            list.push(single);
        }

        for single in list {
            tasks.push(single.try_download(self.global_progress.clone(), callback.clone()));
        }

        let stream = futures::stream::iter(tasks).buffer_unordered(self.limit_thread);
        let res = stream.collect::<Vec<_>>().await;
        callback(0, DownloadEvent::AllDone);

        res
    }
}
