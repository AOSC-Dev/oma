use std::{
    path::{Path, PathBuf},
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
    time::Duration,
};

use console::style;
use futures::StreamExt;
use tokio::{runtime::Runtime, task::spawn_blocking};

use anyhow::{anyhow, bail, Context, Result};
use indicatif::{MultiProgress, ProgressBar, ProgressStyle};
use reqwest::Client;
use tokio::{fs, io::AsyncWriteExt};

use crate::{
    checksum::Checksum, cli::gen_prefix, db::DOWNLOAD_DIR, info, oma::InstallRow, success,
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
        .context("Can not parse filename")?;

    let arch_deb = filename_split
        .nth(1)
        .take()
        .context("Can not parse version")?;

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
    for (i, c) in urls.iter().enumerate() {
        if let Err(e) = download(
            c,
            client,
            filename.to_string(),
            download_dir,
            Some(&hash),
            opb.clone(),
        )
        .await
        {
            let mut s = format!("{e}");
            if i < urls.len() - 1 {
                s += ", try next url to download this package ...";
            }
            if let Some(ref gpb) = opb.global_bar {
                gpb.println(format!(
                    "{}{s}",
                    gen_prefix(&style("WARNING").yellow().bold().to_string())
                ));
            } else {
                warn!("{s}");
            }
        } else {
            all_is_err = false;
            break;
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
    global_bar.set_message(style("Progress").bold().to_string());

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
    mbc: Arc<MultiProgress>,
    progress: Option<(usize, usize)>,
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

/// Download file
pub async fn download(
    url: &str,
    client: &Client,
    filename: String,
    dir: &Path,
    hash: Option<&str>,
    opb: OmaProgressBar,
) -> Result<()> {
    let is_send = Arc::new(AtomicBool::new(false));
    let is_send_clone = is_send.clone();

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

    let pb = spawn_blocking(move || {
        // 若请求头的速度太慢，会看到假进度条直到拿到头的信息
        let pb = mbcc.add(ProgressBar::new_spinner());
        pb.set_message(format!("{progress_clone}{msg_clone}"));

        oma_spinner(&pb);

        while !is_send.load(Ordering::Relaxed) {}

        pb.finish_and_clear();
    });

    let (total_size, resp) = {
        let resp = client.get(url).send().await?;
        if resp.status().is_success() {
            is_send_clone.store(true, Ordering::Relaxed);
            (resp.content_length().unwrap_or(0), resp)
        } else {
            is_send_clone.store(true, Ordering::Relaxed);
            return Err(anyhow!(
                "Couldn't download URL: {}. Error: {:?}",
                url,
                resp.status(),
            ));
        }
    };

    pb.await?;

    let pb = opb.mbc.add(ProgressBar::new(total_size));
    pb.set_style(oma_style_pb(false)?);

    if console::measure_text_width(&msg) > 60 {
        msg = console::truncate_str(&msg, 57, "...").to_string();
    }

    let progress = if let Some((count, len)) = opb.progress {
        format!("({count}/{len}) ")
    } else {
        "".to_string()
    };

    pb.set_message(format!("{progress}{msg}"));
    pb.enable_steady_tick(Duration::from_millis(100));

    let file = dir.join(filename);

    if file.exists() {
        if let Some(hash) = hash {
            let hash = hash.to_owned();
            let file_clone = file.clone();

            let result = spawn_blocking(move || {
                Checksum::from_sha256_str(&hash).and_then(|x| x.cmp_file(&file_clone))
            })
            .await??;

            if result {
                pb.finish_and_clear();
                if let Some(ref global_bar) = opb.global_bar {
                    global_bar.inc(total_size);
                }
                return Ok(());
            }
        }
    }

    let mut source = resp;

    let mut dest = fs::File::create(&file).await?;
    while let Some(chunk) = source.chunk().await? {
        dest.write_all(&chunk).await?;
        pb.inc(chunk.len() as u64);

        if let Some(ref global_bar) = opb.global_bar {
            global_bar.inc(chunk.len() as u64);
        }
    }

    dest.flush().await?;
    drop(dest);

    if let Some(hash) = hash {
        pb.set_message("Checking integrity ...");
        let hash = hash.to_string();
        let result = spawn_blocking(move || {
            Checksum::from_sha256_str(&hash).and_then(|x| x.cmp_file(&file))
        })
        .await??;

        if !result {
            return Err(anyhow!(
                "Url: {url} checksum mismatch! Please check your network connection!"
            ));
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
                " {wide_msg} {bytes}".to_owned()
                    + &style("/").bold().to_string()
                    + "{total_bytes:>10} {eta:>4} {percent:>3}%"
            } else {
                " {msg:<48.blue.bold} {bytes:.blue.bold}".to_owned() + &style("/").bold().blue().to_string() + "{total_bytes:>10.blue.bold} {eta:>4.blue.bold} [{wide_bar:.blue.bold}] {percent:>3.blue}" + &style("%").blue().to_string()
            }
        } else if max_len < 90 {
            " {wide_msg} {total_bytes:>10} {binary_bytes_per_sec:>12} {eta:>4} {percent:>3}%"
                .to_owned()
        } else {
            " {msg:<48} {total_bytes:>10} {binary_bytes_per_sec:>12} {eta:>4} [{wide_bar:.white/black}] {percent:>3}%".to_owned()
        }
    };

    let barsty = ProgressStyle::default_bar()
        .template(&bar_template)?
        .progress_chars("=>-");

    Ok(barsty)
}
