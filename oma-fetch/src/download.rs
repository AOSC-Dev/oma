use std::{io::SeekFrom, path::Path};

use async_compression::tokio::write::{GzipDecoder as WGzipDecoder, XzDecoder as WXzDecoder};
use indicatif::ProgressBar;
use oma_console::{debug, error, indicatif, warn, writer::Writer};
use reqwest::{
    header::{HeaderValue, ACCEPT_RANGES, CONTENT_LENGTH, RANGE},
    Client, StatusCode,
};
use tokio::io::{AsyncReadExt, AsyncSeekExt, AsyncWrite, AsyncWriteExt};

use crate::{
    checksum::Checksum, DownloadEntry, DownloadError, DownloadResult, DownloadSourceType,
    FetchProgressBar, Summary,
};

pub(crate) async fn try_download(
    client: &Client,
    entry: &DownloadEntry,
    fpb: Option<FetchProgressBar>,
    count: usize,
    retry_times: usize,
    context: Option<String>,
) -> DownloadResult<Summary> {
    let mut sources = entry.source.clone();
    sources.sort_by(|a, b| a.source_type.cmp(&b.source_type));

    let mut res = None;

    let mut err = None;

    for (i, c) in sources.iter().enumerate() {
        let download_res = match c.source_type {
            DownloadSourceType::Http => {
                try_http_download(
                    client,
                    entry,
                    fpb.clone(),
                    count,
                    retry_times,
                    context.clone(),
                    i,
                )
                .await
            }
            DownloadSourceType::Local => {
                download_local(entry, fpb.clone(), count, context.clone(), i).await
            }
        };

        match download_res {
            Ok(download_res) => {
                res = Some(download_res);
                break;
            }
            Err(e) => {
                err = Some(e.to_string());
                error!("Download failed: {e}, trying next mirror ...");
            }
        }
    }

    res.ok_or_else(|| DownloadError::DownloadAllFailed(entry.filename.to_string(), err.unwrap()))
}

async fn try_http_download(
    client: &Client,
    entry: &DownloadEntry,
    fpb: Option<FetchProgressBar>,
    count: usize,
    retry_times: usize,
    context: Option<String>,
    position: usize,
) -> DownloadResult<Summary> {
    let mut times = 0;
    loop {
        match http_download(client, entry, fpb.clone(), count, context.clone(), position).await {
            Ok(s) => {
                return Ok(s);
            }
            Err(e) => match e {
                DownloadError::ChecksumMisMatch(_, _) | DownloadError::ReqwestError(_) => {
                    if retry_times == times {
                        return Err(e);
                    }
                    warn!("Download Error: {e:?}, retrying {times} times ...");
                    times += 1;
                }
                _ => return Err(e),
            },
        }
    }
}

/// Download file
async fn http_download(
    client: &Client,
    entry: &DownloadEntry,
    fpb: Option<FetchProgressBar>,
    count: usize,
    context: Option<String>,
    position: usize,
) -> DownloadResult<Summary> {
    let file = entry.dir.join(&entry.filename);
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
            entry.filename
        );
        let hash = entry.hash.clone();
        if let Some(hash) = hash {
            debug!("Hash exist! It is: {hash}");

            let mut f = tokio::fs::OpenOptions::new()
                .create(true)
                .write(true)
                .read(true)
                .open(&file)
                .await?;

            debug!(
                "oma opened file: {} with create, write and read mode",
                entry.filename
            );

            let mut v = Checksum::from_sha256_str(&hash)?.get_validator();

            debug!("Validator created.");

            let mut buf = vec![0; 4096];
            let mut readed = 0;

            loop {
                if readed == file_size {
                    break;
                }

                let count = f.read(&mut buf[..]).await?;
                v.update(&buf[..count]);

                if let Some(ref fpb) = fpb {
                    if let Some(ref gpb) = fpb.global_bar {
                        gpb.inc(count as u64);
                    }
                }

                readed += count as u64;
            }

            if v.finish() {
                debug!(
                    "{} checksum success, no need to download anything.",
                    entry.filename
                );
                return Ok(Summary::new(&entry.filename, false, count, context));
            }

            debug!("checksum fail, will download this file: {}", entry.filename);

            if !entry.allow_resume {
                if let Some(ref gpb) = fpb.as_ref().and_then(|x| x.global_bar.clone()) {
                    gpb.set_position(gpb.position() - readed);
                }
            } else {
                dest = Some(f);
                validator = Some(v);
            }
        }
    }

    let fpbc = fpb.clone();
    let progress = if let Some((count, len)) = fpbc.and_then(|x| x.progress) {
        format!("({count}/{len}) ")
    } else {
        "".to_string()
    };

    let progress_clone = progress.clone();

    // 若请求头的速度太慢，会看到 Spinner 直到拿到头的信息
    let fpbc = fpb.clone();
    let pb = if let Some(fpb) = fpbc {
        let (style, inv) = oma_console::pb::oma_spinner(false)?;
        let pb = fpb.mb.add(ProgressBar::new_spinner().with_style(style));
        pb.enable_steady_tick(inv);
        let msg = fpb.msg.unwrap_or(entry.filename.clone());
        pb.set_message(format!("{progress_clone}{msg}"));

        Some(pb)
    } else {
        None
    };

    let url = entry.source[position].url.clone();
    let resp_head = client.head(url).send().await?;

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

    let url = entry.source[position].url.clone();
    let mut req = client.get(url);

    if can_resume && entry.allow_resume {
        // 如果已存在的文件大小大于或等于要下载的文件，则重置文件大小，重新下载
        // 因为已经走过一次 chekcusm 了，函数走到这里，则说明肯定文件完整性不对
        if total_size <= file_size {
            debug!("Exist file size is reset to 0, because total size <= exist file size");
            let fpbc = fpb.clone();
            if let Some(ref global_bar) = fpbc.and_then(|x| x.global_bar) {
                global_bar.set_position(global_bar.position() - file_size);
            }
            file_size = 0;
            can_resume = false;
        }

        // 发送 RANGE 的头，传入的是已经下载的文件的大小
        debug!("oma will set header range as bytes={file_size}-");
        req = req.header(RANGE, format!("bytes={}-", file_size));
    }

    debug!("Can resume? {can_resume}");

    let resp = req.send().await?;

    if let Err(e) = resp.error_for_status_ref() {
        if let Some(pb) = pb {
            pb.finish_and_clear();
        }
        match e.status() {
            Some(StatusCode::NOT_FOUND) => {
                let url = entry.source[position].url.clone();
                return Err(DownloadError::NotFound(url));
            }
            _ => return Err(e.into()),
        }
    } else if let Some(pb) = pb {
        // dbg!("111");
        pb.finish_and_clear();
    }

    let fpbc = fpb.clone();
    let pb = if let Some(fpb) = fpbc {
        let writer = Writer::default();
        let pb = fpb.mb.add(
            ProgressBar::new(total_size).with_style(oma_console::pb::oma_style_pb(writer, false)?),
        );

        Some(pb)
    } else {
        None
    };

    let progress = if let Some((count, len)) = fpb.clone().and_then(|x| x.progress) {
        format!("({count}/{len}) ")
    } else {
        "".to_string()
    };

    if let Some(pb) = &pb {
        let fpbc = fpb.clone();
        let msg = fpbc.and_then(|x| x.msg).unwrap_or(entry.filename.clone());

        pb.set_message(format!("{progress}{msg}"));
    }

    let mut source = resp;

    // 初始化 checksum 验证器
    // 如果文件存在，则 checksum 验证器已经初试过一次，因此进度条加已经验证过的文件大小

    let hash = entry.hash.clone();
    let mut validator = if let Some(v) = validator {
        if let Some(pb) = &pb {
            pb.inc(file_size);
        }
        Some(v)
    } else if let Some(hash) = hash {
        Some(Checksum::from_sha256_str(&hash)?.get_validator())
    } else {
        None
    };

    let mut dest = if !entry.allow_resume || !can_resume {
        // 如果不能 resume，则加入 truncate 这个 flag，告诉内核截断文件
        // 并把文件长度设置为 0
        debug!(
            "oma will open file: {} as truncate, create, write and read mode.",
            entry.filename
        );
        let f = tokio::fs::OpenOptions::new()
            .truncate(true)
            .create(true)
            .write(true)
            .read(true)
            .open(&file)
            .await?;

        debug!("Setting file length as 0");
        f.set_len(0).await?;

        f
    } else if let Some(dest) = dest {
        debug!("oma will re use opened dest file for {}", entry.filename);

        dest
    } else {
        debug!(
            "oma will open file: {} as create, write and read mode.",
            entry.filename
        );

        tokio::fs::OpenOptions::new()
            .create(true)
            .write(true)
            .read(true)
            .open(&file)
            .await?
    };

    // 把文件指针移动到末尾
    debug!("oma will seek file: {} to end", entry.filename);
    dest.seek(SeekFrom::End(0)).await?;

    let mut writer: Box<dyn AsyncWrite + Unpin + Send> =
        match Path::new(&entry.source[position].url)
            .extension()
            .and_then(|x| x.to_str())
        {
            Some("xz") if entry.extract => Box::new(WXzDecoder::new(&mut dest)),
            Some("gz") if entry.extract => Box::new(WGzipDecoder::new(&mut dest)),
            _ => Box::new(&mut dest),
        };

    // 下载！
    debug!("Start download!");
    while let Some(chunk) = source.chunk().await? {
        writer.write_all(&chunk).await?;
        if let Some(pb) = &pb {
            pb.inc(chunk.len() as u64);
        }

        let fpbc = fpb.clone();
        if let Some(ref gpb) = fpbc.and_then(|x| x.global_bar) {
            gpb.inc(chunk.len() as u64);
        }

        if let Some(ref mut v) = validator {
            v.update(&chunk);
        }
    }

    // 下载完成，告诉内核不再写这个文件了
    debug!("Download complete! shutting down dest file stream ...");
    writer.shutdown().await?;

    // 最后看看 chekcsum 验证是否通过
    if let Some(v) = validator {
        if !v.finish() {
            debug!("checksum fail: {}", entry.filename);
            let fpbc = fpb.clone();
            if let Some(ref gpb) = fpbc.and_then(|x| x.global_bar) {
                let pb = pb.unwrap();
                gpb.set_position(gpb.position() - pb.position());
                pb.reset();
            }

            let url = entry.source[position].url.clone();
            return Err(DownloadError::ChecksumMisMatch(
                url,
                entry.dir.display().to_string(),
            ));
        }

        debug!("checksum success: {}", entry.filename);
    }

    if let Some(pb) = pb {
        pb.finish_and_clear();
    }

    Ok(Summary::new(&entry.filename, true, count, context))
}

pub async fn download_local(
    entry: &DownloadEntry,
    fpb: Option<FetchProgressBar>,
    count: usize,
    context: Option<String>,
    position: usize,
) -> DownloadResult<Summary> {
    let pb = fpb.as_ref().map(|x| x.mb.add(ProgressBar::new_spinner()));

    let url = &entry.source[position].url;
    let mut from = tokio::fs::File::open(url).await.map_err(|e| {
        DownloadError::FailedOpenLocalSourceFile(entry.filename.clone(), e.to_string())
    })?;

    let mut to = tokio::fs::File::create(entry.dir.join(&entry.filename))
        .await
        .map_err(|e| {
            DownloadError::FailedOpenLocalSourceFile(entry.filename.clone(), e.to_string())
        })?;

    let size = tokio::io::copy(&mut from, &mut to).await.map_err(|e| {
        DownloadError::FailedOpenLocalSourceFile(entry.filename.clone(), e.to_string())
    })?;

    if let Some(pb) = pb {
        pb.finish_and_clear();
    }

    if let Some(ref gb) = fpb.and_then(|x| x.global_bar) {
        gb.inc(size);
    }

    Ok(Summary::new(&entry.filename, true, count, context))
}
