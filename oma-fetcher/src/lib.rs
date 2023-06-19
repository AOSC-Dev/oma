use std::{io::SeekFrom, path::Path, sync::Arc};

use indicatif::{MultiProgress, ProgressBar};
use reqwest::{
    header::{HeaderValue, ACCEPT_RANGES, CONTENT_LENGTH, RANGE},
    Client, StatusCode,
};
use tokio::io::{AsyncReadExt, AsyncSeekExt, AsyncWriteExt};

use crate::checksum::Checksum;

pub mod checksum;

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
}

pub type DownloadResult<T> = std::result::Result<T, DownloadError>;

pub struct FetchFile<'a> {
    url: &'a str,
    client: &'a Client,
    filename: String,
    dir: &'a Path,
    hash: Option<&'a str>,
    allow_resume: bool,
    fpb: Option<FetchProgressBar>,
}

#[derive(Clone)]
pub struct FetchProgressBar {
    mb: Arc<MultiProgress>,
    global_bar: Option<ProgressBar>,
    pub progress: Option<(usize, usize)>,
    pub msg: Option<String>,
}

/// Download file
/// Return bool is file is started download
pub async fn download(ff: FetchFile<'_>) -> DownloadResult<bool> {
    let file = ff.dir.join(&ff.filename);
    let file_exist = file.exists();
    let mut file_size = file.metadata().ok().map(|x| x.len()).unwrap_or(0);

    tracing::debug!("Exist file size is: {file_size}");
    let mut dest = None;
    let mut validator = None;

    // 如果要下载的文件已经存在，则验证 Checksum 是否正确，若正确则添加总进度条的进度，并返回
    // 如果不存在，则继续往下走
    if file_exist {
        tracing::debug!("File: {} exists, oma will checksum this file.", ff.filename);
        if let Some(hash) = ff.hash {
            tracing::debug!("Hash exist! It is: {hash}");
            let hash = hash.to_owned();

            let mut f = tokio::fs::OpenOptions::new()
                .create(true)
                .write(true)
                .read(true)
                .open(&file)
                .await?;

            tracing::debug!(
                "oma opened file: {} with create, write and read mode",
                ff.filename
            );

            let mut v = Checksum::from_sha256_str(&hash)?.get_validator();

            tracing::debug!("Validator created.");

            let mut buf = vec![0; 4096];
            let mut readed = 0;

            loop {
                if readed == file_size {
                    break;
                }

                let count = f.read(&mut buf[..]).await?;
                v.update(&buf[..count]);

                let fpb = ff.fpb.clone();
                if let Some(fpb) = fpb {
                    if let Some(ref gpb) = fpb.global_bar {
                        gpb.inc(count as u64);
                    }
                }

                readed += count as u64;
            }

            if v.finish() {
                tracing::debug!(
                    "{} checksum success, no need to download anything.",
                    ff.filename
                );
                return Ok(false);
            }

            tracing::debug!("checksum fail, will download this file: {}", ff.filename);

            if !ff.allow_resume {
                let fpb = ff.fpb.clone();
                if let Some(ref gpb) = fpb.and_then(|x| x.global_bar) {
                    gpb.set_position(gpb.position() - readed);
                }
            } else {
                dest = Some(f);
                validator = Some(v);
            }
        }
    }

    let fpb = ff.fpb.clone();
    let progress = if let Some((count, len)) = fpb.and_then(|x| x.progress) {
        format!("({count}/{len}) ")
    } else {
        "".to_string()
    };

    let progress_clone = progress.clone();

    // 若请求头的速度太慢，会看到 Spinner 直到拿到头的信息
    let fpb = ff.fpb.clone();
    let pb = if let Some(fpb) = fpb {
        let pb = fpb.mb.add(ProgressBar::new_spinner());
        let msg = "todo";
        pb.set_message(format!("{progress_clone}{msg}"));

        Some(pb)
    } else {
        None
    };

    let resp_head = ff.client.head(ff.url).send().await?;

    let head = resp_head.headers();

    // 看看头是否有 ACCEPT_RANGES 这个变量
    // 如果有，而且值不为 none，则可以断点续传
    // 反之，则不能断点续传
    let mut can_resume = match head.get(ACCEPT_RANGES) {
        Some(x) if x == "none" => false,
        Some(_) => true,
        None => false,
    };

    tracing::debug!("Can resume? {can_resume}");

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
            .ok_or_else(|| DownloadError::InvaildTotal(ff.url.to_string()))?
    };

    tracing::debug!("File total size is: {total_size}");

    let mut req = ff.client.get(ff.url);

    if can_resume && ff.allow_resume {
        // 如果已存在的文件大小大于或等于要下载的文件，则重置文件大小，重新下载
        // 因为已经走过一次 chekcusm 了，函数走到这里，则说明肯定文件完整性不对
        if total_size <= file_size {
            tracing::debug!("Exist file size is reset to 0, because total size <= exist file size");
            file_size = 0;
            can_resume = false;
        }

        // 发送 RANGE 的头，传入的是已经下载的文件的大小
        tracing::debug!("oma will set header range as bytes={file_size}-");
        req = req.header(RANGE, format!("bytes={}-", file_size));
    }

    tracing::debug!("Can resume? {can_resume}");

    let resp = req.send().await?;

    if let Err(e) = resp.error_for_status_ref() {
        if let Some(pb) = pb {
            pb.finish_and_clear();
        }
        match e.status() {
            Some(StatusCode::NOT_FOUND) => return Err(DownloadError::NotFound(ff.url.to_string())),
            _ => return Err(e.into()),
        }
    } else {
        if let Some(pb) = pb {
            pb.finish_and_clear();
        }
    }

    let fpb = ff.fpb.clone();
    let pb = if let Some(fpb) = fpb {
        Some(fpb.mb.add(ProgressBar::new(total_size)))
    } else {
        None
    };

    let fpb = ff.fpb.clone();
    let progress = if let Some((count, len)) = fpb.and_then(|x| x.progress) {
        format!("({count}/{len}) ")
    } else {
        "".to_string()
    };

    if let Some(pb) = &pb {
        let msg = "todo";
        pb.set_message(format!("{progress}{msg}"));
    }

    let mut source = resp;

    // 初始化 checksum 验证器
    // 如果文件存在，则 checksum 验证器已经初试过一次，因此进度条加已经验证过的文件大小
    let mut validator = if let Some(v) = validator {
        if let Some(pb) = &pb {
            pb.inc(file_size);
        }
        Some(v)
    } else if let Some(hash) = ff.hash {
        Some(Checksum::from_sha256_str(hash)?.get_validator())
    } else {
        None
    };

    let mut dest = if !ff.allow_resume || !can_resume {
        // 如果不能 resume，则加入 truncate 这个 flag，告诉内核截断文件
        // 并把文件长度设置为 0
        tracing::debug!(
            "oma will open file: {} as truncate, create, write and read mode.",
            ff.filename
        );
        let f = tokio::fs::OpenOptions::new()
            .truncate(true)
            .create(true)
            .write(true)
            .read(true)
            .open(&file)
            .await?;

        tracing::debug!("Setting file length as 0");
        f.set_len(0).await?;

        f
    } else if let Some(dest) = dest {
        tracing::debug!("oma will re use opened dest file for {}", ff.filename);

        dest
    } else {
        tracing::debug!(
            "oma will open file: {} as create, write and read mode.",
            ff.filename
        );

        tokio::fs::OpenOptions::new()
            .create(true)
            .write(true)
            .read(true)
            .open(&file)
            .await?
    };

    // 把文件指针移动到末尾
    tracing::debug!("oma will seek file: {} to end", ff.filename);
    dest.seek(SeekFrom::End(0)).await?;

    // 下载！
    tracing::debug!("Start download!");
    while let Some(chunk) = source.chunk().await? {
        dest.write_all(&chunk).await?;
        if let Some(pb) = &pb {
            pb.inc(chunk.len() as u64);
        }

        let fpb = ff.fpb.clone();
        if let Some(ref gpb) = fpb.and_then(|x| x.global_bar) {
            gpb.inc(chunk.len() as u64);
        }

        if let Some(ref mut v) = validator {
            v.update(&chunk);
        }
    }

    // 下载完成，告诉内核不再写这个文件了
    tracing::debug!("Download complete! shutting down dest file stream ...");
    dest.shutdown().await?;

    // 最后看看 chekcsum 验证是否通过
    if let Some(v) = validator {
        if !v.finish() {
            tracing::debug!("checksum fail: {}", ff.filename);

            let fpb = ff.fpb.clone();
            if let Some(ref gpb) = fpb.and_then(|x| x.global_bar) {
                let pb = pb.unwrap();
                gpb.set_position(gpb.position() - pb.position());
                pb.reset();
            }

            return Err(DownloadError::ChecksumMisMatch(ff.url.to_string()));
        }

        tracing::debug!("checksum success: {}", ff.filename);
    }

    if let Some(pb) = pb {
        pb.finish_and_clear();
    }

    Ok(true)
}
