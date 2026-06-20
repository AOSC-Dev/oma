use std::{borrow::Cow, cmp::Ordering, fmt::Debug, path::PathBuf, time::Duration};

use bon::Builder;
use checksum::Checksum;
use download::{BuilderError, SingleDownloader, SuccessSummary};
use futures::StreamExt;

use reqwest::{Method, Response};
use reqwest_middleware::{ClientWithMiddleware, RequestBuilder};
use serde::{Deserialize, Serialize};
use spdlog::debug;

pub mod checksum;
pub mod download;
pub use crate::download::SingleDownloadError;

pub use reqwest;

#[derive(Clone, Default, Builder)]
pub struct DownloadEntry {
    pub source: Vec<DownloadSource>,
    pub filename: String,
    dir: PathBuf,
    hash: Option<Checksum>,
    allow_resume: bool,
    msg: Option<Cow<'static, str>>,
    #[builder(default)]
    file_type: CompressType,
}

impl Debug for DownloadEntry {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("DownloadEntry")
            .field("source", &self.source)
            .field("filename", &self.filename)
            .field("dir", &self.dir)
            .field("hash", &self.hash.as_ref().map(|c| c.to_string()))
            .field("allow_resume", &self.allow_resume)
            .field("msg", &self.msg)
            .field("file_type", &self.file_type)
            .finish()
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Copy)]
pub enum CompressType {
    Bz2,
    Gzip,
    Xz,
    Zstd,
    Lzma,
    Lz4,
    #[default]
    None,
}

impl From<&str> for CompressType {
    fn from(s: &str) -> Self {
        match s {
            "xz" => CompressType::Xz,
            "gz" => CompressType::Gzip,
            "bz2" => CompressType::Bz2,
            "zst" => CompressType::Zstd,
            _ => CompressType::None,
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
    Http,
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

#[derive(Debug, Serialize, Deserialize)]
pub enum Event {
    ChecksumMismatch {
        index: usize,
        filename: String,
        times: usize,
    },
    Timeout {
        filename: String,
        times: usize,
    },
    GlobalProgressAdd(u64),
    GlobalProgressSub(u64),
    ProgressDone(usize),
    NewProgressSpinner {
        index: usize,
        total: usize,
        msg: String,
    },
    NewProgressBar {
        index: usize,
        total: usize,
        msg: String,
        size: u64,
    },
    ProgressInc {
        index: usize,
        size: u64,
    },
    NextUrl {
        index: usize,
        file_name: String,
        err: SingleDownloadError,
    },
    DownloadDone {
        index: usize,
        msg: Box<str>,
    },
    Failed {
        file_name: String,
        error: SingleDownloadError,
    },
    AllDone,
    NewGlobalProgressBar(u64),
}

#[derive(Serialize, Deserialize)]
pub(crate) enum SingleDownloadErrorHelper {
    SetPermission { source: String },
    OpenAsWriteMode { source: String },
    Open { source: String },
    Create { source: String },
    Seek { source: String },
    Write { source: String },
    Flush { source: String },
    Remove { source: String },
    CreateSymlink { source: String },
    ReqwestMiddlewareError { source: String },
    BrokenPipe { source: String },
    SendRequestTimeout,
    DownloadTimeout,
    ChecksumMismatch,
}

#[derive(Builder)]
pub struct DownloadManager {
    client: ClientWithMiddleware,
    download_list: Box<[DownloadEntry]>,
    #[builder(default = 4)]
    threads: usize,
    #[builder(default = 3)]
    retry_times: usize,
    #[builder(default)]
    total_size: u64,
    #[builder(default = Duration::from_secs(15))]
    timeout: Duration,
}

#[derive(Debug)]
pub struct Summary {
    pub success: Vec<SuccessSummary>,
    pub failed: Vec<String>,
}

impl Summary {
    pub fn is_download_success(&self) -> bool {
        self.failed.is_empty()
    }

    pub fn has_wrote(&self) -> bool {
        self.success.iter().any(|x| x.wrote)
    }
}

impl DownloadManager {
    /// Start download
    pub async fn start_download(
        mut self,
        callback: impl AsyncFn(Event),
    ) -> Result<Summary, BuilderError> {
        if self.threads == 0 || self.threads > 255 {
            return Err(BuilderError::IllegalDownloadThread {
                count: self.threads,
            });
        }

        let mut tasks = Vec::new();
        let mut list = vec![];
        let len = self.download_list.len();
        for (i, c) in std::mem::take(&mut self.download_list)
            .into_iter()
            .enumerate()
        {
            let single = SingleDownloader::builder()
                .client(self.client.clone())
                .download_list_index(i)
                .entry(c)
                .total(len)
                .retry_times(self.retry_times)
                .timeout(self.timeout)
                .build()?;

            list.push(single);
        }

        for single in list {
            tasks.push(single.try_download(&callback));
        }

        if self.total_size != 0 {
            callback(Event::NewGlobalProgressBar(self.total_size)).await;
        }

        let stream = futures::stream::iter(tasks).buffer_unordered(self.threads);
        let res = stream.collect::<Vec<_>>().await;
        callback(Event::AllDone).await;

        let (mut success, mut failed) = (vec![], vec![]);

        for i in res {
            match i {
                download::DownloadResult::Success(success_summary) => {
                    success.push(success_summary);
                }
                download::DownloadResult::Failed { file_name } => {
                    failed.push(file_name);
                }
            }
        }

        Ok(Summary { success, failed })
    }
}

pub async fn send_request_with_url_and_method(
    url: &str,
    client: &ClientWithMiddleware,
    method: Method,
) -> Result<Response, reqwest_middleware::Error> {
    let resp = client.request(method, url).send().await?;
    let headers = resp.headers();

    debug!(
        "\nDownload URL: {url}\nStatus: {}\nHeaders: {headers:#?}",
        resp.status()
    );

    let resp = resp.error_for_status()?;

    Ok(resp)
}

pub async fn send_request(request: RequestBuilder) -> Result<Response, reqwest_middleware::Error> {
    let resp = request.send().await?;
    let headers = resp.headers();
    let url = resp.url();

    debug!(
        "\nDownload URL: {url}\nStatus: {}\nHeaders: {headers:#?}",
        resp.status()
    );

    let resp = resp.error_for_status()?;

    Ok(resp)
}
