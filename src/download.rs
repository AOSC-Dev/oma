use std::{
    io::SeekFrom,
    path::{Path, PathBuf},
    sync::{atomic::Ordering, Arc},
    time::Duration,
};

use console::style;
use futures::StreamExt;
use futures_util::future::BoxFuture;
use tokio::{
    io::{AsyncReadExt, AsyncSeekExt},
    runtime::Runtime,
};

use anyhow::{anyhow, bail, Context, Result};
use indicatif::{MultiProgress, ProgressBar, ProgressStyle};
use reqwest::{
    header::{HeaderValue, ACCEPT_RANGES, CONTENT_LENGTH, RANGE},
    Client, StatusCode,
};
use tokio::io::AsyncWriteExt;

use crate::{
    checksum::Checksum,
    cli::gen_prefix,
    db::DOWNLOAD_DIR,
    error, info,
    oma::InstallRow,
    success,
    utils::{error_due_to, reverse_apt_style_url},
    warn, AILURUS, DRYRUN, WRITER,
};

/// Download a package
async fn download_single_pkg(
    urls: Vec<String>,
    client: &Client,
    hash: String,
    version: String,
    opb: OmaProgressBar,
    download_dir: PathBuf,
) -> Result<()> {
    let filename = urls
        .first()
        .and_then(|x| x.split('/').last())
        .take()
        .context("URLs is none or Invalid URL")?;

    let filename = trans_filename(filename, version)?;

    try_download(urls, client, &filename, hash, opb, &download_dir).await?;

    Ok(())
}

fn trans_filename(filename: &str, version: String) -> Result<String> {
    let mut filename_split = filename.split('_');
    let package = filename_split
        .next()
        .take()
        .context(format!("Can not parse filename: {filename}"))?;

    let arch_deb = filename_split
        .nth(1)
        .take()
        .context(format!("Can not parse version: {version}"))?;

    let arch_deb = if arch_deb == "noarch.deb" {
        "all.deb"
    } else {
        arch_deb
    };

    let version = version.replace(':', "%3a");
    let filename = format!("{package}_{version}_{arch_deb}").replace("%2b", "+");

    Ok(filename)
}

async fn try_download(
    urls: Vec<String>,
    client: &Client,
    filename: &String,
    hash: String,
    opb: OmaProgressBar,
    download_dir: &Path,
) -> Result<()> {
    let mut retry = 1;
    for (i, c) in urls.iter().enumerate() {
        let mut allow_resume = true;

        let r = loop {
            match download(
                c,
                client,
                filename.to_string(),
                download_dir,
                Some(&hash),
                opb.clone(),
                allow_resume,
            )
            .await
            {
                Ok(_) => {
                    break Ok(());
                }
                Err(e) => {
                    match e {
                        DownloadError::ChecksumMisMatch(_) => {
                            allow_resume = false;
                            if retry == 3 {
                                let s = format!("{c} checksum mismatch, try next url to download this package ...");
                                try_download_msg_display(&opb, i, &urls, &s);
                                break Err(e);
                            }
                            let s = format!("{c} checksum mismatch, retry {retry} times ...");
                            try_download_msg_display(&opb, i, &urls, &s);
                            retry += 1;
                        }
                        DownloadError::ReqwestError(_) => {
                            let mut s = format!("{e}");
                            if i < urls.len() - 1 {
                                s += ", try next url to download this package ...";
                            }
                            try_download_msg_display(&opb, i, &urls, &s);
                            break Err(e);
                        }
                        _ => {
                            break Err(e);
                        }
                    };
                }
            }
        };

        match r {
            Ok(_) => return Ok(()),
            Err(e) => match e {
                DownloadError::ChecksumMisMatch(_) => {
                    return Err(error_due_to(
                        format!(
                            "Can not download file: {filename} to dir {}, checksum mismatch.",
                            download_dir.display()
                        ),
                        "Maybe mirror still sync progress?",
                    ));
                }
                DownloadError::ReqwestError(e) => {
                    if i != urls.len() - 1 {
                        continue;
                    }
                    return Err(error_due_to(
                        format!("Can not download file: {filename}, why: {e}"),
                        "Maybe check your network settings?",
                    ));
                }
                DownloadError::IOError(e) => {
                    bail!(
                        "Can not download file: {filename} to dir {}, why: {e}",
                        download_dir.display()
                    );
                }
                _ => return Err(e.into()),
            },
        }
    }

    Ok(())
}

fn try_download_msg_display(opb: &OmaProgressBar, i: usize, urls: &Vec<String>, s: &str) {
    if let Some(ref gpb) = opb.global_bar {
        gpb.println(format!(
            "{}{s}",
            if i < urls.len() - 1 {
                gen_prefix(&style("WARNING").yellow().bold().to_string())
            } else {
                gen_prefix(&style("ERROR").red().bold().to_string())
            }
        ));
    } else if i < urls.len() - 1 {
        warn!("{s}");
    } else {
        error!("{s}");
    }
}

pub fn packages_download_runner(
    runtime: &Runtime,
    list: &[InstallRow],
    client: &Client,
    limit: Option<usize>,
    download_dir: Option<&Path>,
) -> Result<()> {
    runtime.block_on(packages_download(list, client, limit, download_dir))?;

    Ok(())
}

/// Download packages
async fn packages_download(
    list: &[InstallRow],
    client: &Client,
    limit: Option<usize>,
    download_dir: Option<&Path>,
) -> Result<()> {
    if DRYRUN.load(Ordering::Relaxed) {
        return Ok(());
    }

    let mut task = vec![];
    let mb = Arc::new(MultiProgress::new());
    let mut total = 0;

    if list.is_empty() {
        return Ok(());
    }

    info!("Downloading {} packages ...", list.len());

    for i in list.iter() {
        total += i.pure_download_size;
    }

    let global_bar = mb.insert(0, ProgressBar::new(total));
    global_bar.set_style(oma_style_pb(true)?);
    global_bar.enable_steady_tick(Duration::from_millis(100));
    global_bar.set_message(style("Progress:").bold().to_string());

    let list_len = list.len();

    let mut download_len = 0;

    for (i, c) in list.iter().enumerate() {
        let mbc = mb.clone();

        if let Some(url) = c.pkg_urls.iter().find(|x| x.starts_with("file:")) {
            let t: BoxFuture<'_, Result<()>> = Box::pin(download_single_pkg_local(
                url,
                c,
                OmaProgressBar::new(
                    None,
                    Some((i + 1, list_len)),
                    mbc.clone(),
                    Some(global_bar.clone()),
                ),
                download_dir,
            ));

            task.push(t);
        } else {
            let hash = c.checksum.as_ref().unwrap().to_owned();

            let download_dir_def = DOWNLOAD_DIR.clone();

            let t: BoxFuture<'_, Result<()>> = Box::pin(download_single_pkg(
                c.pkg_urls.clone(),
                client,
                hash,
                c.new_version.clone(),
                OmaProgressBar::new(
                    None,
                    Some((i + 1, list_len)),
                    mbc.clone(),
                    Some(global_bar.clone()),
                ),
                download_dir.unwrap_or(&download_dir_def).to_path_buf(),
            ));

            task.push(t);
        }

        download_len += 1;
    }

    // 默认限制一次最多下载八个包，减少服务器负担
    let stream = futures::stream::iter(task).buffer_unordered(limit.unwrap_or(4));
    let res = stream.collect::<Vec<_>>().await;

    global_bar.finish_and_clear();

    // 遍历结果看是否有下载出错
    for i in res {
        i?;
    }

    if download_len != 0 {
        success!("Downloaded {download_len} package.");
    } else {
        info!("No need to Fetch anything.");
    }

    Ok(())
}

async fn download_single_pkg_local(
    url: &str,
    c: &InstallRow,
    opb: OmaProgressBar,
    download_dir: Option<&Path>,
) -> Result<()> {
    let url = url.strip_prefix("file:").unwrap();
    let url = Path::new(url);
    let filename = url
        .file_name()
        .context(format!("Can not get filename {}!", url.display()))?
        .to_str()
        .context(format!("Can not get str {}!", url.display()))?;

    let url_filename = reverse_apt_style_url(filename);
    let filename = trans_filename(filename, c.new_version.clone())?;
    let url = url.parent().unwrap().join(url_filename);

    download_local(url, download_dir, filename, opb, c.pure_download_size).await?;

    Ok(())
}

pub async fn download_local(
    url: PathBuf,
    download_dir: Option<&Path>,
    filename: String,
    opb: OmaProgressBar,
    size: u64,
) -> Result<()> {
    let mut f = tokio::fs::File::open(url).await?;
    let mut to_f = tokio::fs::OpenOptions::new()
        .write(true)
        .create(true)
        .truncate(true)
        .open(download_dir.unwrap_or(&DOWNLOAD_DIR).join(&filename))
        .await?;

    to_f.set_len(0).await?;

    let mut buf = vec![0; 4096];

    let pb = opb.mbc.add(ProgressBar::new(size));
    pb.set_style(oma_style_pb(false)?);
    pb.enable_steady_tick(Duration::from_millis(100));

    let msg = opb.msg.unwrap_or_else(|| download_pkg_msg(&filename));
    let progress = if let Some((count, len)) = opb.progress {
        format!("({count}/{len}) ")
    } else {
        "".to_string()
    };

    pb.set_message(format!("{progress}{msg}"));

    loop {
        let read_count = f.read(&mut buf).await?;

        if read_count == 0 {
            break;
        }

        to_f.write_all(&buf[..read_count]).await?;
        pb.inc(read_count as u64);
        if let Some(ref gb) = opb.global_bar {
            gb.inc(read_count as u64);
        }
    }

    pb.finish_and_clear();

    Ok(())
}

#[derive(Clone)]
pub struct OmaProgressBar {
    pub msg: Option<String>,
    pub mbc: Arc<MultiProgress>,
    pub progress: Option<(usize, usize)>,
    pub global_bar: Option<ProgressBar>,
}

impl OmaProgressBar {
    pub fn new(
        msg: Option<String>,
        progress: Option<(usize, usize)>,
        mbc: Arc<MultiProgress>,
        global_bar: Option<ProgressBar>,
    ) -> Self {
        Self {
            msg,
            mbc,
            progress,
            global_bar,
        }
    }
}

#[derive(thiserror::Error, Debug)]
pub enum DownloadError {
    #[error("checksum mismatch {0}")]
    ChecksumMisMatch(String),
    #[error("404 not found: {0}")]
    NotFound(String),
    #[error(transparent)]
    Anyhow(#[from] anyhow::Error),
    #[error(transparent)]
    JoinError(#[from] tokio::task::JoinError),
    #[error(transparent)]
    IOError(#[from] tokio::io::Error),
    #[error(transparent)]
    ReqwestError(#[from] reqwest::Error),
}

type DownloadResult<T> = std::result::Result<T, DownloadError>;

/// Download file
/// Return bool is file is started download
pub async fn download(
    url: &str,
    client: &Client,
    filename: String,
    dir: &Path,
    hash: Option<&str>,
    opb: OmaProgressBar,
    allow_resume: bool,
) -> DownloadResult<bool> {
    let file = dir.join(&filename);
    let file_exist = file.exists();
    let mut file_size = file.metadata().ok().map(|x| x.len()).unwrap_or(0);

    tracing::debug!("Exist file size is: {file_size}");
    let mut dest = None;
    let mut validator = None;

    // 如果要下载的文件已经存在，则验证 Checksum 是否正确，若正确则添加总进度条的进度，并返回
    // 如果不存在，则继续往下走
    if file_exist {
        tracing::debug!("File: {filename} exists, oma will checksum this file.");
        if let Some(hash) = hash {
            tracing::debug!("Hash exist! It is: {hash}");
            let hash = hash.to_owned();

            let mut f = tokio::fs::OpenOptions::new()
                .create(true)
                .write(true)
                .read(true)
                .open(&file)
                .await?;

            tracing::debug!("oma opened file: {filename} with create, write and read mode");

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

                if let Some(ref global_bar) = opb.global_bar {
                    global_bar.inc(count as u64);
                }

                readed += count as u64;
            }

            if v.finish() {
                tracing::debug!("{filename} checksum success, no need to download anything.");
                return Ok(false);
            }

            tracing::debug!("checksum fail, will download this file: {filename}");

            dest = Some(f);
            validator = Some(v);
        }
    }

    // 写入进度条显示的信息
    let mut msg = opb.msg.unwrap_or_else(|| download_pkg_msg(&filename));

    let progress = if let Some((count, len)) = opb.progress {
        format!("({count}/{len}) ")
    } else {
        "".to_string()
    };

    let msg_clone = msg.clone();
    let progress_clone = progress.clone();

    let mbcc = opb.mbc.clone();

    // 若请求头的速度太慢，会看到 Spinner 直到拿到头的信息
    let pb = mbcc.add(ProgressBar::new_spinner());
    pb.set_message(format!("{progress_clone}{msg_clone}"));

    oma_spinner(&pb);

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

    tracing::debug!("Can resume? {can_resume}");

    // 从服务器获取文件的总大小
    let total_size = {
        let total_size = head
            .get(CONTENT_LENGTH)
            .map(|x| x.to_owned())
            .unwrap_or(HeaderValue::from(0));

        total_size
            .to_str()
            .map_err(|e| anyhow!("{e}"))?
            .parse::<u64>()
            .map_err(|e| anyhow!("{e}"))?
    };

    tracing::debug!("File total size is: {total_size}");

    let mut req = client.get(url);

    if can_resume && allow_resume {
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
        pb.finish_and_clear();
        match e.status() {
            Some(StatusCode::NOT_FOUND) => return Err(DownloadError::NotFound(url.to_string())),
            _ => return Err(e.into()),
        }
    } else {
        pb.finish_and_clear();
    }

    let pb = opb.mbc.add(ProgressBar::new(total_size));
    pb.set_style(oma_style_pb(false)?);

    if console::measure_text_width(&msg) > 30 {
        msg = console::truncate_str(&msg, 27, "...").to_string();
    }

    let progress = if let Some((count, len)) = opb.progress {
        format!("({count}/{len}) ")
    } else {
        "".to_string()
    };

    pb.set_message(format!("{progress}{msg}"));
    pb.enable_steady_tick(Duration::from_millis(100));

    let mut source = resp;

    // 初始化 checksum 验证器
    // 如果文件存在，则 checksum 验证器已经初试过一次，因此进度条加已经验证过的文件大小
    let mut validator = if let Some(v) = validator {
        pb.inc(file_size);
        Some(v)
    } else if let Some(hash) = hash {
        Some(Checksum::from_sha256_str(hash)?.get_validator())
    } else {
        None
    };

    let mut dest = if !allow_resume || !can_resume {
        // 如果不能 resume，则加入 truncate 这个 flag，告诉内核截断文件
        // 并把文件长度设置为 0
        tracing::debug!("oma will open file: {filename} as truncate, create, write and read mode.");
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
        tracing::debug!("oma will re use opened dest file for {filename}");

        dest
    } else {
        tracing::debug!("oma will open file: {filename} as create, write and read mode.");

        tokio::fs::OpenOptions::new()
            .create(true)
            .write(true)
            .read(true)
            .open(&file)
            .await?
    };

    // 把文件指针移动到末尾
    tracing::debug!("oma will seek file: {filename} to end");
    dest.seek(SeekFrom::End(0)).await?;

    // 下载！
    tracing::debug!("Start download!");
    while let Some(chunk) = source.chunk().await? {
        dest.write_all(&chunk).await?;
        pb.inc(chunk.len() as u64);

        if let Some(ref global_bar) = opb.global_bar {
            global_bar.inc(chunk.len() as u64);
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
            tracing::debug!("checksum fail: {filename}");

            if let Some(ref global_bar) = opb.global_bar {
                global_bar.set_position(global_bar.position() - pb.position());
            }
            pb.reset();
            return Err(DownloadError::ChecksumMisMatch(url.to_string()));
        }

        tracing::debug!("checksum success: {filename}");
    }

    pb.finish_and_clear();

    Ok(true)
}

fn download_pkg_msg(filename: &str) -> String {
    let mut filename_split = filename.split('_');
    let name = filename_split.next();
    let version = filename_split.next().map(|x| x.replace("%3a", ":"));
    let arch = filename_split.next().map(|x| x.replace(".deb", ""));

    if name.and(version.clone()).and(arch.clone()).is_none() {
        filename.to_string()
    } else {
        format!("{} {} ({})", name.unwrap(), version.unwrap(), arch.unwrap())
    }
}

pub fn oma_spinner(pb: &ProgressBar) {
    let (is_egg, inv) = if AILURUS.load(Ordering::Relaxed) {
        (
            &[
                "☀️ ", "☀️ ", "☀️ ", "🌤 ", "⛅️ ", "🌥 ", "☁️ ", "🌧 ", "🌨 ", "🌧 ", "🌨 ", "🌧 ", "🌨 ",
                "⛈ ", "🌨 ", "🌧 ", "🌨 ", "☁️ ", "🌥 ", "⛅️ ", "🌤 ", "☀️ ", "☀️ ",
            ][..],
            100,
        )
    } else {
        (
            &[
                "( ●    )",
                "(  ●   )",
                "(   ●  )",
                "(    ● )",
                "(     ●)",
                "(    ● )",
                "(   ●  )",
                "(  ●   )",
                "( ●    )",
                "(●     )",
            ][..],
            80,
        )
    };

    pb.enable_steady_tick(Duration::from_millis(inv));

    pb.set_style(
        ProgressStyle::with_template(" {msg:<48} {spinner}")
            .unwrap()
            // For more spinners check out the cli-spinners project:
            // https://github.com/sindresorhus/cli-spinners/blob/master/spinners.json
            .tick_strings(is_egg),
    );
}

pub fn oma_style_pb(is_global: bool) -> Result<ProgressStyle> {
    let bar_template = {
        let max_len = WRITER.get_max_len();
        if is_global {
            if max_len < 90 {
                " {msg:.blue.bold}".to_owned()
                    + " {bytes:>10.green.bold} "
                    + &style("/").green().bold().to_string()
                    + " {total_bytes:.green.bold} "
                    + &style("@").green().bold().to_string()
                    + " {binary_bytes_per_sec:<13.green.bold}"
            } else {
                " {msg:.blue.bold}".to_owned()
                    + " {bytes:>10.green.bold} "
                    + &style("/").green().bold().to_string()
                    + " {total_bytes:.green.bold} "
                    + &style("@").green().bold().to_string()
                    + " {binary_bytes_per_sec:<13.green.bold}"
                    + "{eta_precise:>12.blue.bold}   [{wide_bar:.blue.bold}] {percent:>3.blue.bold}"
                    + &style("%").blue().bold().to_string()
            }
        } else if max_len < 90 {
            " {msg} {percent:>3}%".to_owned()
        } else {
            " {msg:<48} {total_bytes:>10}   [{wide_bar:.white/black}] {percent:>3}%".to_owned()
        }
    };

    let barsty = ProgressStyle::default_bar()
        .template(&bar_template)?
        .progress_chars("=>-");

    Ok(barsty)
}
