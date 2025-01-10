use std::{cmp::Ordering, path::PathBuf, sync::atomic::AtomicU64, time::Duration};

use bon::{builder, Builder};
use checksum::Checksum;
use download::SingleDownloader;
use futures::{Future, StreamExt};

use reqwest::Client;

pub mod checksum;
mod download;

pub use reqwest;

#[derive(thiserror::Error, Debug)]
pub enum DownloadError {
    #[error("checksum mismatch {0}")]
    ChecksumMismatch(String),
    #[error("Failed to download file: {0}, kind: {1}")]
    IOError(String, std::io::Error),
    #[error(transparent)]
    ReqwestError(reqwest::Error),
    #[error(transparent)]
    ChecksumError(#[from] crate::checksum::ChecksumError),
    #[error("Failed to open local source file {0}: {1}")]
    FailedOpenLocalSourceFile(String, tokio::io::Error),
    #[error("Invalid URL: {0}")]
    InvalidURL(String),
    #[error("download source list is empty")]
    EmptySources,
}

pub type DownloadResult<T> = std::result::Result<T, DownloadError>;

#[derive(Debug, Clone, Default, Builder)]
pub struct DownloadEntry {
    pub source: Vec<DownloadSource>,
    pub filename: String,
    dir: PathBuf,
    hash: Option<Checksum>,
    allow_resume: bool,
    msg: Option<String>,
    #[builder(default)]
    file_type: CompressFile,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Copy)]
pub enum CompressFile {
    Bz2,
    Gzip,
    Xz,
    Zstd,
    #[default]
    Nothing,
}

// 压缩文件下载顺序：Zstd -> XZ -> Gzip -> Bz2 -> 未压缩
impl Ord for CompressFile {
    fn cmp(&self, other: &Self) -> Ordering {
        match self {
            CompressFile::Bz2 => match other {
                CompressFile::Bz2 => Ordering::Equal,
                CompressFile::Gzip => Ordering::Less,
                CompressFile::Xz => Ordering::Less,
                CompressFile::Zstd => Ordering::Less,
                CompressFile::Nothing => Ordering::Greater,
            },
            CompressFile::Gzip => match other {
                CompressFile::Bz2 => Ordering::Greater,
                CompressFile::Gzip => Ordering::Less,
                CompressFile::Xz => Ordering::Less,
                CompressFile::Zstd => Ordering::Less,
                CompressFile::Nothing => Ordering::Greater,
            },
            CompressFile::Xz => match other {
                CompressFile::Bz2 => Ordering::Greater,
                CompressFile::Gzip => Ordering::Greater,
                CompressFile::Xz => Ordering::Equal,
                CompressFile::Zstd => Ordering::Less,
                CompressFile::Nothing => Ordering::Greater,
            },
            CompressFile::Zstd => match other {
                CompressFile::Bz2 => Ordering::Greater,
                CompressFile::Gzip => Ordering::Greater,
                CompressFile::Xz => Ordering::Greater,
                CompressFile::Zstd => Ordering::Equal,
                CompressFile::Nothing => Ordering::Greater,
            },
            CompressFile::Nothing => match other {
                CompressFile::Bz2 => Ordering::Less,
                CompressFile::Gzip => Ordering::Less,
                CompressFile::Xz => Ordering::Less,
                CompressFile::Zstd => Ordering::Less,
                CompressFile::Nothing => Ordering::Equal,
            },
        }
    }
}

impl PartialOrd for CompressFile {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl From<&str> for CompressFile {
    fn from(s: &str) -> Self {
        match s {
            "xz" => CompressFile::Xz,
            "gz" => CompressFile::Gzip,
            "bz2" => CompressFile::Bz2,
            "zst" => CompressFile::Zstd,
            _ => CompressFile::Nothing,
        }
    }
}

#[derive(Debug, Clone)]
pub struct DownloadSource {
    pub url: String,
    pub source_type: DownloadSourceType,
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub enum DownloadSourceType {
    Http { auth: Option<(Box<str>, Box<str>)> },
    Local(bool),
}

impl PartialOrd for DownloadSourceType {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for DownloadSourceType {
    fn cmp(&self, other: &Self) -> Ordering {
        match self {
            DownloadSourceType::Http { .. } => match other {
                DownloadSourceType::Http { .. } => Ordering::Equal,
                DownloadSourceType::Local { .. } => Ordering::Less,
            },
            DownloadSourceType::Local { .. } => match other {
                DownloadSourceType::Http { .. } => Ordering::Greater,
                DownloadSourceType::Local { .. } => Ordering::Equal,
            },
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Event {
    ChecksumMismatch {
        index: usize,
        filename: String,
        times: usize,
    },
    GlobalProgressSet(u64),
    ProgressDone(usize),
    NewProgressSpinner {
        index: usize,
        msg: String,
    },
    NewProgressBar {
        index: usize,
        msg: String,
        size: u64,
    },
    ProgressInc {
        index: usize,
        size: u64,
    },
    NextUrl {
        index: usize,
        err: String,
    },
    DownloadDone {
        index: usize,
        msg: Box<str>,
    },
    AllDone,
    NewGlobalProgressBar(u64),
}

#[derive(Builder)]
pub struct DownloadManager<'a> {
    client: &'a Client,
    download_list: &'a [DownloadEntry],
    #[builder(default = 4)]
    threads: usize,
    #[builder(default = 3)]
    retry_times: usize,
    #[builder(skip = AtomicU64::new(0))]
    global_progress: AtomicU64,
    #[builder(default)]
    total_size: u64,
    set_permission: Option<u32>,
    #[builder(default = Duration::from_secs(15))]
    timeout: Duration,
}

#[derive(Debug)]
pub struct Summary {
    pub filename: String,
    pub wrote: bool,
    pub count: usize,
    pub context: Option<String>,
}

impl DownloadManager<'_> {
    /// Start download
    pub async fn start_download<F, Fut>(&self, callback: F) -> Vec<DownloadResult<Summary>>
    where
        F: Fn(Event) -> Fut,
        Fut: Future<Output = ()>,
    {
        let mut tasks = Vec::new();
        let mut list = vec![];
        for (i, c) in self.download_list.iter().enumerate() {
            let msg = c.msg.clone();
            let single = SingleDownloader::builder()
                .client(self.client)
                .maybe_msg(msg)
                .download_list_index(i)
                .entry(c)
                .progress((i + 1, self.download_list.len()))
                .retry_times(self.retry_times)
                .file_type(c.file_type)
                .maybe_set_permission(self.set_permission)
                .timeout(self.timeout)
                .build();

            list.push(single);
        }

        let file_download_source = list
            .iter()
            .filter(|x| {
                x.entry
                    .source
                    .iter()
                    .any(|x| matches!(x.source_type, DownloadSourceType::Local { .. }))
            })
            .count();

        let http_download_source = list.len() - file_download_source;

        for single in list {
            tasks.push(single.try_download(&self.global_progress, &callback));
        }

        let thread = if file_download_source >= http_download_source {
            1
        } else {
            self.threads
        };

        if self.total_size != 0 {
            callback(Event::NewGlobalProgressBar(self.total_size)).await;
        }

        let stream = futures::stream::iter(tasks).buffer_unordered(thread);
        let res = stream.collect::<Vec<_>>().await;
        callback(Event::AllDone).await;

        res
    }
}
