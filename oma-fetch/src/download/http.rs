//! HTTP download implementation for [`SingleDownloader`](super::SingleDownloader):
//! request sending, range/resume negotiation, destination file management,
//! progress syncing and body streaming.

use std::{
    io::{self, SeekFrom},
    path::Path,
    sync::atomic::{AtomicUsize, Ordering},
};

use async_compression::futures::bufread::{
    BzDecoder, GzipDecoder, Lz4Decoder, LzmaDecoder, XzDecoder, ZstdDecoder,
};
use flume::Sender;
use futures::{AsyncRead, TryStreamExt, io::BufReader};
use headers::{ContentLength, ContentRange, HeaderMapExt};
use reqwest::{Method, StatusCode, header::RANGE};
use spdlog::{debug, trace};
use tokio::{
    fs::{File, OpenOptions},
    io::{AsyncReadExt as _, AsyncSeekExt, AsyncWriteExt},
    time::timeout,
};
use tokio_util::compat::FuturesAsyncReadCompatExt;

use crate::{CompressType, DownloadSource, Event, checksum::ChecksumValidator, send_request};

use super::{
    DOWNLOAD_BUFSIZE, SingleDownloader,
    counter::Counter,
    error::SingleDownloadError,
    progress::{DownloadState, ProgressReporter, RequestOutcome, ResumeOutcome},
    verify,
};

impl SingleDownloader {
    /// Download file with retry (http)
    pub(super) async fn try_http_download(
        &self,
        source: &DownloadSource,
        tx: &Sender<Event>,
    ) -> Result<bool, SingleDownloadError> {
        let mut times = 1;
        let mut allow_resume = self.entry.allow_resume;
        loop {
            // A fresh reporter per attempt: its `Drop` clears the per-file bar
            // (even on error), and `reported()` tells us how many bytes to undo
            // on the global bar before retrying.
            let mut progress = ProgressReporter::new(tx, self.download_list_index, self.total);

            match self
                .http_download(allow_resume, source, tx, &mut progress)
                .await
            {
                Ok(s) => {
                    return Ok(s);
                }
                Err(e) => {
                    // Undo this attempt's global progress contribution before
                    // retrying or handing the failure up, so the retry (or the
                    // next source) starts from a clean bar.
                    let bytes = progress.reported();
                    drop(progress);
                    if bytes != 0 {
                        let _ = tx.send(Event::GlobalProgressSub(bytes));
                    }
                    match e {
                        SingleDownloadError::ChecksumMismatch => {
                            if self.retry_times == times {
                                return Err(e);
                            }

                            if times > 1 {
                                let _ = tx.send(Event::ChecksumMismatch {
                                    index: self.download_list_index,
                                    filename: self.entry.filename.to_string(),
                                    times,
                                });
                            }

                            times += 1;
                            allow_resume = false;
                        }
                        SingleDownloadError::DownloadTimeout => {
                            if self.retry_times == times {
                                return Err(e);
                            }

                            if times > 1 {
                                let _ = tx.send(Event::Timeout {
                                    filename: self.entry.filename.to_string(),
                                    times,
                                });
                            }

                            times += 1;
                        }
                        e => {
                            return Err(e);
                        }
                    }
                }
            }
        }
    }

    /// Inner download attempt. The caller owns the [`ProgressReporter`], so on
    /// error it can read `reported()` and undo the bytes this attempt already
    /// added to the global progress bar before retrying / trying the next
    /// source. The per-file bar is cleared when the reporter drops.
    async fn http_download(
        &self,
        allow_resume: bool,
        source: &DownloadSource,
        tx: &Sender<Event>,
        progress: &mut ProgressReporter,
    ) -> Result<bool, SingleDownloadError> {
        let file = self.entry.dir.join(&*self.entry.filename);

        trace!("{} download url is: {}", file.display(), source.url);

        let mut validator = self
            .entry
            .hash
            .as_ref()
            .map(|hash| hash.get_validator())
            .unwrap_or(ChecksumValidator::None);
        let mut state = DownloadState::new();

        // The phases below report progress only on success and propagate plain
        // errors with `?`; the per-file bar is cleared automatically when the
        // reporter is dropped below.
        let result: Result<bool, SingleDownloadError> = async {
            // 1. Remove stale symlinks and seed the resume offset from an
            //    existing file; a matching checksum is a cache hit.
            if self
                .prepare_existing_file(&file, tx, &mut state, &mut validator)
                .await?
            {
                return Ok(false);
            }

            progress.spinner(&self.download_message());

            // 2. Open the destination without truncating, so resuming works.
            let mut dest = self.open_destination(&file).await?;

            // 3. Fetch the body, resuming as needed.
            self.download_loop(
                source,
                allow_resume,
                &mut dest,
                &mut validator,
                &mut state,
                progress,
            )
            .await?;

            // 4. Verify the checksum and flush the file.
            self.finish_download(&mut dest, &mut validator).await?;

            Ok(true)
        }
        .await;

        result
    }

    /// Inspect the (possibly existing) destination file: remove a stale
    /// symlink, and for an existing regular file seed the resume offset and
    /// hasher so the download can continue where it left off. Returns `true`
    /// when the existing file already matches the checksum (a cache hit, so
    /// nothing needs to be downloaded).
    async fn prepare_existing_file(
        &self,
        file: &Path,
        tx: &Sender<Event>,
        state: &mut DownloadState,
        validator: &mut ChecksumValidator,
    ) -> Result<bool, SingleDownloadError> {
        let file_exist = file.exists();
        let file_size = file.metadata().ok().map(|x| x.len()).unwrap_or(0);
        let is_symlink = file.is_symlink();

        trace!("{} Exist file size is: {file_size}", file.display());
        debug!("file {} is symlink = {}", file.display(), is_symlink);

        if is_symlink {
            tokio::fs::remove_file(file)
                .await
                .map_err(|source| SingleDownloadError::Remove { source })?;
        }

        if !file_exist || is_symlink {
            return Ok(false);
        }

        trace!(
            "File {} already exists, verifying checksum ...",
            self.entry.filename
        );
        state.downloaded_size = file_size;

        if let Some(hash) = &self.entry.hash {
            trace!("Hash {} exists for the existing file.", hash);

            let mut f = OpenOptions::new()
                .read(true)
                .open(file)
                .await
                .map_err(|source| SingleDownloadError::Open { source })?;

            let (read, finish) = verify::checksum(tx, &mut f, validator).await;

            if finish {
                trace!("checksum of {} matches, cache hit!", self.entry.filename);
                return Ok(true);
            }

            debug!(
                "checksum mismatch, initiating re-download for file {} ...",
                self.entry.filename
            );
            state.old_downloaded_size = read;
        }

        if self.entry.file_type != CompressType::None {
            state.downloaded_size = 0;
        }

        Ok(false)
    }

    /// Open (or create) the destination file for writing without truncating,
    /// so an interrupted download can be resumed from its current length.
    async fn open_destination(&self, file: &Path) -> Result<File, SingleDownloadError> {
        OpenOptions::new()
            .write(true)
            .read(true)
            .create(true)
            .truncate(false)
            .open(file)
            .await
            .map_err(|source| SingleDownloadError::Create { source })
    }

    /// Repeatedly fetch the body, resuming from the current offset, until the
    /// file is complete. Restarts the loop whenever the server doesn't honor
    /// our range request.
    async fn download_loop(
        &self,
        source: &DownloadSource,
        allow_resume: bool,
        dest: &mut File,
        validator: &mut ChecksumValidator,
        state: &mut DownloadState,
        progress: &mut ProgressReporter,
    ) -> Result<(), SingleDownloadError> {
        let mut buf = read_buffer();

        'download: while state.in_progress() {
            state.begin_attempt(allow_resume, self.entry.file_type);

            // 1. send request (resume from `downloaded_size` when non-zero)
            let resp = match self.send_request(source, state).await {
                RequestOutcome::Ready(resp) => resp,
                RequestOutcome::Restart => continue 'download,
                RequestOutcome::Fatal(error) => return Err(error),
            };

            // 2. reconcile the response with our resume offset
            if Self::negotiate_resume(&resp, state) == ResumeOutcome::Restart {
                continue 'download;
            }

            debug!(
                "response body is at {}/{:?}",
                state.downloaded_size, state.total_size
            );

            let is_complete = resp.status() == StatusCode::OK;

            // 3. position + truncate the destination file
            self.prepare_destination(dest, validator, state).await?;

            // 4. (re)create or advance the progress bar
            self.sync_progress(progress, state);
            progress.set_reported(state.downloaded_size);

            // 5. stream the body into the file (decompressing as needed)
            self.copy_body(resp, dest, validator, &mut buf, state, progress)
                .await?;

            debug!(
                "downloaded {} bytes",
                state.downloaded_size - state.old_downloaded_size
            );

            if state.downloaded_size == state.old_downloaded_size {
                // this should not happen ...
                break 'download;
            }

            if is_complete || state.total_size.is_none() {
                // total size is unknown, we have to assume that the body is complete
                break 'download;
            }

            state.old_downloaded_size = state.downloaded_size;
        }

        debug!("download end, {} bytes", state.downloaded_size);

        Ok(())
    }

    /// Verify the checksum and flush the destination file. On a mismatch the
    /// file is truncated first so retries don't accidentally reuse it.
    async fn finish_download(
        &self,
        dest: &mut File,
        validator: &mut ChecksumValidator,
    ) -> Result<(), SingleDownloadError> {
        if !validator.finish() {
            debug!("checksum mismatch for {}", self.entry.filename);

            // truncate file, avoid attempts to reuse it in retries
            dest.set_len(0)
                .await
                .map_err(|source| SingleDownloadError::Write { source })?;

            return Err(SingleDownloadError::ChecksumMismatch);
        }

        if matches!(validator, ChecksumValidator::None) {
            trace!(
                "checksum verification succeeded for {}",
                self.entry.filename
            );
        }

        // flush
        dest.shutdown()
            .await
            .map_err(|source| SingleDownloadError::Flush { source })?;

        Ok(())
    }

    /// Send one HTTP request, resuming from `downloaded_size` when non-zero.
    async fn send_request(
        &self,
        source: &DownloadSource,
        state: &mut DownloadState,
    ) -> RequestOutcome {
        let mut req = self.client.request(Method::GET, &source.url);

        if state.downloaded_size != 0 {
            // request for resume
            // assume reqwest's automatic decompression is disabled
            debug!("sending partial request ...");
            req = req.header(RANGE, format!("bytes={}-", state.downloaded_size));
        } else {
            debug!("sending complete request ...");
        }
        let resp = timeout(self.timeout, send_request(req)).await;

        match resp {
            Ok(Ok(resp)) => RequestOutcome::Ready(resp),
            Ok(Err(e)) => match e.status() {
                Some(StatusCode::RANGE_NOT_SATISFIABLE) => {
                    debug!("range not satisfiable from server, restarting ...");
                    state.restart();
                    RequestOutcome::Restart
                }
                Some(StatusCode::BAD_REQUEST) => {
                    // some servers reply with Bad Request when Range is invalid
                    // so retry once
                    if state.downloaded_size == 0 {
                        RequestOutcome::Fatal(SingleDownloadError::ReqwestMiddlewareError {
                            source: e,
                        })
                    } else {
                        debug!("HTTP Bad Request from server, restarting ...");
                        state.restart();
                        RequestOutcome::Restart
                    }
                }
                _ => {
                    RequestOutcome::Fatal(SingleDownloadError::ReqwestMiddlewareError { source: e })
                }
            },
            Err(_) => RequestOutcome::Fatal(SingleDownloadError::SendRequestTimeout),
        }
    }

    /// Check the response against our resume offset and update the total size
    /// when the server provides one.
    fn negotiate_resume(resp: &reqwest::Response, state: &mut DownloadState) -> ResumeOutcome {
        let resp_headers = resp.headers();
        if resp.status() == StatusCode::PARTIAL_CONTENT {
            match resp_headers.typed_get::<ContentRange>() {
                Some(range) => {
                    // update total size if possible
                    if let Some(new_total_size) = range.bytes_len() {
                        debug!("extracted complete length from Content-Range: {new_total_size}");
                        state.total_size = Some(new_total_size);
                    }

                    // check returned range is the expected
                    if let Some((returned_start, _)) = range.bytes_range() {
                        if returned_start != state.downloaded_size {
                            // The server didn't send us the request range
                            // Implementing part combination is too complex, just restart it
                            debug!("incomplete Content-Range, restarting ...");
                            state.restart();
                            return ResumeOutcome::Restart;
                        }
                        debug!("partial request succeeded");
                        return ResumeOutcome::Proceed;
                    } else {
                        // Unsatisfiable Content-Range should never appear in HTTP 206
                        // per RFC 9110. The server implementation is violating RFC.
                        debug!("unsatisfiable Content-Range in HTTP 206, restarting ...");
                        state.restart();
                        return ResumeOutcome::Restart;
                    }
                }
                None => {
                    debug!("multi-parts are not supported, restarting ...");
                    state.restart();
                    return ResumeOutcome::Restart;
                }
            }
        }

        if resp.status() == StatusCode::OK {
            // update total size if possible.
            // Content-Length is the complete length for OK but it may not be the case for other statuses
            if let Some(length) = resp_headers.typed_get::<ContentLength>() {
                let length = length.0;
                debug!("extracted complete length from Content-Length: {length}");
                state.total_size = Some(length);
            }
        }

        if state.downloaded_size != 0 {
            // requested partial response, but not getting expected response
            debug!("range request failed");
            state.restart();
            // no need to re-send request in this case, the body is already complete
        }

        ResumeOutcome::Proceed
    }

    /// Position the destination file at `downloaded_size` and refresh the
    /// hasher state, so the incoming body can be appended and verified.
    async fn prepare_destination(
        &self,
        dest: &mut File,
        validator: &mut ChecksumValidator,
        state: &mut DownloadState,
    ) -> Result<(), SingleDownloadError> {
        if state.downloaded_size != state.old_downloaded_size {
            assert!(state.downloaded_size == 0 || self.entry.file_type == CompressType::None);
            debug!(
                "moving writer from {} to {}",
                state.old_downloaded_size, state.downloaded_size
            );
            dest.seek(SeekFrom::Start(0))
                .await
                .map_err(|source| SingleDownloadError::Seek { source })?;

            if state.downloaded_size == 0 {
                validator.reset();
            } else {
                // refresh hasher state
                let mut dest_buf = Vec::with_capacity(state.downloaded_size.try_into().unwrap());
                dest.read_to_end(&mut dest_buf)
                    .await
                    .map_err(|source| SingleDownloadError::Seek { source })?;

                validator.reset();
                validator.update(dest_buf);

                dest.seek(SeekFrom::Start(state.downloaded_size))
                    .await
                    .map_err(|source| SingleDownloadError::Seek { source })?;
            }
        } else {
            dest.seek(SeekFrom::Start(state.downloaded_size))
                .await
                .map_err(|source| SingleDownloadError::Seek { source })?;
        }

        // truncate file
        dest.set_len(state.downloaded_size)
            .await
            .map_err(|source| SingleDownloadError::Write { source })?;

        Ok(())
    }

    /// (Re)create or advance the per-file progress bar to match the current
    /// download offset, keeping the global bar consistent with it.
    fn sync_progress(&self, progress: &ProgressReporter, state: &mut DownloadState) {
        if state.old_total_size != state.total_size
            || state.old_downloaded_size > state.downloaded_size
            || state.first_request
        {
            // recreate the progress bar if:
            // 1. total size updated
            // 2. offset moved backwards
            // 3. is the first request (the previous bar is a spinner)
            state.first_request = false;
            progress.done();
            progress.bar(&self.download_message(), state.total_size.unwrap_or(0));
            progress.inc(state.downloaded_size);
            if state.old_downloaded_size != state.downloaded_size {
                progress.sub(state.old_downloaded_size);
                progress.add(state.downloaded_size);
            }
        } else if state.old_downloaded_size < state.downloaded_size {
            let new_offset = state.downloaded_size - state.old_downloaded_size;
            progress.inc(new_offset);
            progress.add(new_offset);
        }
    }

    /// Stream the response body into `dest`, decompressing as needed, and
    /// report per-file + global progress. Updates `downloaded_size` and the
    /// reporter's byte count on each chunk.
    async fn copy_body(
        &self,
        resp: reqwest::Response,
        dest: &mut File,
        validator: &mut ChecksumValidator,
        buf: &mut [u8],
        state: &mut DownloadState,
        progress: &mut ProgressReporter,
    ) -> Result<(), SingleDownloadError> {
        let stream = resp
            .bytes_stream()
            .map_err(io::Error::other)
            .into_async_read();
        let mut stream = BufReader::new(stream);

        // initialize decompressor
        let reader: &mut (dyn AsyncRead + Unpin + Send) = match self.entry.file_type {
            CompressType::Xz => &mut XzDecoder::new(&mut stream),
            CompressType::Gzip => &mut GzipDecoder::new(&mut stream),
            CompressType::Bz2 => &mut BzDecoder::new(&mut stream),
            CompressType::Zstd => &mut ZstdDecoder::new(&mut stream),
            CompressType::Lzma => &mut LzmaDecoder::new(&mut stream),
            CompressType::Lz4 => &mut Lz4Decoder::new(&mut stream),
            CompressType::None => &mut stream,
        };

        let stream_counter = AtomicUsize::new(0);
        let counted_reader = Counter::new(reader, &stream_counter);
        let mut reader = counted_reader.compat();

        // copy data
        loop {
            let buf_size = match timeout(self.timeout, reader.read(&mut buf[..])).await {
                Ok(Ok(size)) => size,
                Ok(Err(e)) => return Err(SingleDownloadError::BrokenPipe { source: e }),
                Err(_) => return Err(SingleDownloadError::DownloadTimeout),
            };

            if buf_size == 0 {
                break; // EOF
            }

            dest.write_all(&buf[..buf_size])
                .await
                .map_err(|source| SingleDownloadError::Write { source })?;
            validator.update(&buf[..buf_size]);

            let http_size = stream_counter.swap(0, Ordering::AcqRel);
            let http_size: u64 = http_size.try_into().unwrap();
            state.downloaded_size += http_size;
            progress.inc(http_size);
            progress.add(http_size);
            progress.set_reported(state.downloaded_size);
        }

        Ok(())
    }
}

/// A reusable buffer for streaming the response body.
fn read_buffer() -> Vec<u8> {
    let mut buf = Vec::with_capacity(DOWNLOAD_BUFSIZE);

    #[allow(clippy::uninit_vec)]
    unsafe {
        buf.set_len(DOWNLOAD_BUFSIZE)
    };

    buf
}
