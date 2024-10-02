use crate::{CompressFile, DownloadProgressControl, DownloadSource};
use std::{
    io::{self, ErrorKind, SeekFrom},
    path::Path,
    sync::{
        atomic::{AtomicU64, Ordering},
        Arc,
    },
};

use async_compression::futures::bufread::{BzDecoder, GzipDecoder, XzDecoder, ZstdDecoder};
use bon::Builder;
use futures::{io::BufReader, AsyncBufRead, AsyncRead, TryStreamExt};
use oma_utils::url_no_escape::url_no_escape;
use reqwest::{
    header::{HeaderValue, ACCEPT_RANGES, CONTENT_LENGTH, RANGE},
    Client,
};
use tokio::io::{AsyncReadExt as _, AsyncSeekExt, AsyncWriteExt};

use tokio_util::compat::{FuturesAsyncReadCompatExt, TokioAsyncReadCompatExt};
use tracing::debug;

use crate::{DownloadEntry, DownloadError, DownloadResult, DownloadSourceType, Summary};

#[derive(Debug, Builder)]
pub(crate) struct SingleDownloader<'a> {
    client: &'a Client,
    pub entry: &'a DownloadEntry,
    progress: (usize, usize, Option<String>),
    retry_times: usize,
    context: Option<String>,
    download_list_index: usize,
    file_type: CompressFile,
}

impl SingleDownloader<'_> {
    pub(crate) async fn try_download(
        self,
        global_progress: Arc<AtomicU64>,
        progress_manager: &dyn DownloadProgressControl,
    ) -> DownloadResult<Summary> {
        let mut sources = self.entry.source.clone();
        sources.sort_unstable_by(|a, b| b.source_type.cmp(&a.source_type));

        let gpc = global_progress.clone();
        let msg = self.progress.2.as_deref().unwrap_or(&*self.entry.filename);

        for (i, c) in sources.iter().enumerate() {
            let download_res = match c.source_type {
                DownloadSourceType::Http => {
                    self.try_http_download(progress_manager, gpc.clone(), c)
                        .await
                }
                DownloadSourceType::Local(as_symlink) => {
                    self.download_local(progress_manager, gpc.clone(), c, as_symlink)
                        .await
                }
            };

            match download_res {
                Ok(download_res) => {
                    progress_manager.download_done(self.download_list_index, msg);
                    return Ok(download_res);
                }
                Err(e) => {
                    if i == sources.len() - 1 {
                        return Err(e);
                    }
                    debug!("{c:?} download failed {e}, trying next url.");
                    progress_manager
                        .failed_to_get_source_next_url(self.download_list_index, &e.to_string());
                }
            }
        }

        Err(DownloadError::EmptySources)
    }

    /// Download file with retry (http)
    async fn try_http_download(
        &self,
        progress_manager: &dyn DownloadProgressControl,
        global_progress: Arc<AtomicU64>,
        source: &DownloadSource,
    ) -> DownloadResult<Summary> {
        let mut times = 1;
        let mut allow_resume = self.entry.allow_resume;
        loop {
            match self
                .http_download(
                    progress_manager,
                    global_progress.clone(),
                    allow_resume,
                    source,
                )
                .await
            {
                Ok(s) => {
                    return Ok(s);
                }
                Err(e) => match e {
                    DownloadError::ChecksumMismatch(ref filename) => {
                        if self.retry_times == times {
                            return Err(e);
                        }

                        if times > 1 {
                            progress_manager.checksum_mismatch_retry(
                                self.download_list_index,
                                filename,
                                times,
                            );
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
        progress_manager: &dyn DownloadProgressControl,
        global_progress: Arc<AtomicU64>,
        allow_resume: bool,
        source: &DownloadSource,
    ) -> DownloadResult<Summary> {
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

            if let Some(hash) = &self.entry.hash {
                debug!("Hash exist! It is: {}", hash);

                let mut f = tokio::fs::OpenOptions::new()
                    .write(true)
                    .read(true)
                    .open(&file)
                    .await
                    .map_err(|e| DownloadError::IOError(self.entry.filename.to_string(), e))?;

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

                    let read_count = f
                        .read(&mut buf[..])
                        .await
                        .map_err(|e| DownloadError::IOError(self.entry.filename.to_string(), e))?;

                    v.update(&buf[..read_count]);

                    global_progress.fetch_add(read_count as u64, Ordering::SeqCst);
                    progress_manager.global_progress_set(&global_progress);

                    read += read_count as u64;
                }

                if v.finish() {
                    debug!(
                        "{} checksum success, no need to download anything.",
                        self.entry.filename
                    );

                    progress_manager.progress_done(self.download_list_index);

                    return Ok(Summary {
                        filename: self.entry.filename.clone(),
                        wrote: false,
                        count: self.download_list_index,
                        context: self.context.clone(),
                    });
                }

                debug!(
                    "checksum fail, will download this file: {}",
                    self.entry.filename
                );

                if !allow_resume {
                    global_progress.fetch_sub(read, Ordering::SeqCst);
                    progress_manager.global_progress_set(&global_progress);
                } else {
                    dest = Some(f);
                    validator = Some(v);
                }
            }
        }

        let msg = self.set_progress_msg();
        progress_manager.new_progress_spinner(self.download_list_index, &msg);

        let resp_head = match self.client.head(&source.url).send().await {
            Ok(resp) => resp,
            Err(e) => {
                progress_manager.progress_done(self.download_list_index);
                return Err(DownloadError::ReqwestError(e));
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

        let mut req = self.client.get(&source.url);

        if can_resume && allow_resume {
            // 如果已存在的文件大小大于或等于要下载的文件，则重置文件大小，重新下载
            // 因为已经走过一次 chekcusm 了，函数走到这里，则说明肯定文件完整性不对
            if total_size <= file_size {
                debug!("Exist file size is reset to 0, because total size <= exist file size");
                let gpc = global_progress.as_ref();
                let gp = gpc.load(Ordering::SeqCst);
                global_progress.store(gp.saturating_sub(file_size), Ordering::SeqCst);
                progress_manager.global_progress_set(&global_progress);
                file_size = 0;
                can_resume = false;
            }

            // 发送 RANGE 的头，传入的是已经下载的文件的大小
            debug!("oma will set header range as bytes={file_size}-");
            req = req.header(RANGE, format!("bytes={}-", file_size));
        }

        debug!("Can resume? {can_resume}");

        let resp = req.send().await.map_err(DownloadError::ReqwestError)?;

        if let Err(e) = resp.error_for_status_ref() {
            progress_manager.progress_done(self.download_list_index);
            return Err(DownloadError::ReqwestError(e));
        } else {
            progress_manager.progress_done(self.download_list_index);
        }

        progress_manager.new_progress_bar(self.download_list_index, &msg, total_size);

        let source = resp;

        // 初始化 checksum 验证器
        // 如果文件存在，则 checksum 验证器已经初试过一次，因此进度条加已经验证过的文件大小
        let hash = &self.entry.hash;
        let mut validator = if let Some(v) = validator {
            progress_manager.progress_inc(self.download_list_index, file_size);
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

            let f = match tokio::fs::File::create(&file).await {
                Ok(f) => f,
                Err(e) => {
                    progress_manager.progress_done(self.download_list_index);
                    return Err(DownloadError::IOError(self.entry.filename.to_string(), e));
                }
            };

            if let Err(e) = f.set_len(0).await {
                progress_manager.progress_done(self.download_list_index);
                return Err(DownloadError::IOError(self.entry.filename.to_string(), e));
            }

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

            let f = match tokio::fs::File::create(&file).await {
                Ok(f) => f,
                Err(e) => {
                    progress_manager.progress_done(self.download_list_index);
                    return Err(DownloadError::IOError(self.entry.filename.to_string(), e));
                }
            };

            if let Err(e) = f.set_len(0).await {
                progress_manager.progress_done(self.download_list_index);
                return Err(DownloadError::IOError(self.entry.filename.to_string(), e));
            }

            f
        };

        if can_resume && allow_resume {
            // 把文件指针移动到末尾
            debug!("oma will seek file: {} to end", self.entry.filename);
            if let Err(e) = dest.seek(SeekFrom::End(0)).await {
                progress_manager.progress_done(self.download_list_index);
                return Err(DownloadError::IOError(self.entry.filename.to_string(), e));
            }
        }
        // 下载！
        debug!("Start download!");

        let bytes_stream = source
            .bytes_stream()
            .map_err(|e| io::Error::new(ErrorKind::Other, e))
            .into_async_read();

        let mut reader = match self.file_type {
            CompressFile::Xz => CompressStream::Xz(XzDecoder::new(BufReader::new(bytes_stream))),
            CompressFile::Gzip => {
                CompressStream::Gzip(GzipDecoder::new(BufReader::new(bytes_stream)))
            }
            CompressFile::Bz2 => CompressStream::Bz2(BzDecoder::new(BufReader::new(bytes_stream))),
            CompressFile::Nothing => CompressStream::Nothing(BufReader::new(bytes_stream)),
            CompressFile::Zstd => {
                CompressStream::Zstd(ZstdDecoder::new(BufReader::new(bytes_stream)))
            }
        };
        let reader = reader.as_inner_mut();
        let mut reader = reader.compat();

        let mut buf = vec![0u8; 8 * 1024];

        loop {
            let size = reader.read(&mut buf[..]).await.map_err(|e| {
                progress_manager.progress_done(self.download_list_index);
                DownloadError::IOError(self.entry.filename.to_string(), e)
            })?;

            if size == 0 {
                break;
            }

            dest.write_all(&buf[..size]).await.map_err(|e| {
                progress_manager.progress_done(self.download_list_index);
                DownloadError::IOError(self.entry.filename.to_string(), e)
            })?;

            progress_manager.progress_inc(self.download_list_index, size as u64);

            self_progress += size as u64;

            global_progress.fetch_add(size as u64, Ordering::SeqCst);
            progress_manager.global_progress_set(&global_progress);

            if let Some(ref mut v) = validator {
                v.update(&buf[..size]);
            }
        }

        // 下载完成，告诉内核不再写这个文件了
        debug!("Download complete! shutting down dest file stream ...");
        if let Err(e) = dest.shutdown().await {
            progress_manager.progress_done(self.download_list_index);
            return Err(DownloadError::IOError(self.entry.filename.to_string(), e));
        }

        // 最后看看 chekcsum 验证是否通过
        if let Some(v) = validator {
            if !v.finish() {
                debug!("checksum fail: {}", self.entry.filename);
                debug!("{global_progress:?}");
                debug!("{self_progress}");

                global_progress.fetch_sub(self_progress, Ordering::SeqCst);

                progress_manager.global_progress_set(&global_progress);
                progress_manager.progress_done(self.download_list_index);
                return Err(DownloadError::ChecksumMismatch(
                    self.entry.filename.to_string(),
                ));
            }

            debug!("checksum success: {}", self.entry.filename);
        }

        progress_manager.progress_done(self.download_list_index);

        Ok(Summary {
            filename: self.entry.filename.clone(),
            wrote: true,
            count: self.download_list_index,
            context: self.context.clone(),
        })
    }

    fn set_progress_msg(&self) -> String {
        let (count, len, msg) = &self.progress;
        let msg = msg.as_deref().unwrap_or(&self.entry.filename);
        let msg = format!("({count}/{len}) {msg}");

        msg
    }

    /// Download local source file
    async fn download_local(
        &self,
        progress_manager: &dyn DownloadProgressControl,
        global_progress: Arc<AtomicU64>,
        source: &DownloadSource,
        as_symlink: bool,
    ) -> DownloadResult<Summary> {
        debug!("{:?}", self.entry);
        let msg = self.set_progress_msg();

        let url = &source.url;
        let url_path = url_no_escape(
            url.strip_prefix("file:")
                .ok_or_else(|| DownloadError::InvalidURL(url.to_string()))?,
        );

        let url_path = Path::new(&url_path);

        let total_size = tokio::fs::metadata(url_path)
            .await
            .map_err(|e| {
                DownloadError::FailedOpenLocalSourceFile(self.entry.filename.to_string(), e)
            })?
            .len();

        progress_manager.new_progress_bar(self.download_list_index, &msg, total_size);

        if as_symlink {
            let symlink = self.entry.dir.join(&*self.entry.filename);
            if symlink.exists() {
                tokio::fs::remove_file(&symlink).await.map_err(|e| {
                    DownloadError::FailedOpenLocalSourceFile(self.entry.filename.to_string(), e)
                })?;
            }

            tokio::fs::symlink(url_path, symlink).await.map_err(|e| {
                DownloadError::FailedOpenLocalSourceFile(self.entry.filename.to_string(), e)
            })?;

            global_progress.fetch_add(total_size as u64, Ordering::SeqCst);
            progress_manager.global_progress_set(&global_progress);
            progress_manager.progress_done(self.download_list_index);

            return Ok(Summary {
                filename: self.entry.filename.clone(),
                wrote: true,
                count: self.download_list_index,
                context: self.context.clone(),
            });
        }

        debug!("File path is: {}", url_path.display());

        let from = tokio::fs::File::open(&url_path).await.map_err(|e| {
            DownloadError::FailedOpenLocalSourceFile(self.entry.filename.to_string(), e)
        })?;
        let from = tokio::io::BufReader::new(from).compat();

        debug!("Success open file: {}", url_path.display());

        let mut to = tokio::fs::File::create(self.entry.dir.join(&*self.entry.filename))
            .await
            .map_err(|e| {
                DownloadError::FailedOpenLocalSourceFile(self.entry.filename.to_string(), e)
            })?;

        let mut reader = match self.file_type {
            CompressFile::Xz => CompressStream::Xz(XzDecoder::new(BufReader::new(from))),
            CompressFile::Gzip => CompressStream::Gzip(GzipDecoder::new(BufReader::new(from))),
            CompressFile::Bz2 => CompressStream::Bz2(BzDecoder::new(BufReader::new(from))),
            CompressFile::Nothing => CompressStream::Nothing(BufReader::new(from)),
            CompressFile::Zstd => CompressStream::Zstd(ZstdDecoder::new(BufReader::new(from))),
        };
        let reader = reader.as_inner_mut();
        let mut reader = reader.compat();

        debug!(
            "Success create file: {}",
            self.entry.dir.join(&*self.entry.filename).display()
        );

        let mut buf = vec![0u8; 8 * 1024];

        loop {
            let size = reader.read(&mut buf[..]).await.map_err(|e| {
                DownloadError::FailedOpenLocalSourceFile(self.entry.filename.to_string(), e)
            })?;

            if size == 0 {
                break;
            }

            to.write_all(&buf[..size]).await.map_err(|e| {
                DownloadError::FailedOpenLocalSourceFile(self.entry.filename.to_string(), e)
            })?;

            progress_manager.progress_inc(self.download_list_index, size as u64);
            global_progress.fetch_add(size as u64, Ordering::SeqCst);
            progress_manager.global_progress_set(&global_progress);
        }

        progress_manager.progress_done(self.download_list_index);

        Ok(Summary {
            filename: self.entry.filename.clone(),
            wrote: true,
            count: self.download_list_index,
            context: self.context.clone(),
        })
    }
}

enum CompressStream<R: AsyncRead + Unpin + Send> {
    Xz(XzDecoder<R>),
    Gzip(GzipDecoder<R>),
    Bz2(BzDecoder<R>),
    Zstd(ZstdDecoder<R>),
    Nothing(R),
}

impl<R: AsyncRead + Unpin + Send + AsyncBufRead> CompressStream<R> {
    fn as_inner_mut(&mut self) -> &mut (dyn AsyncRead + Unpin + Send) {
        match self {
            CompressStream::Xz(reader) => reader,
            CompressStream::Gzip(reader) => reader,
            CompressStream::Bz2(reader) => reader,
            CompressStream::Zstd(reader) => reader,
            CompressStream::Nothing(reader) => reader,
        }
    }
}
