use std::{path::PathBuf, sync::Arc};

use futures::StreamExt;
use oma_console::{
    indicatif::{self, MultiProgress, ProgressBar},
    writer::Writer,
};

use reqwest::{Client, ClientBuilder};

pub mod checksum;
mod download;

use download::try_download;

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
    #[error(transparent)]
    TemplateError(#[from] indicatif::style::TemplateError),
    #[error("Failed to open local source file {0}: {1}")]
    FailedOpenLocalSourceFile(String, String),
    #[error("Download all file failed: {0}: {1}")]
    DownloadAllFailed(String, String),
}

pub type DownloadResult<T> = std::result::Result<T, DownloadError>;

#[derive(Debug, Clone)]
pub struct DownloadEntry {
    source: Vec<DownloadSource>,
    filename: String,
    dir: PathBuf,
    hash: Option<String>,
    allow_resume: bool,
    msg: Option<String>,
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

impl DownloadEntry {
    pub fn new(
        source: Vec<DownloadSource>,
        filename: String,
        dir: PathBuf,
        hash: Option<String>,
        allow_resume: bool,
        msg: Option<String>,
    ) -> Self {
        Self {
            source,
            filename,
            dir,
            hash,
            allow_resume,
            msg,
        }
    }
}

#[derive(Clone)]
pub struct FetchProgressBar {
    pub mb: Arc<MultiProgress>,
    pub global_bar: Option<ProgressBar>,
    pub progress: Option<(usize, usize)>,
    pub msg: Option<String>,
}

impl FetchProgressBar {
    pub fn new(
        mb: Arc<MultiProgress>,
        global_bar: Option<ProgressBar>,
        progress: Option<(usize, usize)>,
        msg: Option<String>,
    ) -> Self {
        Self {
            mb,
            global_bar,
            progress,
            msg,
        }
    }
}

pub struct OmaFetcher {
    client: Client,
    bar: Option<(Arc<MultiProgress>, Option<ProgressBar>)>,
    download_list: Vec<DownloadEntry>,
    limit_thread: usize,
    retry_times: usize,
}

#[derive(Debug)]
pub struct Summary {
    pub filename: String,
    pub writed: bool,
    pub count: usize,
    pub context: Option<String>,
}

impl Summary {
    fn new(filename: &str, writed: bool, count: usize, context: Option<String>) -> Self {
        Self {
            filename: filename.to_string(),
            writed,
            count,
            context,
        }
    }
}

impl OmaFetcher {
    pub fn new(
        client: Option<Client>,
        bar: bool,
        total_size: Option<u64>,
        download_list: Vec<DownloadEntry>,
        limit_thread: Option<usize>,
    ) -> DownloadResult<Self> {
        let client = client.unwrap_or(ClientBuilder::new().user_agent("oma").build()?);

        let bar = if bar {
            let mb = Arc::new(MultiProgress::new());
            let writer = Writer::default();
            let gpb = if let Some(total_size) = total_size {
                Some(
                    mb.insert(
                        0,
                        ProgressBar::new(total_size)
                            .with_style(oma_console::pb::oma_style_pb(writer, true)?),
                    ),
                )
            } else {
                None
            };

            if let Some(ref gpb) = gpb {
                gpb.set_message("Progress")
            }

            Some((mb, gpb))
        } else {
            None
        };

        Ok(Self {
            client,
            bar,
            download_list,
            limit_thread: limit_thread.unwrap_or(4),
            retry_times: 3,
        })
    }

    pub fn retry_times(&mut self, retry_times: usize) -> &mut Self {
        self.retry_times = retry_times;
        self
    }

    pub async fn start_download(&self) -> Vec<DownloadResult<Summary>> {
        let mut tasks = Vec::new();
        for (i, c) in self.download_list.iter().enumerate() {
            let fpb = if let Some((mb, gpb)) = &self.bar {
                Some(FetchProgressBar {
                    mb: mb.clone(),
                    global_bar: gpb.clone(),
                    progress: Some((i + 1, self.download_list.len())),
                    msg: c.msg.clone(),
                })
            } else {
                None
            };

            tasks.push(try_download(
                &self.client,
                c,
                fpb,
                i,
                self.retry_times,
                c.msg.clone(),
            ));
        }

        let stream = futures::stream::iter(tasks).buffer_unordered(self.limit_thread);

        let res = stream.collect::<Vec<_>>().await;

        if let Some(gpb) = self.bar.as_ref().and_then(|x| x.1.as_ref()) {
            gpb.finish_and_clear();
        }

        res
    }
}
