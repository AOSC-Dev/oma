use std::{borrow::Cow, cmp::Ordering, fmt::Debug, path::PathBuf, sync::Arc, time::Duration};

use ahash::AHashMap;
use bon::Builder;
use checksum::Checksum;
use download::{BuilderError, SingleDownloader, SuccessSummary};

use reqwest::{Method, Response, Url};
use reqwest_middleware::{ClientWithMiddleware, RequestBuilder};
use serde::{Deserialize, Serialize};
use spdlog::debug;
use tokio::task::JoinSet;

pub mod checksum;
pub mod download;
pub use crate::download::SingleDownloadError;

pub use reqwest;

#[derive(Clone, Default, Builder)]
pub struct DownloadEntry {
    pub source: Vec<DownloadSource>,
    pub filename: String,
    dir: PathBuf,
    final_dir: Option<PathBuf>,
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
    Cleared {
        index: usize,
        /// Bytes to remove from the global progress bar (0 on success).
        sub: u64,
    },
    Indeterminate {
        index: usize,
        total: usize,
        msg: String,
    },
    Determinate {
        index: usize,
        total: usize,
        msg: String,
        size: u64,
    },
    Advance {
        index: usize,
        size: u64,
    },
    NextUrl {
        file_name: String,
        err: SingleDownloadError,
    },
    FileDone {
        msg: Box<str>,
    },
    Failed {
        file_name: String,
        error: SingleDownloadError,
    },
    AllDone,
    GlobalDeterminate(u64),
}

#[derive(Serialize, Deserialize)]
pub(crate) enum SingleDownloadErrorHelper {
    Open { source: String },
    Create { source: String },
    Seek { source: String },
    Write { source: String },
    Flush { source: String },
    Remove { source: String },
    CreateSymlink { source: String },
    ReqwestMiddlewareError { source: String },
    Read { source: String },
    SendRequestTimeout,
    DownloadTimeout,
    ChecksumMismatch,
    AcquireError,
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
    pub async fn start_download<F, Fut>(mut self, callback: F) -> Result<Summary, BuilderError>
    where
        F: Fn(Event) -> Fut + Send + Sync + 'static,
        Fut: std::future::Future<Output = ()> + Send + 'static,
    {
        let mut list = vec![];
        let len = self.download_list.len();

        let mut source_locks = AHashMap::new();

        for (i, c) in std::mem::take(&mut self.download_list)
            .into_iter()
            .enumerate()
        {
            let source_key = if let Some(src) = c.source.first() {
                if let Ok(url) = Url::parse(&src.url) {
                    format!("{}://{}", url.scheme(), url.host_str().unwrap_or("unknown"))
                } else {
                    "fallback_source".to_string()
                }
            } else {
                "unknown_source".to_string()
            };

            let source_sem = source_locks
                .entry(source_key)
                .or_insert_with(|| Arc::new(tokio::sync::Semaphore::new(self.threads)))
                .clone();

            let single = SingleDownloader::builder()
                .client(self.client.clone())
                .download_list_index(i)
                .entry(c)
                .total(len)
                .retry_times(self.retry_times)
                .timeout(self.timeout)
                .build()?;

            list.push((single, source_sem));
        }

        // Downloaders produce events synchronously onto this channel; a
        // forwarding task delivers them in order to the async callback. This
        // lets the internal progress code run without awaiting (and enables
        // automatic cleanup via `Drop`).
        let (tx, rx) = flume::unbounded::<Event>();
        let forwarder = tokio::spawn(async move {
            while let Ok(event) = rx.recv_async().await {
                callback(event).await;
            }
        });

        if self.total_size != 0 {
            let _ = tx.send(Event::GlobalDeterminate(self.total_size));
        }

        let mut set = JoinSet::new();

        for (single, source_sem) in list {
            let tx = tx.clone();

            set.spawn(async move {
                let _permit = match source_sem.acquire_owned().await {
                    Ok(p) => Some(p),
                    Err(_) => {
                        let _ = tx.send(Event::Failed {
                            file_name: single.entry.filename.to_string(),
                            error: SingleDownloadError::AcquireError,
                        });

                        return download::DownloadResult::Failed {
                            file_name: single.entry.filename,
                        };
                    }
                };

                single.try_download(&tx).await
            });
        }

        let mut success = vec![];
        let mut failed = vec![];

        while let Some(res) = set.join_next().await {
            match res {
                Ok(download::DownloadResult::Success(success_summary)) => {
                    success.push(success_summary);
                }
                Ok(download::DownloadResult::Failed { file_name }) => {
                    failed.push(file_name);
                }
                Err(_) => {
                    failed.push("task_panicked".to_string());
                }
            }
        }

        let _ = tx.send(Event::AllDone);
        drop(tx);
        let _ = forwarder.await;

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

/// Test-only helpers shared across the crate's `#[cfg(test)]` modules.
#[cfg(test)]
pub(crate) mod test_support {
    use std::path::{Path, PathBuf};

    /// A unique temporary directory that is removed when dropped.
    pub(crate) struct TempDir(PathBuf);

    impl TempDir {
        pub(crate) fn new(label: &str) -> Self {
            let path = std::env::temp_dir().join(format!(
                "oma-fetch-{label}-{}-{}",
                std::process::id(),
                std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .expect("system clock before epoch")
                    .as_nanos()
            ));
            std::fs::create_dir_all(&path).expect("create temp dir");
            Self(path)
        }

        pub(crate) fn path(&self) -> &Path {
            &self.0
        }
    }

    impl Drop for TempDir {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn compress_type_from_str() {
        assert_eq!(CompressType::from("xz"), CompressType::Xz);
        assert_eq!(CompressType::from("gz"), CompressType::Gzip);
        assert_eq!(CompressType::from("bz2"), CompressType::Bz2);
        assert_eq!(CompressType::from("zst"), CompressType::Zstd);
        assert_eq!(CompressType::from("unknown"), CompressType::None);
    }

    #[test]
    fn local_sources_rank_above_http() {
        let http = DownloadSource {
            url: "http://example.com/a".into(),
            source_type: DownloadSourceType::Http,
        };
        let local = DownloadSource {
            url: "file:///a".into(),
            source_type: DownloadSourceType::Local(false),
        };
        assert!(local.source_type > http.source_type);

        let mut sources = [http.clone(), local.clone()];
        sources.sort_unstable_by(|a, b| b.source_type.cmp(&a.source_type));
        assert_eq!(sources[0].source_type, local.source_type);
        assert_eq!(sources[1].source_type, http.source_type);
    }
}
