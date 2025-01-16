use crate::{CompressFile, DownloadSource, Event};
use std::{
    fs::Permissions,
    future::Future,
    io::{self, ErrorKind, SeekFrom},
    os::unix::fs::PermissionsExt,
    path::Path,
    time::Duration,
};

use async_compression::futures::bufread::{BzDecoder, GzipDecoder, XzDecoder, ZstdDecoder};
use bon::Builder;
use futures::{io::BufReader, AsyncRead, TryStreamExt};
use oma_utils::url_no_escape::url_no_escape;
use reqwest::{
    header::{HeaderValue, ACCEPT_RANGES, CONTENT_LENGTH, RANGE},
    Client, Method, RequestBuilder,
};
use snafu::{ResultExt, Snafu};
use tokio::{
    fs::{self, File},
    io::{AsyncReadExt as _, AsyncSeekExt, AsyncWriteExt},
    time::timeout,
};

use tokio_util::compat::{FuturesAsyncReadCompatExt, TokioAsyncReadCompatExt};
use tracing::debug;

use crate::{DownloadEntry, DownloadSourceType};

#[derive(Snafu, Debug)]
#[snafu(display("source list is empty"))]
pub struct EmptySource {
    file_name: String,
}

#[derive(Builder)]
pub(crate) struct SingleDownloader<'a> {
    client: &'a Client,
    #[builder(with = |entry: &'a DownloadEntry| -> Result<_, EmptySource> {
        if entry.source.is_empty() {
            return Err(EmptySource { file_name: entry.filename.to_string() });
        } else {
            return Ok(entry);
        }
    })]
    pub entry: &'a DownloadEntry,
    progress: (usize, usize),
    retry_times: usize,
    msg: Option<String>,
    download_list_index: usize,
    file_type: CompressFile,
    set_permission: Option<u32>,
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

impl SingleDownloader<'_> {
    pub(crate) async fn try_download<F, Fut>(self, callback: &F) -> DownloadResult
    where
        F: Fn(Event) -> Fut,
        Fut: Future<Output = ()>,
    {
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
    async fn try_http_download<F, Fut>(
        &self,
        source: &DownloadSource,
        auth: &Option<(String, String)>,
        callback: &F,
    ) -> Result<bool, SingleDownloadError>
    where
        F: Fn(Event) -> Fut,
        Fut: Future<Output = ()>,
    {
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

    async fn http_download<F, Fut>(
        &self,
        allow_resume: bool,
        source: &DownloadSource,
        auth: &Option<(String, String)>,
        callback: &F,
    ) -> Result<bool, SingleDownloadError>
    where
        F: Fn(Event) -> Fut,
        Fut: Future<Output = ()>,
    {
        let file = self.entry.dir.join(&*self.entry.filename);
        let file_exist = file.exists();
        let mut file_size = file.metadata().ok().map(|x| x.len()).unwrap_or(0);

        debug!("{} Exist file size is: {file_size}", file.display());
        debug!("{} download url is: {}", file.display(), source.url);
        let mut dest = None;
        let mut validator = None;

        // 如果要下载的文件已经存在，则验证 Checksum 是否正确，若正确则添加总进度条的进度，并返回
        // 如果不存在，则继续往下走
        if file_exist {
            debug!("File: {} exists", self.entry.filename);

            self.set_permission_with_path(&file).await?;

            if let Some(hash) = &self.entry.hash {
                debug!("Hash exist! It is: {}", hash);

                let mut f = tokio::fs::OpenOptions::new()
                    .write(true)
                    .read(true)
                    .open(&file)
                    .await
                    .context(OpenAsWriteModeSnafu)?;

                debug!(
                    "oma opened file: {} with write and read mode",
                    self.entry.filename
                );

                let mut v = hash.get_validator();

                debug!("Validator created.");

                let mut buf = vec![0; 8192];
                let mut read = 0;

                loop {
                    if read == file_size {
                        break;
                    }

                    let Ok(read_count) = f.read(&mut buf[..]).await else {
                        debug!("Read file get get fk, so re-download it");
                        break;
                    };

                    v.update(&buf[..read_count]);

                    callback(Event::GlobalProgressAdd(read_count as u64)).await;

                    read += read_count as u64;
                }

                if v.finish() {
                    debug!(
                        "{} checksum success, no need to download anything.",
                        self.entry.filename
                    );

                    callback(Event::ProgressDone(self.download_list_index)).await;

                    return Ok(false);
                }

                debug!(
                    "checksum fail, will download this file: {}",
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

        let resp_head = timeout(self.timeout, req.send()).await;

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
        let mut can_resume = match head.get(ACCEPT_RANGES) {
            Some(x) if x == "none" => false,
            Some(_) => true,
            None => false,
        };

        debug!("Can resume? {can_resume}");

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

        debug!("File total size is: {total_size}");

        let mut req = self.build_request_with_basic_auth(&source.url, Method::GET, auth);

        if can_resume && allow_resume {
            // 如果已存在的文件大小大于或等于要下载的文件，则重置文件大小，重新下载
            // 因为已经走过一次 chekcusm 了，函数走到这里，则说明肯定文件完整性不对
            if total_size <= file_size {
                debug!("Exist file size is reset to 0, because total size <= exist file size");
                callback(Event::GlobalProgressSub(file_size)).await;
                file_size = 0;
                can_resume = false;
            }

            // 发送 RANGE 的头，传入的是已经下载的文件的大小
            debug!("oma will set header range as bytes={file_size}-");
            req = req.header(RANGE, format!("bytes={}-", file_size));
        }

        debug!("Can resume? {can_resume}");

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

        // 初始化 checksum 验证器
        // 如果文件存在，则 checksum 验证器已经初试过一次，因此进度条加已经验证过的文件大小
        let hash = &self.entry.hash;
        let mut validator = if let Some(v) = validator {
            callback(Event::ProgressInc {
                index: self.download_list_index,
                size: file_size,
            })
            .await;

            Some(v)
        } else {
            hash.as_ref().map(|hash| hash.get_validator())
        };

        let mut self_progress = 0;
        let mut dest = if !can_resume || !allow_resume {
            // 如果不能 resume，则使用创建模式
            debug!(
                "oma will open file: {} as create mode.",
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

            self.set_permission(&f).await?;

            f
        } else if let Some(dest) = dest {
            debug!(
                "oma will re use opened dest file for {}",
                self.entry.filename
            );
            self_progress += file_size;

            dest
        } else {
            debug!(
                "oma will open file: {} as create mode.",
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

            self.set_permission(&f).await?;

            f
        };

        if can_resume && allow_resume {
            // 把文件指针移动到末尾
            debug!("oma will seek file: {} to end", self.entry.filename);
            if let Err(e) = dest.seek(SeekFrom::End(0)).await {
                callback(Event::ProgressDone(self.download_list_index)).await;
                return Err(SingleDownloadError::Seek { source: e });
            }
        }
        // 下载！
        debug!("Start download!");

        let bytes_stream = source
            .bytes_stream()
            .map_err(|e| io::Error::new(ErrorKind::Other, e))
            .into_async_read();

        let reader: &mut (dyn AsyncRead + Unpin + Send) = match self.file_type {
            CompressFile::Xz => &mut XzDecoder::new(BufReader::new(bytes_stream)),
            CompressFile::Gzip => &mut GzipDecoder::new(BufReader::new(bytes_stream)),
            CompressFile::Bz2 => &mut BzDecoder::new(BufReader::new(bytes_stream)),
            CompressFile::Nothing => &mut BufReader::new(bytes_stream),
            CompressFile::Zstd => &mut ZstdDecoder::new(BufReader::new(bytes_stream)),
        };

        let mut reader = reader.compat();

        let mut buf = vec![0u8; 8 * 1024];

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
        debug!("Download complete! shutting down dest file stream ...");
        if let Err(e) = dest.shutdown().await {
            callback(Event::ProgressDone(self.download_list_index)).await;
            return Err(SingleDownloadError::Flush { source: e });
        }

        // 最后看看 checksum 验证是否通过
        if let Some(v) = validator {
            if !v.finish() {
                debug!("checksum fail: {}", self.entry.filename);
                debug!("{self_progress}");

                callback(Event::GlobalProgressSub(self_progress)).await;
                callback(Event::ProgressDone(self.download_list_index)).await;
                return Err(SingleDownloadError::ChecksumMismatch);
            }

            debug!("checksum success: {}", self.entry.filename);
        }

        callback(Event::ProgressDone(self.download_list_index)).await;

        Ok(true)
    }

    async fn set_permission(&self, f: &File) -> Result<(), SingleDownloadError> {
        if let Some(mode) = self.set_permission {
            debug!("Setting {} permission to {:#o}", self.entry.filename, mode);
            f.set_permissions(Permissions::from_mode(mode))
                .await
                .context(SetPermissionSnafu)?;
        }

        Ok(())
    }

    async fn set_permission_with_path(&self, path: &Path) -> Result<(), SingleDownloadError> {
        if let Some(mode) = self.set_permission {
            debug!("Setting {} permission to {:#o}", self.entry.filename, mode);

            fs::set_permissions(path, Permissions::from_mode(mode))
                .await
                .context(SetPermissionSnafu)?;
        }

        Ok(())
    }

    fn build_request_with_basic_auth(
        &self,
        url: &str,
        method: Method,
        auth: &Option<(String, String)>,
    ) -> RequestBuilder {
        let mut req = self.client.request(method, url);

        if let Some((user, password)) = auth {
            debug!("auth user: {}", user);
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
    async fn download_local<F, Fut>(
        &self,
        source: &DownloadSource,
        as_symlink: bool,
        callback: &F,
    ) -> Result<bool, SingleDownloadError>
    where
        F: Fn(Event) -> Fut,
        Fut: Future<Output = ()>,
    {
        debug!("{:?}", self.entry);
        let msg = self.progress_msg();

        let url = &source.url;

        // 传入的参数不对，应该 panic
        let url_path = url_no_escape(url.strip_prefix("file:").unwrap());

        let url_path = Path::new(&url_path);

        let total_size = tokio::fs::metadata(url_path)
            .await
            .context(OpenSnafu)?
            .len();

        callback(Event::NewProgressBar {
            index: self.download_list_index,
            msg,
            size: total_size,
        })
        .await;

        if as_symlink {
            let symlink = self.entry.dir.join(&*self.entry.filename);
            if symlink.exists() {
                tokio::fs::remove_file(&symlink)
                    .await
                    .context(RemoveSnafu)?;
            }

            tokio::fs::symlink(url_path, symlink)
                .await
                .context(CreateSymlinkSnafu)?;

            callback(Event::GlobalProgressAdd(total_size as u64)).await;
            callback(Event::ProgressDone(self.download_list_index)).await;

            return Ok(true);
        }

        debug!("File path is: {}", url_path.display());

        let from = File::open(&url_path).await.context(CreateSnafu)?;
        let from = tokio::io::BufReader::new(from).compat();

        debug!("Success open file: {}", url_path.display());

        let mut to = File::create(self.entry.dir.join(&*self.entry.filename))
            .await
            .context(CreateSnafu)?;

        self.set_permission(&to).await?;

        let reader: &mut (dyn AsyncRead + Unpin + Send) = match self.file_type {
            CompressFile::Xz => &mut XzDecoder::new(BufReader::new(from)),
            CompressFile::Gzip => &mut GzipDecoder::new(BufReader::new(from)),
            CompressFile::Bz2 => &mut BzDecoder::new(BufReader::new(from)),
            CompressFile::Nothing => &mut BufReader::new(from),
            CompressFile::Zstd => &mut ZstdDecoder::new(BufReader::new(from)),
        };

        let mut reader = reader.compat();

        debug!(
            "Success create file: {}",
            self.entry.dir.join(&*self.entry.filename).display()
        );

        let mut buf = vec![0u8; 8 * 1024];

        loop {
            let size = reader.read(&mut buf[..]).await.context(BrokenPipeSnafu)?;

            if size == 0 {
                break;
            }

            to.write_all(&buf[..size]).await.context(WriteSnafu)?;

            callback(Event::ProgressInc {
                index: self.download_list_index,
                size: size as u64,
            })
            .await;

            callback(Event::GlobalProgressAdd(size as u64)).await;
        }

        callback(Event::ProgressDone(self.download_list_index)).await;

        Ok(true)
    }
}
