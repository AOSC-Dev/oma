use crate::DownloadEvent;
use std::{
    io::SeekFrom,
    path::Path,
    sync::{
        atomic::{AtomicU64, Ordering},
        Arc,
    },
};

use async_compression::tokio::write::{GzipDecoder as WGzipDecoder, XzDecoder as WXzDecoder};
use derive_builder::Builder;
use oma_console::debug;
use oma_utils::url_no_escape::url_no_escape;
use reqwest::{
    header::{HeaderValue, ACCEPT_RANGES, CONTENT_LENGTH, RANGE},
    Client,
};
use tokio::io::{AsyncReadExt, AsyncSeekExt, AsyncWrite, AsyncWriteExt};

use crate::{
    checksum::Checksum, DownloadEntry, DownloadError, DownloadResult, DownloadSourceType, Summary,
};

#[derive(Debug, Builder)]
pub(crate) struct SingleDownloader<'a> {
    client: &'a Client,
    entry: &'a DownloadEntry,
    progress: (usize, usize, Arc<Option<String>>),
    retry_times: usize,
    context: Arc<Option<String>>,
    download_list_index: usize,
}

impl SingleDownloader<'_> {
    pub(crate) async fn try_download<F>(
        self,
        global_progress: Arc<AtomicU64>,
        callback: Arc<F>,
    ) -> DownloadResult<Summary>
    where
        F: Fn(usize, DownloadEvent) + Clone,
    {
        let mut sources = self.entry.source.clone();
        sources.sort_unstable_by(|a, b| a.source_type.cmp(&b.source_type));

        let mut res = None;

        let cc = callback.clone();
        let gpc = global_progress.clone();
        let msg = self.progress.2.as_deref().unwrap_or(&*self.entry.filename);

        for (i, c) in sources.iter().enumerate() {
            let download_res = match c.source_type {
                DownloadSourceType::Http => {
                    self.try_http_download(i, cc.clone(), gpc.clone()).await
                }
                DownloadSourceType::Local => self.download_local(i, cc.clone(), gpc.clone()).await,
            };

            match download_res {
                Ok(download_res) => {
                    res = Some(download_res);
                    callback(
                        self.download_list_index,
                        DownloadEvent::Done(msg.to_string()),
                    );
                    break;
                }
                Err(e) => {
                    if i == sources.len() - 1 {
                        return Err(e);
                    }
                    callback(
                        self.download_list_index,
                        DownloadEvent::CanNotGetSourceNextUrl(e.to_string()),
                    );
                }
            }
        }

        // 如果能够推出循环，则说明 res 肯定有值
        Ok(res.unwrap())
    }

    /// Downlaod file with retry (http)
    async fn try_http_download<F>(
        &self,
        position: usize,
        callback: Arc<F>,
        global_progress: Arc<AtomicU64>,
    ) -> DownloadResult<Summary>
    where
        F: Fn(usize, DownloadEvent) + Clone,
    {
        let mut times = 1;
        let mut allow_resume = self.entry.allow_resume;
        loop {
            match self
                .http_download(
                    position,
                    callback.clone(),
                    global_progress.clone(),
                    allow_resume,
                )
                .await
            {
                Ok(s) => {
                    return Ok(s);
                }
                Err(e) => match e {
                    DownloadError::ChecksumMisMatch(ref filename) => {
                        if self.retry_times == times {
                            return Err(e);
                        }
                        callback(
                            self.download_list_index,
                            DownloadEvent::ChecksumMismatchRetry {
                                filename: filename.clone(),
                                times,
                            },
                        );
                        times += 1;
                        allow_resume = false;
                    }
                    _ => return Err(e),
                },
            }
        }
    }

    async fn http_download<F>(
        &self,
        position: usize,
        callback: Arc<F>,
        global_progress: Arc<AtomicU64>,
        allow_resume: bool,
    ) -> DownloadResult<Summary>
    where
        F: Fn(usize, DownloadEvent) + Clone,
    {
        let file = self.entry.dir.join(&*self.entry.filename);
        let file_exist = file.exists();
        let mut file_size = file.metadata().ok().map(|x| x.len()).unwrap_or(0);

        debug!("Exist file size is: {file_size}");
        let mut dest = None;
        let mut validator = None;

        // 如果要下载的文件已经存在，则验证 Checksum 是否正确，若正确则添加总进度条的进度，并返回
        // 如果不存在，则继续往下走
        if file_exist {
            debug!(
                "File: {} exists, oma will checksum this file.",
                self.entry.filename
            );
            if let Some(hash) = &self.entry.hash {
                debug!("Hash exist! It is: {hash}");

                let mut f = tokio::fs::OpenOptions::new()
                    .create(true)
                    .write(true)
                    .read(true)
                    .open(&file)
                    .await?;

                debug!(
                    "oma opened file: {} with create, write and read mode",
                    self.entry.filename
                );

                let mut v = Checksum::from_sha256_str(hash)?.get_validator();

                debug!("Validator created.");

                let mut buf = vec![0; 4096];
                let mut readed = 0;

                loop {
                    if readed == file_size {
                        break;
                    }

                    let readed_count = f.read(&mut buf[..]).await?;
                    v.update(&buf[..readed_count]);

                    global_progress.fetch_add(readed_count as u64, Ordering::SeqCst);

                    callback(
                        self.download_list_index,
                        DownloadEvent::GlobalProgressInc(readed_count as u64),
                    );

                    readed += readed_count as u64;
                }

                if v.finish() {
                    debug!(
                        "{} checksum success, no need to download anything.",
                        self.entry.filename
                    );

                    callback(self.download_list_index, DownloadEvent::ProgressDone);

                    return Ok(Summary::new(
                        self.entry.filename.clone(),
                        false,
                        self.download_list_index,
                        self.context.clone(),
                    ));
                }

                debug!(
                    "checksum fail, will download this file: {}",
                    self.entry.filename
                );

                if !allow_resume {
                    global_progress.fetch_sub(readed, Ordering::SeqCst);
                    let progress = global_progress.load(Ordering::SeqCst);
                    callback(
                        self.download_list_index,
                        DownloadEvent::GlobalProgressSet(progress),
                    );
                } else {
                    dest = Some(f);
                    validator = Some(v);
                }
            }
        }

        let msg = self.set_progress_msg();
        callback(
            self.download_list_index,
            DownloadEvent::NewProgressSpinner(msg.clone()),
        );

        let resp_head = match self
            .client
            .head(&self.entry.source[position].url)
            .send()
            .await
        {
            Ok(resp) => resp,
            Err(e) => {
                callback(self.download_list_index, DownloadEvent::ProgressDone);
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

        let mut req = self.client.get(&self.entry.source[position].url);

        if can_resume && allow_resume {
            // 如果已存在的文件大小大于或等于要下载的文件，则重置文件大小，重新下载
            // 因为已经走过一次 chekcusm 了，函数走到这里，则说明肯定文件完整性不对
            if total_size <= file_size {
                debug!("Exist file size is reset to 0, because total size <= exist file size");
                let gp = global_progress.load(Ordering::SeqCst);
                callback(
                    self.download_list_index,
                    DownloadEvent::GlobalProgressSet(gp - file_size),
                );
                global_progress.store(gp - file_size, Ordering::SeqCst);
                file_size = 0;
                can_resume = false;
            }

            // 发送 RANGE 的头，传入的是已经下载的文件的大小
            debug!("oma will set header range as bytes={file_size}-");
            req = req.header(RANGE, format!("bytes={}-", file_size));
        }

        debug!("Can resume? {can_resume}");

        let resp = req
            .send()
            .await
            .map_err(|e| DownloadError::ReqwestError(e))?;

        if let Err(e) = resp.error_for_status_ref() {
            callback(self.download_list_index, DownloadEvent::ProgressDone);
            return Err(DownloadError::ReqwestError(e));
        } else {
            callback(self.download_list_index, DownloadEvent::ProgressDone);
        }

        callback(
            self.download_list_index,
            DownloadEvent::NewProgress(total_size, msg.clone()),
        );

        let mut source = resp;

        // 初始化 checksum 验证器
        // 如果文件存在，则 checksum 验证器已经初试过一次，因此进度条加已经验证过的文件大小
        let hash = &self.entry.hash;
        let mut validator = if let Some(v) = validator {
            callback(
                self.download_list_index,
                DownloadEvent::ProgressInc(file_size),
            );
            Some(v)
        } else if let Some(hash) = hash {
            Some(Checksum::from_sha256_str(hash)?.get_validator())
        } else {
            None
        };

        let mut self_progress = 0;
        let mut dest = if !can_resume || !allow_resume {
            // 如果不能 resume，则加入 truncate 这个 flag，告诉内核截断文件
            // 并把文件长度设置为 0
            debug!(
                "oma will open file: {} as truncate, create, write and read mode.",
                self.entry.filename
            );
            let f = match tokio::fs::OpenOptions::new()
                .truncate(true)
                .create(true)
                .write(true)
                .read(true)
                .open(&file)
                .await
            {
                Ok(f) => f,
                Err(e) => {
                    callback(self.download_list_index, DownloadEvent::ProgressDone);
                    return Err(e.into());
                }
            };

            debug!("Setting file length as 0");
            f.set_len(0).await?;

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
                "oma will open file: {} as create, write and read mode.",
                self.entry.filename
            );

            match tokio::fs::OpenOptions::new()
                .create(true)
                .write(true)
                .read(true)
                .open(&file)
                .await
            {
                Ok(f) => f,
                Err(e) => {
                    callback(self.download_list_index, DownloadEvent::ProgressDone);
                    return Err(e.into());
                }
            }
        };

        // 把文件指针移动到末尾
        debug!("oma will seek file: {} to end", self.entry.filename);
        if let Err(e) = dest.seek(SeekFrom::End(0)).await {
            callback(self.download_list_index, DownloadEvent::ProgressDone);
            return Err(e.into());
        }

        let mut writer: Box<dyn AsyncWrite + Unpin + Send> =
            match Path::new(&self.entry.source[position].url)
                .extension()
                .and_then(|x| x.to_str())
            {
                Some("xz") if self.entry.extract => Box::new(WXzDecoder::new(&mut dest)),
                Some("gz") if self.entry.extract => Box::new(WGzipDecoder::new(&mut dest)),
                _ => Box::new(&mut dest),
            };

        // 下载！
        debug!("Start download!");
        while let Some(chunk) = source
            .chunk()
            .await
            .map_err(|e| DownloadError::ReqwestError(e))?
        {
            if let Err(e) = writer.write_all(&chunk).await {
                callback(self.download_list_index, DownloadEvent::ProgressDone);
                return Err(e.into());
            }

            debug!("{self_progress}");

            callback(
                self.download_list_index,
                DownloadEvent::ProgressInc(chunk.len() as u64),
            );

            self_progress += chunk.len() as u64;

            callback(
                self.download_list_index,
                DownloadEvent::GlobalProgressInc(chunk.len() as u64),
            );

            global_progress.fetch_add(chunk.len() as u64, Ordering::SeqCst);

            if let Some(ref mut v) = validator {
                v.update(&chunk);
            }
        }

        // 下载完成，告诉内核不再写这个文件了
        debug!("Download complete! shutting down dest file stream ...");
        if let Err(e) = writer.shutdown().await {
            callback(self.download_list_index, DownloadEvent::ProgressDone);
            return Err(e.into());
        }

        // 最后看看 chekcsum 验证是否通过
        if let Some(v) = validator {
            if !v.finish() {
                debug!("checksum fail: {}", self.entry.filename);
                debug!("{global_progress:?}");
                debug!("{self_progress}");

                global_progress.fetch_sub(self_progress, Ordering::SeqCst);
                let now_gp = global_progress.load(Ordering::SeqCst);

                debug!("Reset to: {now_gp}");

                callback(
                    self.download_list_index,
                    DownloadEvent::GlobalProgressSet(now_gp),
                );
                callback(self.download_list_index, DownloadEvent::ProgressDone);

                return Err(DownloadError::ChecksumMisMatch(
                    self.entry.filename.to_string(),
                ));
            }

            debug!("checksum success: {}", self.entry.filename);
        }

        callback(self.download_list_index, DownloadEvent::ProgressDone);

        Ok(Summary::new(
            self.entry.filename.clone(),
            true,
            self.download_list_index,
            self.context.clone(),
        ))
    }

    fn set_progress_msg(&self) -> String {
        let (count, len, msg) = &self.progress;
        let msg = msg.as_deref().unwrap_or(&self.entry.filename);
        let msg = format!("({count}/{len}) {msg}");

        msg
    }

    /// Download local source file
    async fn download_local<F>(
        &self,
        position: usize,
        callback: Arc<F>,
        global_progress: Arc<AtomicU64>,
    ) -> DownloadResult<Summary>
    where
        F: Fn(usize, DownloadEvent) + Clone,
    {
        debug!("{:?}", self.entry);
        let msg = self.set_progress_msg();
        callback(
            self.download_list_index,
            DownloadEvent::NewProgressSpinner(msg.clone()),
        );

        let url = &self.entry.source[position].url;
        let url_path = url_no_escape(
            url.strip_prefix("file:")
                .ok_or_else(|| DownloadError::InvaildURL(url.to_string()))?,
        );

        debug!("File path is: {url_path}");

        let mut from = tokio::fs::File::open(&url_path).await.map_err(|e| {
            DownloadError::FailedOpenLocalSourceFile(self.entry.filename.to_string(), e)
        })?;

        debug!("Success open file: {url_path}");

        let mut to = tokio::fs::File::create(self.entry.dir.join(&*self.entry.filename))
            .await
            .map_err(|e| {
                DownloadError::FailedOpenLocalSourceFile(self.entry.filename.to_string(), e)
            })?;

        debug!(
            "Success create file: {}",
            self.entry.dir.join(&*self.entry.filename).display()
        );

        let size = tokio::io::copy(&mut from, &mut to).await.map_err(|e| {
            DownloadError::FailedOpenLocalSourceFile(self.entry.filename.to_string(), e)
        })?;

        debug!(
            "Success copy file from {url_path} to {}",
            self.entry.dir.join(&*self.entry.filename).display()
        );

        callback(self.download_list_index, DownloadEvent::ProgressDone);
        callback(
            self.download_list_index,
            DownloadEvent::GlobalProgressInc(size),
        );
        global_progress.fetch_add(size, Ordering::SeqCst);

        Ok(Summary::new(
            self.entry.filename.clone(),
            true,
            position,
            self.context.clone(),
        ))
    }
}
