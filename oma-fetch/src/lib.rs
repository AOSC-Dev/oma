use std::{path::PathBuf, sync::Arc};

use futures::{future::BoxFuture, StreamExt};
use oma_console::{
    indicatif::{self, MultiProgress, ProgressBar},
    writer::Writer,
};

use reqwest::{Client, ClientBuilder};

pub mod checksum;
mod download;

use download::{download_local, http_download};

#[derive(thiserror::Error, Debug)]
pub enum DownloadError {
    #[error("checksum mismatch {0}")]
    ChecksumMisMatch(String),
    #[error("404 not found: {0}")]
    NotFound(String),
    #[error(transparent)]
    JoinError(#[from] tokio::task::JoinError),
    #[error(transparent)]
    IOError(#[from] tokio::io::Error),
    #[error(transparent)]
    ReqwestError(#[from] reqwest::Error),
    #[error(transparent)]
    ChecksumError(#[from] crate::checksum::ChecksumError),
    #[error("Invalid total: {0}")]
    InvaildTotal(String),
    #[error(transparent)]
    TemplateError(#[from] indicatif::style::TemplateError),
    #[error("Failed to open local source file {0}: {1}")]
    FailedOpenLocalSourceFile(String, String),
}

pub type DownloadResult<T> = std::result::Result<T, DownloadError>;

pub struct DownloadEntry {
    url: String,
    filename: String,
    dir: PathBuf,
    hash: Option<String>,
    allow_resume: bool,
    source_type: DownloadSourceType,
}

pub enum DownloadSourceType {
    Http,
    Local,
}

impl DownloadEntry {
    pub fn new(
        url: String,
        filename: String,
        dir: PathBuf,
        hash: Option<String>,
        allow_resume: bool,
        source_type: DownloadSourceType,
    ) -> Self {
        Self {
            url,
            filename,
            dir,
            hash,
            allow_resume,
            source_type,
        }
    }
}

#[derive(Clone)]
pub struct FetchProgressBar {
    mb: Arc<MultiProgress>,
    global_bar: Option<ProgressBar>,
    progress: Option<(usize, usize)>,
    msg: Option<String>,
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

            Some((mb, gpb))
        } else {
            None
        };

        Ok(Self {
            client,
            bar,
            download_list,
            limit_thread: limit_thread.unwrap_or(4),
        })
    }

    pub async fn start_download(&self) -> Vec<DownloadResult<bool>> {
        let mut tasks = Vec::new();
        for (i, c) in self.download_list.iter().enumerate() {
            let fpb = if let Some((mb, gpb)) = &self.bar {
                Some(FetchProgressBar {
                    mb: mb.clone(),
                    global_bar: gpb.clone(),
                    progress: Some((i + 1, self.download_list.len())),
                    msg: None,
                })
            } else {
                None
            };
            match c.source_type {
                DownloadSourceType::Http => {
                    let task: BoxFuture<'_, DownloadResult<bool>> =
                        Box::pin(http_download(&self.client, c, fpb));
                    tasks.push(task);
                }
                DownloadSourceType::Local => {
                    let task: BoxFuture<'_, DownloadResult<bool>> =
                        Box::pin(download_local(c, fpb));
                    tasks.push(task);
                }
            }
        }

        let stream = futures::stream::iter(tasks).buffer_unordered(self.limit_thread);

        stream
            .collect::<Vec<_>>()
            .await
    }
}
