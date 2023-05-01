use std::{
    io::SeekFrom,
    path::{Path, PathBuf},
    sync::{atomic::Ordering, Arc},
    time::Duration,
};

use console::style;
use futures::StreamExt;
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
    checksum::Checksum, cli::gen_prefix, db::DOWNLOAD_DIR, error, info, oma::InstallRow, success,
    utils::reverse_apt_style_url, warn, AILURUS, DRYRUN, WRITER,
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
    let mut all_is_err = true;
    let mut retry = 1;
    for (i, c) in urls.iter().enumerate() {
        let mut allow_resule = true;
        loop {
            if let Err(e) = download(
                c,
                client,
                filename.to_string(),
                download_dir,
                Some(&hash),
                opb.clone(),
                allow_resule,
            )
            .await
            {
                match e {
                    DownloadError::ChecksumMisMatch => {
                        allow_resule = false;
                        if retry == 3 {
                            break;
                        }
                        let s = format!("{c} checksum mismatch, retry {retry} times ...");
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
                        retry += 1;
                    }
                    _ => {
                        let mut s = format!("{e}");
                        if i < urls.len() - 1 {
                            s += ", try next url to download this package ...";
                        }
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
                        break;
                    }
                }
            } else {
                all_is_err = false;
                break;
            }
        }
    }

    if all_is_err {
        bail!("Can not download package: {filename}, Maybe your network connect is broken!")
    } else {
        Ok(())
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
            global_bar.inc(c.pure_download_size);
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

            tokio::fs::copy(&url, DOWNLOAD_DIR.join(filename))
                .await
                .context(format!("Can not find file: {}", url.display()))?;
        } else {
            let hash = c.checksum.as_ref().unwrap().to_owned();

            let download_dir_def = DOWNLOAD_DIR.clone();

            task.push(download_single_pkg(
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
    #[error("checksum mismatch")]
    ChecksumMisMatch,
    #[error("404 not found: {0}")]
    NotFound(String),
    #[error(transparent)]
    Anyhow(#[from] anyhow::Error),
    #[error(transparent)]
    JoinError(#[from] tokio::task::JoinError),
    #[error(transparent)]
    IOError(#[from] tokio::io::Error),
    #[error(transparent)]
    SendRequestError(#[from] reqwest::Error),
}

type DownloadResult<T> = std::result::Result<T, DownloadError>;

/// Download file
pub async fn download(
    url: &str,
    client: &Client,
    filename: String,
    dir: &Path,
    hash: Option<&str>,
    opb: OmaProgressBar,
    allow_resume: bool,
) -> DownloadResult<()> {
    let file = dir.join(&filename);
    let file_exist = file.exists();
    let file_size = file.metadata().ok().map(|x| x.len()).unwrap_or(0);

    let mut dest = None;
    let mut validator = None;

    // 如果要下载的文件已经存在，则验证 Checksum 是否正确，若正确则添加总进度条的进度，并返回
    // 如果不存在，则继续往下走
    if file_exist {
        if let Some(hash) = hash {
            let hash = hash.to_owned();

            let mut f = tokio::fs::OpenOptions::new()
                .create(true)
                .write(true)
                .read(true)
                .open(&file)
                .await?;

            let mut v = Checksum::from_sha256_str(&hash)?.get_validator();

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
                return Ok(());
            }

            dest = Some(f);
            validator = Some(v);
        }
    }

    // 写入进度条显示的信息
    let mut msg = opb.msg.unwrap_or_else(|| {
        let mut filename_split = filename.split('_');
        let name = filename_split.next();
        let version = filename_split.next().map(|x| x.replace("%3a", ":"));
        let arch = filename_split.next().map(|x| x.replace(".deb", ""));

        if name.and(version.clone()).and(arch.clone()).is_none() {
            filename.clone()
        } else {
            format!("{} {} ({})", name.unwrap(), version.unwrap(), arch.unwrap())
        }
    });

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
    let can_resume = match head.get(ACCEPT_RANGES) {
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
            .map_err(|e| anyhow!("{e}"))?
            .parse::<u64>()
            .map_err(|e| anyhow!("{e}"))?
    };

    let mut req = client.get(url);

    if can_resume && allow_resume {
        // 如果已存在的文件大小与要下载的文件大小相同，则返回 Checksum 不匹配
        // 因为刚刚已经验证过 checksum 了，如果函数能走到这里
        // 说明文件完整性是不一致的
        if total_size == file_size {
            return Err(DownloadError::ChecksumMisMatch);
        }

        // 发送 RANGE 的头，传入的是已经下载的文件的大小
        req = req.header(RANGE, format!("bytes={}-", file_size));
    }

    let resp = req.send().await?;

    if let Err(e) = resp.error_for_status_ref() {
        pb.finish_and_clear();
        match e.status() {
            Some(StatusCode::NOT_FOUND) => return Err(DownloadError::NotFound(url.to_string())),
            e => {
                return Err(DownloadError::Anyhow(anyhow!(
                    "Couldn't download URL: {}. Status Code: {:?}",
                    url,
                    e,
                )))
            }
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

    let mut option = tokio::fs::OpenOptions::new();
    option.create(true);
    option.write(true);
    option.read(true);

    let mut dest = if !allow_resume || !can_resume {
        // 如果不能 resume，则加入 truncate 这个 flag，告诉内核截断文件
        // 并把文件长度设置为 0
        let f = tokio::fs::OpenOptions::new()
            .truncate(true)
            .create(true)
            .write(true)
            .read(true)
            .open(&file)
            .await?;
        f.set_len(0).await?;

        f
    } else if let Some(dest) = dest {
        dest
    } else {
        tokio::fs::OpenOptions::new()
            .create(true)
            .write(true)
            .read(true)
            .open(&file)
            .await?
    };

    // 把文件指针移动到末尾
    dest.seek(SeekFrom::End(0)).await?;

    // 下载！
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
    dest.shutdown().await?;

    // 最后看看 chekcsum 验证是否通过
    if let Some(v) = validator {
        if !v.finish() {
            if let Some(ref global_bar) = opb.global_bar {
                global_bar.set_position(global_bar.position() - pb.position());
            }
            pb.reset();
            return Err(DownloadError::ChecksumMisMatch);
        }
    }

    pb.finish_and_clear();

    Ok(())
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
