use crate::{CompressType, DownloadSource, Event, checksum::ChecksumValidator, send_request};
use std::{
    borrow::Cow,
    io::{self, SeekFrom},
    path::Path,
    pin::Pin,
    sync::atomic::{AtomicUsize, Ordering},
    task::{Context, Poll},
    time::Duration,
};

use async_compression::futures::bufread::{
    BzDecoder, GzipDecoder, Lz4Decoder, LzmaDecoder, XzDecoder, ZstdDecoder,
};
use bon::bon;
use futures::{AsyncBufRead, AsyncRead, TryStreamExt, io::BufReader};
use headers::{ContentLength, ContentRange, HeaderMapExt};
use reqwest::{Client, Method, RequestBuilder, StatusCode, header::RANGE};
use snafu::{ResultExt, Snafu};
use tokio::{
    fs::{self, File},
    io::{AsyncBufReadExt as _, AsyncReadExt as _, AsyncSeekExt, AsyncWriteExt},
    time::timeout,
};

use spdlog::{debug, trace};
use tokio_util::compat::{FuturesAsyncReadCompatExt, TokioAsyncReadCompatExt};

use crate::{DownloadEntry, DownloadSourceType};

const READ_FILE_BUFSIZE: usize = 65536;
const DOWNLOAD_BUFSIZE: usize = 8192;

#[derive(Debug, Snafu)]
pub enum BuilderError {
    #[snafu(display("Download task {file_name} sources is empty"))]
    EmptySource { file_name: String },
    #[snafu(display("Not allow set illegal download threads: {count}"))]
    IllegalDownloadThread { count: usize },
}

pub(crate) struct SingleDownloader<'a> {
    client: &'a Client,
    pub entry: &'a DownloadEntry,
    total: usize,
    retry_times: usize,
    msg: Option<Cow<'static, str>>,
    download_list_index: usize,
    file_type: CompressType,
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

#[derive(Debug, Snafu)]
pub enum SingleDownloadError {
    #[snafu(display("Failed to set permission"))]
    SetPermission { source: io::Error },
    #[snafu(display("Failed to open file as rw mode"))]
    OpenAsWriteMode { source: io::Error },
    #[snafu(display("Failed to open file"))]
    Open { source: io::Error },
    #[snafu(display("Failed to create file"))]
    Create { source: io::Error },
    #[snafu(display("Failed to seek file"))]
    Seek { source: io::Error },
    #[snafu(display("Failed to write file"))]
    Write { source: io::Error },
    #[snafu(display("Failed to flush file"))]
    Flush { source: io::Error },
    #[snafu(display("Failed to remove file"))]
    Remove { source: io::Error },
    #[snafu(display("Failed to create symlink"))]
    CreateSymlink { source: io::Error },
    #[snafu(display("Request Error"))]
    ReqwestError { source: reqwest::Error },
    #[snafu(display("Broken pipe"))]
    BrokenPipe { source: io::Error },
    #[snafu(display("Send request timeout"))]
    SendRequestTimeout,
    #[snafu(display("Download file timeout"))]
    DownloadTimeout,
    #[snafu(display("checksum mismatch"))]
    ChecksumMismatch,
}

#[bon]
impl<'a> SingleDownloader<'a> {
    #[builder]
    pub(crate) fn new(
        client: &'a Client,
        entry: &'a DownloadEntry,
        total: usize,
        retry_times: usize,
        msg: Option<Cow<'static, str>>,
        download_list_index: usize,
        file_type: CompressType,
        timeout: Duration,
    ) -> Result<SingleDownloader<'a>, BuilderError> {
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
            msg,
            download_list_index,
            file_type,
            timeout,
        })
    }

    pub(crate) async fn try_download(self, callback: &impl AsyncFn(Event)) -> DownloadResult {
        let mut sources = self.entry.source.clone();
        assert!(!sources.is_empty());

        sources.sort_unstable_by(|a, b| b.source_type.cmp(&a.source_type));

        let msg = self.msg.as_deref().unwrap_or(&*self.entry.filename);

        for (index, c) in sources.iter().enumerate() {
            let download_res = match &c.source_type {
                DownloadSourceType::Http { auth } => {
                    self.try_http_download(c, auth, callback).await
                }
                DownloadSourceType::Local(as_symlink) => {
                    self.download_local(c, *as_symlink, callback).await
                }
            };

            match download_res {
                Ok(b) => {
                    callback(Event::DownloadDone {
                        index: self.download_list_index,
                        msg: msg.into(),
                    })
                    .await;

                    return DownloadResult::Success(SuccessSummary {
                        file_name: self.entry.filename.to_string(),
                        url: c.url.clone(),
                        index: self.download_list_index,
                        wrote: b,
                    });
                }
                Err(e) => {
                    if index == sources.len() - 1 {
                        callback(Event::Failed {
                            file_name: self.entry.filename.clone(),
                            error: e,
                        })
                        .await;

                        return DownloadResult::Failed {
                            file_name: self.entry.filename.to_string(),
                        };
                    }

                    callback(Event::NextUrl {
                        index: self.download_list_index,
                        file_name: self.entry.filename.to_string(),
                        err: e,
                    })
                    .await;
                }
            }
        }

        unreachable!()
    }

    /// Download file with retry (http)
    async fn try_http_download(
        &self,
        source: &DownloadSource,
        auth: &Option<(String, String)>,
        callback: &impl AsyncFn(Event),
    ) -> Result<bool, SingleDownloadError> {
        let mut times = 1;
        let mut allow_resume = self.entry.allow_resume;
        loop {
            match self
                .http_download(allow_resume, source, auth, callback)
                .await
            {
                Ok(s) => {
                    return Ok(s);
                }
                Err(e) => match e {
                    SingleDownloadError::ChecksumMismatch => {
                        if self.retry_times == times {
                            return Err(e);
                        }

                        if times > 1 {
                            callback(Event::ChecksumMismatch {
                                index: self.download_list_index,
                                filename: self.entry.filename.to_string(),
                                times,
                            })
                            .await;
                        }

                        times += 1;
                        allow_resume = false;
                    }
                    SingleDownloadError::DownloadTimeout => {
                        if self.retry_times == times {
                            return Err(e);
                        }

                        if times > 1 {
                            callback(Event::Timeout {
                                filename: self.entry.filename.to_string(),
                                times,
                            })
                            .await;
                        }

                        times += 1;
                    }
                    e => {
                        return Err(e);
                    }
                },
            }
        }
    }

    async fn http_download(
        &self,
        allow_resume: bool,
        source: &DownloadSource,
        auth: &Option<(String, String)>,
        callback: &impl AsyncFn(Event),
    ) -> Result<bool, SingleDownloadError> {
        let file = self.entry.dir.join(&*self.entry.filename);
        let file_exist = file.exists();
        let file_size = file.metadata().ok().map(|x| x.len()).unwrap_or(0);

        trace!("{} Exist file size is: {file_size}", file.display());
        trace!("{} download url is: {}", file.display(), source.url);
        let is_symlink = file.is_symlink();

        debug!("file {} is symlink = {}", file.display(), is_symlink);

        if is_symlink {
            tokio::fs::remove_file(&file).await.context(RemoveSnafu)?;
        }

        // downloaded HTTP content representation size
        let mut downloaded_size: u64 = 0;
        let mut old_downloaded_size: u64 = 0;

        let mut validator = self
            .entry
            .hash
            .as_ref()
            .map(|hash| hash.get_validator())
            .unwrap_or(ChecksumValidator::None);

        if file_exist && !is_symlink {
            trace!(
                "File {} already exists, verifying checksum ...",
                self.entry.filename
            );
            downloaded_size = file_size;

            if let Some(hash) = &self.entry.hash {
                trace!("Hash {} exists for the existing file.", hash);

                let mut f = tokio::fs::OpenOptions::new()
                    .read(true)
                    .open(&file)
                    .await
                    .context(OpenSnafu)?;

                let (read, finish) = checksum(callback, &mut f, &mut validator).await;

                if finish {
                    trace!("checksum of {} matches, cache hit!", self.entry.filename);
                    callback(Event::ProgressDone(self.download_list_index)).await;
                    return Ok(false);
                }

                debug!(
                    "checksum mismatch, initiating re-download for file {} ...",
                    self.entry.filename
                );
                old_downloaded_size = read;
            }

            if self.entry.file_type != CompressType::None {
                downloaded_size = 0;
            }
        }

        callback(Event::NewProgressSpinner {
            index: self.download_list_index,
            msg: self.download_message(),
            total: self.total,
        })
        .await;

        // open destination file
        let mut dest = match tokio::fs::OpenOptions::new()
            .write(true)
            .read(true)
            .create(true)
            .truncate(false)
            .open(&file)
            .await
        {
            Ok(f) => f,
            Err(e) => {
                callback(Event::ProgressDone(self.download_list_index)).await;
                return Err(SingleDownloadError::Create { source: e });
            }
        };

        let mut buf = Vec::with_capacity(DOWNLOAD_BUFSIZE);
        #[allow(clippy::uninit_vec)]
        unsafe {
            buf.set_len(DOWNLOAD_BUFSIZE)
        };
        let mut total_size: Option<u64> = None;
        let mut first_request = true;
        'download: while if let Some(total_size) = total_size {
            downloaded_size < total_size
        } else {
            true
        } {
            if !allow_resume || self.entry.file_type != CompressType::None {
                // restart download if resume is disallowed
                downloaded_size = 0;
            }

            let old_total_size = total_size;

            // send request
            let mut req = self.build_request_with_basic_auth(&source.url, Method::GET, auth);
            if downloaded_size != 0 {
                // request for resume
                // assume reqwest's automatical decompression is disabled
                debug!("sending partial request ...");
                req = req.header(RANGE, format!("bytes={downloaded_size}-"));
            } else {
                debug!("sending complete request ...");
            }
            let resp = timeout(self.timeout, send_request(&source.url, req)).await;
            let resp = match resp {
                Ok(Ok(resp)) => resp,
                Ok(Err(e)) => match e.status() {
                    Some(StatusCode::RANGE_NOT_SATISFIABLE) => {
                        debug!("range not satisfiable from server, restarting ...");
                        downloaded_size = 0;
                        continue 'download;
                    }
                    Some(StatusCode::BAD_REQUEST) => {
                        // some servers reply with Bad Request when Range is invalid
                        // so retry once
                        if downloaded_size == 0 {
                            return Err(SingleDownloadError::ReqwestError { source: e });
                        } else {
                            debug!("HTTP Bad Request from server, restarting ...");
                            downloaded_size = 0;
                            continue 'download;
                        }
                    }
                    _ => return Err(SingleDownloadError::ReqwestError { source: e }),
                },
                Err(_) => {
                    return Err(SingleDownloadError::SendRequestTimeout);
                }
            };

            // check resume
            let resp_headers = resp.headers();
            'resume: {
                if resp.status() == StatusCode::PARTIAL_CONTENT {
                    match resp_headers.typed_get::<ContentRange>() {
                        Some(range) => {
                            // update total size if possible
                            if let Some(new_total_size) = range.bytes_len() {
                                debug!(
                                    "extracted complete length from Content-Range: {new_total_size}"
                                );
                                total_size = Some(new_total_size);
                            }

                            // check returned range is the expected
                            if let Some((returned_start, _)) = range.bytes_range() {
                                if returned_start != downloaded_size {
                                    // The server didn't send us the request range
                                    // Implementing part combination is too complex, just restart it
                                    debug!("incomplete Content-Range, restarting ...");
                                    downloaded_size = 0;
                                    continue 'download;
                                }
                                debug!("partial request succeeded");
                                break 'resume;
                            } else {
                                // Unsatisfiable Content-Range should never appear in HTTP 206
                                // per RFC 9110. The server implementation is violating RFC.
                                debug!("unsatisfiable Content-Range in HTTP 206, restarting ...");
                                downloaded_size = 0;
                                continue 'download;
                            }
                        }
                        None => {
                            debug!("multi-parts are not supported, restarting ...");
                            downloaded_size = 0;
                            continue 'download;
                        }
                    }
                }
                if resp.status() == StatusCode::OK {
                    // update total size if possible.
                    // Content-Length is the complete length for OK but it may not be the case for other statuses
                    if let Some(length) = resp_headers.typed_get::<ContentLength>() {
                        let length = length.0;
                        debug!("extracted complete length from Content-Length: {length}");
                        total_size = Some(length);
                    }
                }
                if downloaded_size != 0 {
                    // requested partial response, but not getting expected response
                    debug!("range request failed");
                    downloaded_size = 0;
                    // no need to re-send request in this case, the body is already complete
                }
            }
            debug!("response body is at {downloaded_size}/{total_size:?}");

            // seek to expected location
            if downloaded_size != old_downloaded_size {
                assert!(downloaded_size == 0 || self.entry.file_type == CompressType::None);
                debug!("moving writer from {old_downloaded_size} to {downloaded_size}");
                if let Err(e) = dest.seek(SeekFrom::Start(0)).await {
                    callback(Event::ProgressDone(self.download_list_index)).await;
                    return Err(SingleDownloadError::Seek { source: e });
                }

                if downloaded_size == 0 {
                    validator.reset();
                } else {
                    {
                        // refresh hasher state
                        let mut dest_buf = Vec::with_capacity(downloaded_size.try_into().unwrap());
                        if let Err(e) = dest.read_to_end(&mut dest_buf).await {
                            callback(Event::ProgressDone(self.download_list_index)).await;
                            return Err(SingleDownloadError::Seek { source: e });
                        }
                        validator.reset();
                        validator.update(dest_buf);
                    }

                    if let Err(e) = dest.seek(SeekFrom::Start(downloaded_size)).await {
                        callback(Event::ProgressDone(self.download_list_index)).await;
                        return Err(SingleDownloadError::Seek { source: e });
                    }
                }

                // truncate file
                if let Err(e) = dest.set_len(downloaded_size).await {
                    callback(Event::ProgressDone(self.download_list_index)).await;
                    return Err(SingleDownloadError::Write { source: e });
                }
            }

            // update progress
            if old_total_size != total_size
                || old_downloaded_size > downloaded_size
                || first_request
            {
                // recreate the progress bar if:
                // 1. total size updated
                // 2. offset moved backwards
                // 3. is the first request (the previous bar is a spinner)
                first_request = false;
                callback(Event::ProgressDone(self.download_list_index)).await;
                callback(Event::NewProgressBar {
                    index: self.download_list_index,
                    msg: self.download_message(),
                    total: self.total,
                    size: total_size.unwrap_or(0),
                })
                .await;
                callback(Event::ProgressInc {
                    index: self.download_list_index,
                    size: downloaded_size,
                })
                .await;
                if old_downloaded_size != downloaded_size {
                    callback(Event::GlobalProgressSub(old_downloaded_size)).await;
                    callback(Event::GlobalProgressAdd(downloaded_size)).await;
                }
            } else if old_downloaded_size < downloaded_size {
                let new_offset = downloaded_size - old_downloaded_size;
                callback(Event::ProgressInc {
                    index: self.download_list_index,
                    size: new_offset,
                })
                .await;
                callback(Event::GlobalProgressAdd(new_offset)).await;
            }

            old_downloaded_size = downloaded_size;

            let stream = resp
                .bytes_stream()
                .map_err(io::Error::other)
                .into_async_read();
            let stream = BufReader::new(stream);
            let stream_counter = AtomicUsize::new(0);
            let mut stream = Counter::new(stream, &stream_counter);

            // initialize decompressor
            let reader: &mut (dyn AsyncRead + Unpin + Send) = match self.file_type {
                CompressType::Xz => &mut XzDecoder::new(&mut stream),
                CompressType::Gzip => &mut GzipDecoder::new(&mut stream),
                CompressType::Bz2 => &mut BzDecoder::new(&mut stream),
                CompressType::Zstd => &mut ZstdDecoder::new(&mut stream),
                CompressType::Lzma => &mut LzmaDecoder::new(&mut stream),
                CompressType::Lz4 => &mut Lz4Decoder::new(&mut stream),
                CompressType::None => &mut stream,
            };
            let mut reader = reader.compat();

            // copy data
            loop {
                let buf_size = match timeout(self.timeout, reader.read(&mut buf[..])).await {
                    Ok(Ok(size)) => size,
                    Ok(Err(e)) => {
                        callback(Event::ProgressDone(self.download_list_index)).await;
                        return Err(SingleDownloadError::BrokenPipe { source: e });
                    }
                    Err(_) => {
                        callback(Event::ProgressDone(self.download_list_index)).await;
                        return Err(SingleDownloadError::DownloadTimeout);
                    }
                };
                if buf_size == 0 {
                    break; // EOF
                }
                if let Err(e) = dest.write_all(&buf[..buf_size]).await {
                    callback(Event::ProgressDone(self.download_list_index)).await;
                    return Err(SingleDownloadError::Write { source: e });
                }
                validator.update(&buf[..buf_size]);

                let http_size = stream_counter.swap(0, Ordering::AcqRel);
                let http_size: u64 = http_size.try_into().unwrap();
                downloaded_size += http_size;
                callback(Event::ProgressInc {
                    index: self.download_list_index,
                    size: http_size,
                })
                .await;
                callback(Event::GlobalProgressAdd(http_size)).await;
            }

            debug!("downloaded {} bytes", downloaded_size - old_downloaded_size);
            if downloaded_size == old_downloaded_size {
                // this should not happen ...
                break 'download;
            }

            if total_size.is_none() {
                // total size is unknown, we have to assume that the body is complete
                break 'download;
            }

            old_downloaded_size = downloaded_size;
        }
        debug!("download end, {downloaded_size} bytes");
        callback(Event::ProgressDone(self.download_list_index)).await;

        // verify checksum
        if !validator.finish() {
            debug!("checksum mismatch for {}", self.entry.filename);
            callback(Event::GlobalProgressSub(downloaded_size)).await;
            callback(Event::ProgressDone(self.download_list_index)).await;

            // truncate file, avoid attempts to reuse it in retries
            if let Err(e) = dest.set_len(0).await {
                callback(Event::ProgressDone(self.download_list_index)).await;
                return Err(SingleDownloadError::Write { source: e });
            }

            return Err(SingleDownloadError::ChecksumMismatch);
        }
        if matches!(validator, ChecksumValidator::None) {
            trace!(
                "checksum verification succeeded for {}",
                self.entry.filename
            );
        }

        // flush
        if let Err(e) = dest.shutdown().await {
            callback(Event::ProgressDone(self.download_list_index)).await;
            return Err(SingleDownloadError::Flush { source: e });
        }

        callback(Event::ProgressDone(self.download_list_index)).await;
        Ok(true)
    }

    fn build_request_with_basic_auth(
        &self,
        url: &str,
        method: Method,
        auth: &Option<(String, String)>,
    ) -> RequestBuilder {
        let mut req = self.client.request(method, url);

        if let Some((user, password)) = auth {
            trace!("Authenticating as user: {} ...", user);
            req = req.basic_auth(user, Some(password));
        }

        req
    }

    /// Download local source file
    async fn download_local(
        &self,
        source: &DownloadSource,
        as_symlink: bool,
        callback: &impl AsyncFn(Event),
    ) -> Result<bool, SingleDownloadError> {
        debug!("{:?}", self.entry);

        let url = source.url.strip_prefix("file:").unwrap();

        let url_path = Path::new(url);

        let total_size = tokio::fs::metadata(url_path)
            .await
            .context(OpenSnafu)?
            .len();

        let file = self.entry.dir.join(&*self.entry.filename);
        if file.is_symlink() || (as_symlink && file.is_file()) {
            tokio::fs::remove_file(&file).await.context(RemoveSnafu)?;
        }

        if as_symlink {
            if let Some(hash) = &self.entry.hash {
                self.checksum_local(callback, url_path, hash).await?;
            }

            tokio::fs::symlink(url_path, file)
                .await
                .context(CreateSymlinkSnafu)?;

            return Ok(true);
        }

        callback(Event::NewProgressBar {
            index: self.download_list_index,
            total: self.total,
            msg: self.download_message(),
            size: total_size,
        })
        .await;

        trace!("Path for file: {}", url_path.display());

        let from = File::open(&url_path).await.context(CreateSnafu)?;
        let from = tokio::io::BufReader::new(from).compat();

        trace!("Successfully opened file: {}", url_path.display());

        let mut to = File::create(self.entry.dir.join(&*self.entry.filename))
            .await
            .context(CreateSnafu)?;

        let reader: &mut (dyn AsyncRead + Unpin + Send) = match self.file_type {
            CompressType::Xz => &mut XzDecoder::new(BufReader::new(from)),
            CompressType::Gzip => &mut GzipDecoder::new(BufReader::new(from)),
            CompressType::Bz2 => &mut BzDecoder::new(BufReader::new(from)),
            CompressType::Zstd => &mut ZstdDecoder::new(BufReader::new(from)),
            CompressType::Lzma => &mut LzmaDecoder::new(BufReader::new(from)),
            CompressType::Lz4 => &mut Lz4Decoder::new(BufReader::new(from)),
            CompressType::None => &mut BufReader::new(from),
        };

        let mut reader = reader.compat();

        trace!(
            "Successfully created file: {}",
            self.entry.dir.join(&*self.entry.filename).display()
        );

        let mut v = self.entry.hash.as_ref().map(|v| v.get_validator());

        let mut buf = vec![0u8; READ_FILE_BUFSIZE];
        let mut self_progress = 0;

        loop {
            let size = reader.read(&mut buf[..]).await.context(BrokenPipeSnafu)?;
            self_progress += size;

            if size == 0 {
                break;
            }

            to.write_all(&buf[..size]).await.context(WriteSnafu)?;

            callback(Event::ProgressInc {
                index: self.download_list_index,
                size: size as u64,
            })
            .await;

            if let Some(ref mut v) = v {
                v.update(&buf[..size]);
            }

            callback(Event::GlobalProgressAdd(size as u64)).await;
        }

        if v.is_some_and(|v| !v.finish()) {
            callback(Event::GlobalProgressSub(self_progress as u64)).await;
            callback(Event::ProgressDone(self.download_list_index)).await;
            return Err(SingleDownloadError::ChecksumMismatch);
        }

        callback(Event::ProgressDone(self.download_list_index)).await;

        Ok(true)
    }

    async fn checksum_local(
        &self,
        callback: &impl AsyncFn(Event),
        url_path: &Path,
        hash: &crate::checksum::Checksum,
    ) -> Result<(), SingleDownloadError> {
        let mut f = fs::File::open(url_path).await.context(OpenSnafu)?;
        let (size, finish) = checksum(callback, &mut f, &mut hash.get_validator()).await;

        if !finish {
            callback(Event::GlobalProgressSub(size)).await;
            callback(Event::ProgressDone(self.download_list_index)).await;
            return Err(SingleDownloadError::ChecksumMismatch);
        }

        Ok(())
    }

    fn download_message(&self) -> String {
        self.msg
            .as_deref()
            .unwrap_or(&self.entry.filename)
            .to_string()
    }
}

async fn checksum(
    callback: &impl AsyncFn(Event),
    f: &mut File,
    v: &mut ChecksumValidator,
) -> (u64, bool) {
    let mut reader = tokio::io::BufReader::with_capacity(READ_FILE_BUFSIZE, f);

    let mut read = 0;

    loop {
        let buffer = match reader.fill_buf().await {
            Ok([]) => break,
            Ok(buffer) => buffer,
            Err(e) => {
                debug!("Error while reading file: {e}");
                break;
            }
        };

        v.update(buffer);

        callback(Event::GlobalProgressAdd(buffer.len() as u64)).await;
        read += buffer.len() as u64;
        let len = buffer.len();

        reader.consume(len);
    }

    (read, v.finish())
}

pub struct Counter<'a, D> {
    inner: D,
    bytes: &'a AtomicUsize,
}

impl<'a, D> Counter<'a, D> {
    #[inline]
    pub const fn new(inner: D, bytes: &'a AtomicUsize) -> Self {
        Self { inner, bytes }
    }

    #[inline]
    pub fn take_read_bytes(&self) -> usize {
        self.bytes.swap(0, Ordering::AcqRel)
    }
}

impl<R: AsyncRead + Unpin> AsyncRead for Counter<'_, R> {
    fn poll_read(
        self: Pin<&mut Self>,
        ctx: &mut Context<'_>,
        buf: &mut [u8],
    ) -> Poll<std::io::Result<usize>> {
        let counter = self.get_mut();
        let pin = Pin::new(&mut counter.inner);

        let poll = pin.poll_read(ctx, buf);
        if let Poll::Ready(Ok(bytes)) = poll {
            counter.bytes.fetch_add(bytes, Ordering::AcqRel);
        }

        poll
    }
}

impl<R: AsyncBufRead + Unpin> AsyncBufRead for Counter<'_, R> {
    fn poll_fill_buf(self: Pin<&mut Self>, ctx: &mut Context<'_>) -> Poll<std::io::Result<&[u8]>> {
        let counter = self.get_mut();
        let pin = Pin::new(&mut counter.inner);
        pin.poll_fill_buf(ctx)
    }

    fn consume(self: Pin<&mut Self>, amt: usize) {
        let counter = self.get_mut();
        counter.bytes.fetch_add(amt, Ordering::AcqRel);
        let pin = Pin::new(&mut counter.inner);
        pin.consume(amt);
    }
}
