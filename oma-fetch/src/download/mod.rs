//! Single-file download orchestration.
//!
//! [`SingleDownloader`] drives the download of one file from a list of
//! fallback sources. The implementation is split across submodules so no
//! single file gets too large:
//!
//! | Module     | Contents                                                     |
//! |------------|--------------------------------------------------------------|
//! | [`error`]  | error types and their (de)serialization                      |
//! | `progress` | progress bars + per-attempt download state                   |
//! | `http`     | HTTP download implementation                                 |
//! | `local`    | local (`file://`) source implementation                      |
//! | `verify`   | shared checksum helper                                       |

pub(crate) mod error;
pub(crate) mod http;
pub(crate) mod local;
pub(crate) mod progress;
pub(crate) mod verify;

use std::time::Duration;

use async_compression::futures::bufread::{
    BzDecoder, GzipDecoder, Lz4Decoder, LzmaDecoder, XzDecoder, ZstdDecoder,
};
use bon::bon;
use flume::Sender;
use futures::{AsyncRead, io};
use reqwest_middleware::ClientWithMiddleware;
use spdlog::trace;

use crate::{CompressType, DownloadEntry, DownloadSourceType, Event};

pub use self::error::{BuilderError, SingleDownloadError};

use self::progress::ProgressReporter;

const READ_FILE_BUFSIZE: usize = 65536;
const DOWNLOAD_BUFSIZE: usize = 8192;

/// A single file download task, with a list of fallback sources.
pub(crate) struct SingleDownloader {
    client: ClientWithMiddleware,
    pub entry: DownloadEntry,
    total: usize,
    retry_times: usize,
    download_list_index: usize,
    timeout: Duration,
}

pub enum DownloadResult {
    Success(SuccessSummary),
    Failed { file_name: String },
}

#[derive(Debug)]
pub struct SuccessSummary {
    pub file_name: String,
    pub index: usize,
    pub wrote: bool,
    pub url: String,
}

#[bon]
impl SingleDownloader {
    #[builder]
    pub(crate) fn new(
        client: ClientWithMiddleware,
        entry: DownloadEntry,
        total: usize,
        retry_times: usize,
        download_list_index: usize,
        timeout: Duration,
    ) -> Result<SingleDownloader, BuilderError> {
        if entry.source.is_empty() {
            return Err(BuilderError::EmptySource {
                file_name: entry.filename.to_string(),
            });
        }

        Ok(Self {
            client,
            entry,
            total,
            retry_times,
            download_list_index,
            timeout,
        })
    }
}

impl SingleDownloader {
    /// Try to obtain the file from any of its sources, in priority order.
    /// Local sources always sort ahead of HTTP sources.
    pub(crate) async fn try_download(&self, tx: &Sender<Event>) -> DownloadResult {
        if let Err(e) = tokio::fs::create_dir_all(&self.entry.dir).await {
            let _ = tx.send(Event::Failed {
                file_name: self.entry.filename.clone(),
                error: SingleDownloadError::Create(e),
            });
            return DownloadResult::Failed {
                file_name: self.entry.filename.to_string(),
            };
        }

        let msg = self.download_message();

        // If the file already exists at its final destination and matches the
        // checksum, there is nothing to download.
        if let Some(ref final_dir) = self.entry.final_dir {
            let local_file_in_formal = final_dir.join(&*self.entry.filename);

            if local_file_in_formal.is_file()
                && let Some(ref hash) = self.entry.hash
            {
                let mut validator = hash.get_validator();

                if let Ok(mut f) = tokio::fs::File::open(&local_file_in_formal).await {
                    let mut progress =
                        ProgressReporter::new(tx, self.download_list_index, self.total);
                    let result = verify::checksum(&progress, &mut f, &mut validator).await;

                    if result.matches {
                        progress.finish();
                        let _ = tx.send(Event::FileDone { msg: msg.into() });

                        return DownloadResult::Success(SuccessSummary {
                            file_name: self.entry.filename.to_string(),
                            url: self.entry.source.first().unwrap().url.to_string(),
                            index: self.download_list_index,
                            wrote: false,
                        });
                    }

                    // Not a hit: undo the bytes the pre-check added to the
                    // global bar so the real download below starts clean (the
                    // reporter's `Drop` does the undo).
                    progress.set_position(result.bytes);
                }
            }
        }

        let mut sources = self.entry.source.clone();
        assert!(!sources.is_empty());

        sources.sort_unstable_by(|a, b| b.source_type.cmp(&a.source_type));

        for (index, c) in sources.iter().enumerate() {
            let download_res = match &c.source_type {
                DownloadSourceType::Http => self.try_http_download(c, tx).await,
                DownloadSourceType::Local(as_symlink) => {
                    self.download_local(c, *as_symlink, tx).await
                }
            };

            match download_res {
                Ok(wrote) => {
                    if let Some(ref final_dir) = self.entry.final_dir {
                        let current_path = self.entry.dir.join(&*self.entry.filename);
                        let target_path = final_dir.join(&*self.entry.filename);

                        if !final_dir.is_dir()
                            && let Err(e) = tokio::fs::create_dir_all(final_dir).await
                        {
                            let _ = tx.send(Event::Failed {
                                file_name: final_dir.to_string_lossy().to_string(),
                                error: SingleDownloadError::Create(e),
                            });
                            return DownloadResult::Failed {
                                file_name: self.entry.filename.to_string(),
                            };
                        }

                        if current_path.is_file() {
                            trace!(
                                "Moving completed file from {} to {}",
                                current_path.display(),
                                target_path.display()
                            );
                            if let Err(e) = tokio::fs::rename(&current_path, &target_path).await {
                                let _ = tx.send(Event::Failed {
                                    file_name: self.entry.filename.clone(),
                                    error: SingleDownloadError::Write(e),
                                });
                                return DownloadResult::Failed {
                                    file_name: self.entry.filename.to_string(),
                                };
                            }
                        }
                    }

                    let _ = tx.send(Event::FileDone { msg: msg.into() });

                    return DownloadResult::Success(SuccessSummary {
                        file_name: self.entry.filename.to_string(),
                        url: c.url.clone(),
                        index: self.download_list_index,
                        wrote,
                    });
                }
                Err(e) => {
                    // After the last source fails there is nothing left to
                    // fall back to: report the failure and stop.
                    if index + 1 == sources.len() {
                        let _ = tx.send(Event::Failed {
                            file_name: self.entry.filename.clone(),
                            error: e,
                        });
                        return DownloadResult::Failed {
                            file_name: self.entry.filename.to_string(),
                        };
                    }
                    let _ = tx.send(Event::NextUrl {
                        file_name: self.entry.filename.to_string(),
                        err: e,
                    });
                }
            }
        }

        // The loop always returns: the last source's failure is handled inside
        // it, so this is only reachable if `sources` was empty (which the
        // `assert!` above rules out).
        unreachable!()
    }

    /// Message shown next to the per-file progress bar.
    fn download_message(&self) -> String {
        self.entry
            .msg
            .as_deref()
            .unwrap_or(&self.entry.filename)
            .to_string()
    }
}

/// Wrap a raw byte source in the decompressor matching `file_type`, returning
/// an owned reader so callers can `.compat()` it into a tokio reader. Shared
/// by the HTTP and local download paths.
fn decompress_reader(
    raw: impl AsyncRead + Send + Unpin + 'static,
    file_type: CompressType,
) -> Box<dyn AsyncRead + Unpin + Send> {
    let stream = io::BufReader::new(raw);

    match file_type {
        CompressType::Xz => Box::new(XzDecoder::new(stream)),
        CompressType::Gzip => Box::new(GzipDecoder::new(stream)),
        CompressType::Bz2 => Box::new(BzDecoder::new(stream)),
        CompressType::Zstd => Box::new(ZstdDecoder::new(stream)),
        CompressType::Lzma => Box::new(LzmaDecoder::new(stream)),
        CompressType::Lz4 => Box::new(Lz4Decoder::new(stream)),
        CompressType::None => Box::new(stream),
    }
}

/// Test helpers for building ready-to-run downloaders.
#[cfg(test)]
pub(crate) mod test_support {
    use super::*;

    fn build_client(pool_idle_per_host: usize) -> ClientWithMiddleware {
        #[cfg(feature = "rustls")]
        {
            let _ = rustls::crypto::ring::default_provider().install_default();
        }
        reqwest_middleware::ClientBuilder::new(
            reqwest::Client::builder()
                .pool_max_idle_per_host(pool_idle_per_host)
                .build()
                .expect("build reqwest client"),
        )
        .build()
    }

    /// Build a middleware client. Tests that only use local sources never
    /// make network requests, so the concrete backend doesn't matter.
    pub(crate) fn client() -> ClientWithMiddleware {
        build_client(usize::MAX)
    }

    /// A client that never reuses idle connections, for tests that stall the
    /// first connection and rely on the retry opening a fresh one.
    pub(crate) fn client_no_pool() -> ClientWithMiddleware {
        build_client(0)
    }

    /// Build a ready-to-run `SingleDownloader` for one entry.
    pub(crate) fn downloader(entry: DownloadEntry) -> SingleDownloader {
        downloader_with(entry, 1, Duration::from_secs(30))
    }

    /// Like [`downloader`], with explicit retry and timeout settings.
    pub(crate) fn downloader_with(
        entry: DownloadEntry,
        retry_times: usize,
        timeout: Duration,
    ) -> SingleDownloader {
        downloader_with_client(client(), entry, retry_times, timeout)
    }

    /// Like [`downloader`], with a custom client and explicit retry/timeout.
    pub(crate) fn downloader_with_client(
        client: ClientWithMiddleware,
        entry: DownloadEntry,
        retry_times: usize,
        timeout: Duration,
    ) -> SingleDownloader {
        SingleDownloader::builder()
            .client(client)
            .entry(entry)
            .total(1)
            .retry_times(retry_times)
            .download_list_index(0)
            .timeout(timeout)
            .build()
            .expect("valid downloader")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn builder_rejects_empty_sources() {
        let result = SingleDownloader::builder()
            .client(test_support::client())
            .entry(DownloadEntry::default())
            .total(1)
            .retry_times(1)
            .download_list_index(0)
            .timeout(Duration::from_secs(5))
            .build();

        assert!(matches!(result, Err(BuilderError::EmptySource { .. })));
    }
}
