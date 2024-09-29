use std::{
    cmp::Ordering,
    fmt::Display,
    path::PathBuf,
    sync::{atomic::AtomicU64, Arc},
};

use bon::{builder, Builder};
use checksum::Checksum;
use download::SingleDownloader;
use futures::StreamExt;

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

#[derive(Debug, Clone, Default, PartialEq, Eq)]
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
    Local { as_symlink: bool },
}

impl PartialOrd for DownloadSourceType {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for DownloadSourceType {
    fn cmp(&self, other: &Self) -> Ordering {
        match self {
            DownloadSourceType::Http => match other {
                DownloadSourceType::Http => Ordering::Equal,
                DownloadSourceType::Local { .. } => Ordering::Less,
            },
            DownloadSourceType::Local { .. } => match other {
                DownloadSourceType::Http => Ordering::Greater,
                DownloadSourceType::Local { .. } => Ordering::Equal,
            },
        }
    }
}

pub struct OmaFetcher<'a> {
    client: &'a Client,
    download_list: Vec<DownloadEntry>,
    limit_thread: usize,
    retry_times: usize,
    global_progress: Arc<AtomicU64>,
}

#[derive(Debug)]
pub struct Summary {
    pub filename: String,
    pub wrote: bool,
    pub count: usize,
    pub context: Option<String>,
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
    ProgressSet(u64),
    CanNotGetSourceNextUrl(String),
    Done(String),
    AllDone,
}

impl Display for DownloadEvent {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_fmt(format_args!("{self:?}"))
    }
}

/// OmaFetcher is a Download Manager
impl<'a> OmaFetcher<'a> {
    pub fn new(
        client: &'a Client,
        download_list: Vec<DownloadEntry>,
        limit_thread: Option<usize>,
    ) -> DownloadResult<OmaFetcher<'a>> {
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
            let msg = c.msg.clone();
            // 因为数据的来源是确定的，所以这里能够确定肯定不崩溃，因此直接 unwrap
            let single = SingleDownloader::builder()
                .client(self.client)
                .maybe_context(msg.clone())
                .download_list_index(i)
                .entry(c)
                .progress((i + 1, self.download_list.len(), msg))
                .retry_times(self.retry_times)
                .file_type(c.file_type.clone())
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
            tasks.push(single.try_download(self.global_progress.clone(), callback.clone()));
        }

        let thread = if file_download_source >= http_download_source {
            1
        } else {
            self.limit_thread
        };

        let stream = futures::stream::iter(tasks).buffer_unordered(thread);
        let res = stream.collect::<Vec<_>>().await;
        callback(0, DownloadEvent::AllDone);

        res
    }
}
