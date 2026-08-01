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
    progress::{DownloadState, ProgressReporter},
    verify,
};

/// Outcome of sending one HTTP request.
pub(crate) enum RequestOutcome {
    /// Response received, proceed to the next phase.
    Ready(reqwest::Response),
    /// The server rejected our range; restart the download loop.
    Restart,
    /// Fatal request error.
    Fatal(SingleDownloadError),
}

/// Outcome of reconciling a response with our resume offset.
#[derive(Debug, PartialEq, Eq)]
pub(crate) enum ResumeOutcome {
    /// Response accepted, continue to the next phase.
    Proceed,
    /// The server didn't honor our range; restart the download loop.
    Restart,
}

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
            // A fresh reporter per attempt: on success `finish()` clears the
            // per-file bar; on error `Drop` also undoes the bytes reported to
            // the global bar, so a retry starts from a clean bar.
            let mut progress = ProgressReporter::new(tx, self.download_list_index, self.total);

            match self
                .http_download(allow_resume, source, &mut progress)
                .await
            {
                Ok(s) => {
                    progress.finish();
                    return Ok(s);
                }
                Err(e) => {
                    // The reporter is dropped: it clears the per-file bar and
                    // undoes this attempt's global progress contribution, so
                    // the retry (or the next source) starts from a clean bar.
                    drop(progress);
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

    /// Inner download attempt. The caller owns the [`ProgressReporter`]; on
    /// error the reporter's `Drop` clears the per-file bar and undoes the
    /// bytes this attempt already added to the global progress bar, before
    /// retrying or trying the next source.
    async fn http_download(
        &self,
        allow_resume: bool,
        source: &DownloadSource,
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
                .prepare_existing_file(&file, progress, &mut state, &mut validator)
                .await?
            {
                return Ok(false);
            }

            // Seeding checksummed an existing file, so those bytes are already
            // on the global bar (added by the checksum helper). Tell the
            // reporter so failure undo and bar recreation account for them
            // exactly.
            progress.set_position(state.prev_size);

            progress.start_indeterminate(&self.download_message());

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
        progress: &ProgressReporter,
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

            let (read, finish) =
                verify::checksum(progress, &mut f, validator).await;

            if finish {
                trace!("checksum of {} matches, cache hit!", self.entry.filename);
                return Ok(true);
            }

            debug!(
                "checksum mismatch, initiating re-download for file {} ...",
                self.entry.filename
            );
            state.prev_size = read;
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
            progress.update(
                &self.download_message(),
                state.downloaded_size,
                state.total_size,
            );

            // 5. stream the body into the file (decompressing as needed)
            self.copy_body(resp, dest, validator, &mut buf, state, progress)
                .await?;

            debug!(
                "downloaded {} bytes",
                state.downloaded_size - state.prev_size
            );

            if state.downloaded_size == state.prev_size {
                // this should not happen ...
                break 'download;
            }

            if is_complete || state.total_size.is_none() {
                // total size is unknown, we have to assume that the body is complete
                break 'download;
            }

            state.prev_size = state.downloaded_size;
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
        if state.downloaded_size != state.prev_size {
            assert!(state.downloaded_size == 0 || self.entry.file_type == CompressType::None);
            debug!(
                "moving writer from {} to {}",
                state.prev_size, state.downloaded_size
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
        let msg = self.download_message();

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
                Ok(Err(e)) => return Err(SingleDownloadError::Read { source: e }),
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
            progress.update(&msg, state.downloaded_size, state.total_size);
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        DownloadEntry, DownloadSourceType,
        checksum::Checksum,
        download::{DownloadResult, test_support},
        test_support::TempDir,
    };
    use axum::{
        Router,
        body::{Body, Bytes},
        extract::State,
        http::{HeaderMap, StatusCode, header},
        response::{IntoResponse, Response},
        routing::get,
    };
    use futures::StreamExt as _;
    use std::sync::Arc;
    use std::time::Duration;

    fn sha256(data: &[u8]) -> Vec<u8> {
        use sha2::{Digest, Sha256};
        let mut hasher = Sha256::new();
        hasher.update(data);
        hasher.finalize().to_vec()
    }

    /// Build an entry that downloads `url` to `dir/out/out.bin`.
    fn http_entry(dir: &Path, url: &str, hash: Option<Checksum>) -> DownloadEntry {
        DownloadEntry {
            source: vec![DownloadSource {
                url: url.to_string(),
                source_type: DownloadSourceType::Http,
            }],
            filename: "out.bin".to_string(),
            dir: dir.join("out"),
            hash,
            ..Default::default()
        }
    }

    /// Run a downloader, bounded by a timeout so a broken server can't hang
    /// the test suite, and collect every progress event it emitted.
    async fn run_downloader(downloader: SingleDownloader) -> (DownloadResult, Vec<Event>) {
        let (tx, rx) = flume::unbounded::<Event>();
        let result = tokio::time::timeout(Duration::from_secs(20), downloader.try_download(&tx))
            .await
            .expect("download timed out");
        let events: Vec<Event> = rx.drain().collect();
        (result, events)
    }

    /// Run `try_download` for one entry, discarding progress events.
    async fn run_download(entry: DownloadEntry) -> DownloadResult {
        run_downloader(test_support::downloader(entry)).await.0
    }

    /// Serve `app` on an ephemeral port and return its `http://.../pkg` URL.
    async fn serve(app: Router) -> String {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        tokio::spawn(async move {
            axum::serve(listener, app).await.expect("axum server");
        });
        format!("http://{addr}/pkg")
    }

    /// One response plan for [`behavior_handler`].
    #[derive(Clone, Copy)]
    enum Behavior {
        /// Full 200 response carrying `body`.
        Ok(&'static [u8]),
        /// 200 with `Content-Length: full_len`, sending only `partial` bytes
        /// and then stalling, so the client hits its read timeout.
        Stall {
            full_len: usize,
            partial: &'static [u8],
        },
    }

    /// Serves each [`Behavior`] in order across consecutive requests.
    #[derive(Clone)]
    struct BehaviorState {
        behaviors: Arc<Vec<Behavior>>,
        next: Arc<AtomicUsize>,
    }

    async fn behavior_handler(State(state): State<BehaviorState>) -> Response {
        let index = state.next.fetch_add(1, Ordering::SeqCst);
        match state.behaviors[index % state.behaviors.len()] {
            Behavior::Ok(body) => body.into_response(),
            Behavior::Stall { full_len, partial } => {
                // Yield one chunk, then never complete, so the client hits its
                // read timeout and retries on a fresh connection.
                let stream = futures::stream::iter([Ok::<Bytes, std::io::Error>(
                    Bytes::from_static(partial),
                )])
                .chain(futures::stream::pending::<Result<Bytes, std::io::Error>>());

                axum::http::Response::builder()
                    .status(StatusCode::OK)
                    .header(header::CONTENT_LENGTH, full_len)
                    .body(Body::from_stream(stream))
                    .unwrap()
                    .into_response()
            }
        }
    }

    /// The body served by [`range_handler`].
    const RESUME_BODY: &[u8] = b"0123456789abcdefghijklmnopqrstuvwxyz";

    /// Honors `Range: bytes=N-` with a 206 tail response.
    async fn range_handler(headers: HeaderMap) -> Response {
        let start = headers
            .get(header::RANGE)
            .and_then(|v| v.to_str().ok())
            .and_then(|v| v.strip_prefix("bytes="))
            .and_then(|v| v.split('-').next())
            .and_then(|v| v.parse::<usize>().ok())
            .unwrap_or(0);

        if start == 0 {
            return RESUME_BODY.into_response();
        }

        axum::http::Response::builder()
            .status(StatusCode::PARTIAL_CONTENT)
            .header(
                header::CONTENT_RANGE,
                format!(
                    "bytes {start}-{}/{}",
                    RESUME_BODY.len() - 1,
                    RESUME_BODY.len()
                ),
            )
            .body(Body::from(&RESUME_BODY[start..]))
            .unwrap()
            .into_response()
    }

    /// Refuses any range request with 416; otherwise serves [`RESUME_BODY`].
    async fn range_416_handler(headers: HeaderMap) -> Response {
        if headers.contains_key(header::RANGE) {
            return StatusCode::RANGE_NOT_SATISFIABLE.into_response();
        }
        RESUME_BODY.into_response()
    }

    /// Build an `axum::http::Response`-backed [`reqwest::Response`] for
    /// exercising `negotiate_resume` without a real server.
    fn response(status: StatusCode, headers: &[(&str, &str)], body: &[u8]) -> reqwest::Response {
        let mut builder = axum::http::Response::builder().status(status);
        for (name, value) in headers {
            builder = builder.header(*name, *value);
        }

        builder
            .body(reqwest::Body::from(body.to_vec()))
            .unwrap()
            .into()
    }

    #[tokio::test]
    async fn downloads_over_http_with_checksum() {
        let dir = TempDir::new("http-download");
        let data: &'static [u8] = b"hello from the http server";
        let url = serve(Router::new().route("/pkg", get(move || async move { data }))).await;
        let entry = http_entry(dir.path(), &url, Some(Checksum::Sha256(sha256(data))));

        let result = run_download(entry).await;
        match result {
            DownloadResult::Success(summary) => {
                assert!(summary.wrote);
                assert_eq!(
                    tokio::fs::read(dir.path().join("out/out.bin"))
                        .await
                        .unwrap(),
                    data
                );
            }
            DownloadResult::Failed { file_name } => {
                panic!("expected success, got failed: {file_name}")
            }
        }
    }

    #[tokio::test]
    async fn fails_on_checksum_mismatch_over_http() {
        let dir = TempDir::new("http-mismatch");
        let data: &'static [u8] = b"body that does not match";
        let url = serve(Router::new().route("/pkg", get(move || async move { data }))).await;
        let entry = http_entry(dir.path(), &url, Some(Checksum::Sha256(vec![0; 32])));

        let result = run_download(entry).await;
        assert!(matches!(result, DownloadResult::Failed { .. }));
    }

    #[tokio::test]
    async fn fails_on_http_error_status() {
        let dir = TempDir::new("http-404");
        let url = serve(Router::new().route("/pkg", get(|| async { StatusCode::NOT_FOUND }))).await;
        let entry = http_entry(dir.path(), &url, None);

        let result = run_download(entry).await;
        assert!(matches!(result, DownloadResult::Failed { .. }));
    }

    #[tokio::test]
    async fn resumes_from_partial_file() {
        let dir = TempDir::new("http-resume");
        let full = RESUME_BODY;
        let url = serve(Router::new().route("/pkg", get(range_handler))).await;

        // simulate an interrupted download: the first 10 bytes are present
        let out_dir = dir.path().join("out");
        tokio::fs::create_dir_all(&out_dir).await.unwrap();
        tokio::fs::write(out_dir.join("out.bin"), &full[..10])
            .await
            .unwrap();

        let entry = DownloadEntry {
            source: vec![DownloadSource {
                url,
                source_type: DownloadSourceType::Http,
            }],
            filename: "out.bin".to_string(),
            dir: out_dir.clone(),
            hash: Some(Checksum::Sha256(sha256(full))),
            allow_resume: true,
            ..Default::default()
        };

        let result = run_download(entry).await;
        assert!(matches!(result, DownloadResult::Success(_)));
        assert_eq!(
            tokio::fs::read(out_dir.join("out.bin")).await.unwrap(),
            full
        );
    }

    #[tokio::test]
    async fn retries_after_checksum_mismatch_and_undoes_progress() {
        let dir = TempDir::new("http-retry-mismatch");
        let correct: &'static [u8] = b"the correct body";
        let url = {
            let state = BehaviorState {
                behaviors: Arc::new(vec![
                    Behavior::Ok(b"wrong body one"),
                    Behavior::Ok(b"wrong body two"),
                    Behavior::Ok(correct),
                ]),
                next: Arc::new(AtomicUsize::new(0)),
            };
            serve(
                Router::new()
                    .route("/pkg", get(behavior_handler))
                    .with_state(state),
            )
            .await
        };

        let entry = http_entry(dir.path(), &url, Some(Checksum::Sha256(sha256(correct))));
        let downloader = test_support::downloader_with(entry, 3, Duration::from_secs(30));

        let (result, events) = run_downloader(downloader).await;
        assert!(matches!(result, DownloadResult::Success(_)));
        assert_eq!(
            tokio::fs::read(dir.path().join("out/out.bin"))
                .await
                .unwrap(),
            correct
        );

        // both failed attempts were undone on the global progress bar
        assert!(
            events
                .iter()
                .any(|e| matches!(e, Event::Cleared { sub, .. } if *sub > 0))
        );
        // the second mismatch is reported to the UI as a retry notice
        assert!(
            events
                .iter()
                .any(|e| matches!(e, Event::ChecksumMismatch { times: 2, .. }))
        );
    }

    #[tokio::test]
    async fn retries_after_timeout_and_undoes_progress() {
        let dir = TempDir::new("http-retry-timeout");
        let full: &'static [u8] = b"the body that eventually arrives";
        let url = {
            let state = BehaviorState {
                behaviors: Arc::new(vec![
                    Behavior::Stall {
                        full_len: full.len(),
                        partial: b"12345",
                    },
                    Behavior::Stall {
                        full_len: full.len(),
                        partial: b"67890",
                    },
                    Behavior::Ok(full),
                ]),
                next: Arc::new(AtomicUsize::new(0)),
            };
            serve(
                Router::new()
                    .route("/pkg", get(behavior_handler))
                    .with_state(state),
            )
            .await
        };

        let entry = http_entry(dir.path(), &url, Some(Checksum::Sha256(sha256(full))));
        // No connection pooling: the stalled connection must not block the
        // retry. 400ms per attempt is short enough to keep the test fast.
        let downloader = test_support::downloader_with_client(
            test_support::client_no_pool(),
            entry,
            3,
            Duration::from_millis(400),
        );

        let (result, events) = run_downloader(downloader).await;
        assert!(matches!(result, DownloadResult::Success(_)));
        assert_eq!(
            tokio::fs::read(dir.path().join("out/out.bin"))
                .await
                .unwrap(),
            full
        );

        // the stalled attempts' bytes were undone on the global progress bar
        assert!(
            events
                .iter()
                .any(|e| matches!(e, Event::Cleared { sub, .. } if *sub > 0))
        );
        // the second timeout is reported to the UI as a retry notice
        assert!(
            events
                .iter()
                .any(|e| matches!(e, Event::Timeout { times: 2, .. }))
        );
    }

    #[tokio::test]
    async fn restarts_when_server_rejects_range() {
        let dir = TempDir::new("http-416");
        let full = RESUME_BODY;

        // The server refuses any range request with 416, so the client must
        // restart the loop and re-fetch the whole body without a Range header.
        let url = serve(Router::new().route("/pkg", get(range_416_handler))).await;

        // simulate an interrupted download: the first 10 bytes are present
        let out_dir = dir.path().join("out");
        tokio::fs::create_dir_all(&out_dir).await.unwrap();
        tokio::fs::write(out_dir.join("out.bin"), &full[..10])
            .await
            .unwrap();

        let entry = DownloadEntry {
            source: vec![DownloadSource {
                url,
                source_type: DownloadSourceType::Http,
            }],
            filename: "out.bin".to_string(),
            dir: out_dir.clone(),
            hash: Some(Checksum::Sha256(sha256(full))),
            allow_resume: true,
            ..Default::default()
        };

        let result = run_download(entry).await;
        assert!(matches!(result, DownloadResult::Success(_)));
        assert_eq!(
            tokio::fs::read(out_dir.join("out.bin")).await.unwrap(),
            full
        );
    }

    #[tokio::test]
    async fn falls_back_to_next_source() {
        let dir = TempDir::new("http-fallback");
        let correct: &'static [u8] = b"served by the http source";
        let hash = Checksum::Sha256(sha256(correct));

        // A local source whose content does not match the expected checksum:
        // it fails, and the download falls back to the http source.
        let local_src = dir.path().join("local.bin");
        std::fs::write(&local_src, b"wrong local content").unwrap();

        let url = serve(Router::new().route("/pkg", get(move || async move { correct }))).await;

        let entry = DownloadEntry {
            source: vec![
                DownloadSource {
                    url: format!("file://{}", local_src.display()),
                    source_type: DownloadSourceType::Local(false),
                },
                DownloadSource {
                    url,
                    source_type: DownloadSourceType::Http,
                },
            ],
            filename: "out.bin".to_string(),
            dir: dir.path().join("out"),
            hash: Some(hash),
            ..Default::default()
        };

        let (result, events) = run_downloader(test_support::downloader(entry)).await;
        assert!(matches!(result, DownloadResult::Success(_)));
        assert_eq!(
            tokio::fs::read(dir.path().join("out/out.bin"))
                .await
                .unwrap(),
            correct
        );
        // the failed local source was reported as a fallback notice
        assert!(events.iter().any(|e| matches!(e, Event::NextUrl { .. })));
    }

    #[tokio::test]
    async fn fails_on_send_request_timeout_without_retry() {
        let dir = TempDir::new("http-send-timeout");
        let accepted = Arc::new(AtomicUsize::new(0));

        // A server that accepts connections but never writes a response, so
        // the client's request send times out.
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let url = format!("http://{}/pkg", listener.local_addr().unwrap());
        let accepted_in_server = accepted.clone();
        tokio::spawn(async move {
            loop {
                let Ok(sock) = listener.accept().await else {
                    break;
                };
                accepted_in_server.fetch_add(1, Ordering::SeqCst);
                tokio::spawn(async move {
                    // hold the connection open without responding, so the
                    // client's send hangs until its timeout fires
                    let _socket = sock;
                    tokio::time::sleep(Duration::from_secs(10)).await;
                });
            }
        });

        let entry = http_entry(dir.path(), &url, None);
        // SendRequestTimeout is fatal (not retried): even with a retry budget,
        // the download fails after a single attempt.
        let downloader = test_support::downloader_with(entry, 3, Duration::from_millis(300));

        let (result, events) = run_downloader(downloader).await;
        assert!(matches!(result, DownloadResult::Failed { .. }));
        assert!(events.iter().any(|e| matches!(
            e,
            Event::Failed {
                error: SingleDownloadError::SendRequestTimeout,
                ..
            }
        )));
        // exactly one connection: the fatal error was not retried
        assert_eq!(accepted.load(Ordering::SeqCst), 1);
    }

    #[test]
    fn negotiate_resume_accepts_matching_partial() {
        let mut state = DownloadState::new();
        state.downloaded_size = 100;
        let resp = response(
            StatusCode::PARTIAL_CONTENT,
            &[("content-range", "bytes 100-199/200")],
            b"",
        );
        assert_eq!(
            SingleDownloader::negotiate_resume(&resp, &mut state),
            ResumeOutcome::Proceed
        );
        assert_eq!(state.total_size, Some(200));
    }

    #[test]
    fn negotiate_resume_restarts_on_range_mismatch() {
        let mut state = DownloadState::new();
        state.downloaded_size = 100;
        let resp = response(
            StatusCode::PARTIAL_CONTENT,
            &[("content-range", "bytes 0-99/200")],
            b"",
        );
        assert_eq!(
            SingleDownloader::negotiate_resume(&resp, &mut state),
            ResumeOutcome::Restart
        );
        assert_eq!(state.downloaded_size, 0);
    }

    #[test]
    fn negotiate_resume_restarts_without_content_range() {
        let mut state = DownloadState::new();
        state.downloaded_size = 100;
        let resp = response(StatusCode::PARTIAL_CONTENT, &[], b"");
        assert_eq!(
            SingleDownloader::negotiate_resume(&resp, &mut state),
            ResumeOutcome::Restart
        );
    }

    #[test]
    fn negotiate_resume_reads_content_length_from_ok() {
        let mut state = DownloadState::new();
        let resp = response(StatusCode::OK, &[("content-length", "42")], b"");
        assert_eq!(
            SingleDownloader::negotiate_resume(&resp, &mut state),
            ResumeOutcome::Proceed
        );
        assert_eq!(state.total_size, Some(42));
    }

    #[test]
    fn negotiate_resume_restarts_when_range_not_honored() {
        let mut state = DownloadState::new();
        state.downloaded_size = 50;
        let resp = response(StatusCode::OK, &[("content-length", "100")], b"");
        assert_eq!(
            SingleDownloader::negotiate_resume(&resp, &mut state),
            ResumeOutcome::Proceed
        );
        assert_eq!(state.downloaded_size, 0);
    }
}
