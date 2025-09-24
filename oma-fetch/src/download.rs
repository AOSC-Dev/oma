use crate::{CompressFile, DownloadSource, Event, checksum::ChecksumValidator, send_request};
use std::{
    borrow::Cow,
    io::{self, SeekFrom},
    path::Path,
    time::Duration,
};

use async_compression::futures::bufread::{BzDecoder, GzipDecoder, XzDecoder, ZstdDecoder};
use bon::bon;
use futures::{AsyncRead, TryStreamExt, io::BufReader};
use reqwest::{
    Client, Method, RequestBuilder,
    header::{ACCEPT_RANGES, CONTENT_LENGTH, HeaderValue, RANGE},
};
use snafu::{ResultExt, Snafu};
use tokio::{
    fs::{self, File},
    io::{AsyncBufReadExt as _, AsyncReadExt as _, AsyncSeekExt, AsyncWriteExt},
    time::timeout,
};

use tokio_util::compat::{FuturesAsyncReadCompatExt, TokioAsyncReadCompatExt};
use tracing::{debug, trace};

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
    progress: (usize, usize),
    retry_times: usize,
    msg: Option<Cow<'static, str>>,
    download_list_index: usize,
    file_type: CompressFile,
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
    #[snafu(display("Failed to Remove file"))]
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
        progress: (usize, usize),
        retry_times: usize,
        msg: Option<Cow<'static, str>>,
        download_list_index: usize,
        file_type: CompressFile,
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
            progress,
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
        let mut file_size = file.metadata().ok().map(|x| x.len()).unwrap_or(0);

        trace!("{} Exist file size is: {file_size}", file.display());
        trace!("{} download url is: {}", file.display(), source.url);
        let mut dest = None;
        let mut validator = None;
        let is_symlink = file.is_symlink();

        debug!("file {} is symlink = {}", file.display(), is_symlink);

        if is_symlink {
            tokio::fs::remove_file(&file).await.context(RemoveSnafu)?;
        }

        // 如果要下载的文件已经存在，则验证 Checksum 是否正确，若正确则添加总进度条的进度，并返回
        // 如果不存在，则继续往下走
        if file_exist && !is_symlink {
            trace!(
                "File {} already exists, verifying checksum ...",
                self.entry.filename
            );

            if let Some(hash) = &self.entry.hash {
                trace!("Hash {} exists for the existing file.", hash);

                let mut f = tokio::fs::OpenOptions::new()
                    .write(true)
                    .read(true)
                    .open(&file)
                    .await
                    .context(OpenAsWriteModeSnafu)?;

                trace!("oma opened file {} read/write.", self.entry.filename);

                let mut v = hash.get_validator();

                trace!("Validator created.");

                let (read, finish) = checksum(callback, &mut f, &mut v).await;

                if finish {
                    trace!("Checksum {} matches, cache hit!", self.entry.filename);

                    callback(Event::ProgressDone(self.download_list_index)).await;

                    return Ok(false);
                }

                debug!(
                    "Checksum mismatch, initiating re-download for file {} ...",
                    self.entry.filename
                );

                if !allow_resume {
                    callback(Event::GlobalProgressSub(read)).await;
                } else {
                    dest = Some(f);
                    validator = Some(v);
                }
            }
        }

        let msg = self.progress_msg();
        callback(Event::NewProgressSpinner {
            index: self.download_list_index,
            msg: msg.clone(),
        })
        .await;

        let req = self.build_request_with_basic_auth(&source.url, Method::HEAD, auth);
        let resp_head = timeout(self.timeout, send_request(&source.url, req)).await;

        callback(Event::ProgressDone(self.download_list_index)).await;

        let resp_head = match resp_head {
            Ok(Ok(resp)) => resp,
            Ok(Err(e)) => {
                return Err(SingleDownloadError::ReqwestError { source: e });
            }
            Err(_) => {
                return Err(SingleDownloadError::SendRequestTimeout);
            }
        };

        let head = resp_head.headers();

        // 看看头是否有 ACCEPT_RANGES 这个变量
        // 如果有，而且值不为 none，则可以断点续传
        // 反之，则不能断点续传
        let server_can_resume = match head.get(ACCEPT_RANGES) {
            Some(x) if x == "none" => false,
            Some(_) => true,
            None => false,
        };

        // 从服务器获取文件的总大小
        let total_size = {
            let total_size = head
                .get(CONTENT_LENGTH)
                .map(|x| x.to_owned())
                .unwrap_or(HeaderValue::from(0));

            total_size
                .to_str()
                .ok()
                .and_then(|x| x.parse::<u64>().ok())
                .unwrap_or_default()
        };

        trace!("File total size is: {total_size}");

        let mut req = self.build_request_with_basic_auth(&source.url, Method::GET, auth);

        let mut resume = server_can_resume;

        if !allow_resume {
            resume = false;
        }

        if server_can_resume && allow_resume {
            // 如果已存在的文件大小大于或等于要下载的文件，则重置文件大小，重新下载
            // 因为已经走过一次 chekcusm 了，函数走到这里，则说明肯定文件完整性不对
            if total_size <= file_size {
                trace!(
                    "Resetting size indicator for file to 0, as the file to download is larger that the one that already exists."
                );
                callback(Event::GlobalProgressSub(file_size)).await;
                file_size = 0;
                resume = false;
            }

            // 发送 RANGE 的头，传入的是已经下载的文件的大小
            trace!("oma will set header range as bytes={file_size}-");
            req = req.header(RANGE, format!("bytes={file_size}-"));
        }

        debug!("Can resume = {server_can_resume}, will resume = {resume}",);

        let resp = timeout(self.timeout, req.send()).await;

        callback(Event::ProgressDone(self.download_list_index)).await;

        let resp = match resp {
            Ok(resp) => resp
                .and_then(|resp| resp.error_for_status())
                .context(ReqwestSnafu)?,
            Err(_) => return Err(SingleDownloadError::SendRequestTimeout),
        };

        callback(Event::NewProgressBar {
            index: self.download_list_index,
            msg,
            size: total_size,
        })
        .await;

        let source = resp;

        let hash = &self.entry.hash;

        let mut self_progress = 0;
        let (mut dest, mut validator) = if !resume {
            // 如果不能 resume，则使用创建模式
            trace!(
                "oma will open file {} in creation mode.",
                self.entry.filename
            );

            let f = match File::create(&file).await {
                Ok(f) => f,
                Err(e) => {
                    callback(Event::ProgressDone(self.download_list_index)).await;
                    return Err(SingleDownloadError::Create { source: e });
                }
            };

            if file_size > 0 {
                callback(Event::GlobalProgressSub(file_size)).await;
            }

            if let Err(e) = f.set_len(0).await {
                callback(Event::ProgressDone(self.download_list_index)).await;
                return Err(SingleDownloadError::Create { source: e });
            }

            (f, hash.as_ref().map(|hash| hash.get_validator()))
        } else if let Some((dest, validator)) = dest.zip(validator) {
            callback(Event::ProgressInc {
                index: self.download_list_index,
                size: file_size,
            })
            .await;

            trace!(
                "oma will re-use opened destination file for {}",
                self.entry.filename
            );
            self_progress += file_size;

            (dest, Some(validator))
        } else {
            trace!(
                "oma will open file {} in creation mode.",
                self.entry.filename
            );

            let f = match File::create(&file).await {
                Ok(f) => f,
                Err(e) => {
                    callback(Event::ProgressDone(self.download_list_index)).await;
                    return Err(SingleDownloadError::Create { source: e });
                }
            };

            if let Err(e) = f.set_len(0).await {
                callback(Event::ProgressDone(self.download_list_index)).await;
                return Err(SingleDownloadError::Create { source: e });
            }

            (f, hash.as_ref().map(|hash| hash.get_validator()))
        };

        if server_can_resume && allow_resume {
            // 把文件指针移动到末尾
            trace!("oma will seek to end-of-file for {}", self.entry.filename);
            if let Err(e) = dest.seek(SeekFrom::End(0)).await {
                callback(Event::ProgressDone(self.download_list_index)).await;
                return Err(SingleDownloadError::Seek { source: e });
            }
        }
        // 下载！
        trace!("Starting download!");

        let bytes_stream = source
            .bytes_stream()
            .map_err(io::Error::other)
            .into_async_read();

        let reader: &mut (dyn AsyncRead + Unpin + Send) = match self.file_type {
            CompressFile::Xz => &mut XzDecoder::new(BufReader::new(bytes_stream)),
            CompressFile::Gzip => &mut GzipDecoder::new(BufReader::new(bytes_stream)),
            CompressFile::Bz2 => &mut BzDecoder::new(BufReader::new(bytes_stream)),
            CompressFile::Nothing => &mut BufReader::new(bytes_stream),
            CompressFile::Zstd => &mut ZstdDecoder::new(BufReader::new(bytes_stream)),
        };

        let mut reader = reader.compat();

        let mut buf = vec![0u8; DOWNLOAD_BUFSIZE];

        loop {
            let size = match timeout(self.timeout, reader.read(&mut buf[..])).await {
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

            if size == 0 {
                break;
            }

            if let Err(e) = dest.write_all(&buf[..size]).await {
                callback(Event::ProgressDone(self.download_list_index)).await;
                return Err(SingleDownloadError::Write { source: e });
            }

            callback(Event::ProgressInc {
                index: self.download_list_index,
                size: size as u64,
            })
            .await;

            self_progress += size as u64;

            callback(Event::GlobalProgressAdd(size as u64)).await;

            if let Some(ref mut v) = validator {
                v.update(&buf[..size]);
            }
        }

        // 下载完成，告诉运行时不再写这个文件了
        trace!("Download complete! Shutting down destination file stream ...");
        if let Err(e) = dest.shutdown().await {
            callback(Event::ProgressDone(self.download_list_index)).await;
            return Err(SingleDownloadError::Flush { source: e });
        }

        // 最后看看 checksum 验证是否通过
        if let Some(v) = validator {
            if !v.finish() {
                debug!("Checksum mismatch for file {}", self.entry.filename);
                trace!("{self_progress}");

                callback(Event::GlobalProgressSub(self_progress)).await;
                callback(Event::ProgressDone(self.download_list_index)).await;
                return Err(SingleDownloadError::ChecksumMismatch);
            }

            trace!(
                "Checksum verification successful for file {}",
                self.entry.filename
            );
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

    fn progress_msg(&self) -> String {
        let (count, len) = &self.progress;
        let msg = self.msg.as_deref().unwrap_or(&self.entry.filename);
        let msg = format!("({count}/{len}) {msg}");

        msg
    }

    /// Download local source file
    async fn download_local(
        &self,
        source: &DownloadSource,
        as_symlink: bool,
        callback: &impl AsyncFn(Event),
    ) -> Result<bool, SingleDownloadError> {
        debug!("{:?}", self.entry);
        let msg = self.progress_msg();

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
            msg,
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
            CompressFile::Xz => &mut XzDecoder::new(BufReader::new(from)),
            CompressFile::Gzip => &mut GzipDecoder::new(BufReader::new(from)),
            CompressFile::Bz2 => &mut BzDecoder::new(BufReader::new(from)),
            CompressFile::Nothing => &mut BufReader::new(from),
            CompressFile::Zstd => &mut ZstdDecoder::new(BufReader::new(from)),
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
