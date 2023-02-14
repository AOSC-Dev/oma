use std::{path::Path, sync::Arc, time::Duration};

use console::style;
use tokio::task::spawn_blocking;

use anyhow::{anyhow, Context, Result};
use indicatif::{MultiProgress, ProgressBar, ProgressStyle};
use reqwest::Client;
use tokio::{fs, io::AsyncWriteExt};

use crate::{checksum::Checksum, db::DOWNLOAD_DIR, warn, WRITER};

/// Download a package
pub async fn download_package(
    urls: Vec<String>,
    client: &Client,
    hash: String,
    version: String,
    opb: OmaProgressBar,
) -> Result<()> {
    let filename = urls
        .first()
        .and_then(|x| x.split('/').last())
        .take()
        .context("URLs is none or Invaild URL")?;

    // sb apt 会把下载的文件重命名成 url 网址的样子，为保持兼容这里也这么做
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
    let filename = format!("{package}_{version}_{arch_deb}");

    let p = Path::new(DOWNLOAD_DIR).join(&filename);
    if p.exists() {
        let hash_clone = hash.clone();
        let result = spawn_blocking(move || {
            Checksum::from_sha256_str(&hash_clone).and_then(|x| x.cmp_file(&p))
        })
        .await??;

        if !result {
            try_download(urls, client, &filename, hash, opb).await?;
        } else {
            return Ok(());
        }
    } else {
        try_download(urls, client, &filename, hash, opb).await?;
    }

    Ok(())
}

async fn try_download(
    urls: Vec<String>,
    client: &Client,
    filename: &String,
    hash: String,
    opb: OmaProgressBar,
) -> Result<()> {
    let mut all_is_err = true;
    for (i, c) in urls.iter().enumerate() {
        if download(
            c,
            client,
            filename.to_string(),
            Path::new(DOWNLOAD_DIR),
            Some(&hash),
            opb.clone(),
        )
        .await
        .is_ok()
        {
            all_is_err = false;
            break;
        } else if i < urls.len() - 1 {
            warn!("Download {c} failed, try next url to download this package ...");
        }
    }

    if all_is_err {
        Err(anyhow!(
            "Can not download package: {}, Maybe your network connect is broken!",
            filename
        ))
    } else {
        Ok(())
    }
}

#[derive(Clone)]
pub struct OmaProgressBar {
    pub msg: Option<String>,
    mbc: Option<Arc<MultiProgress>>,
    progress: Option<(usize, usize)>,
    global_bar: Option<ProgressBar>,
}

impl OmaProgressBar {
    pub fn new(
        msg: Option<String>,
        progress: Option<(usize, usize)>,
        mbc: Option<Arc<MultiProgress>>,
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
    let total_size = {
        let resp = client.get(url).send().await?;
        if resp.status().is_success() {
            resp.content_length().unwrap_or(0)
        } else {
            return Err(anyhow!(
                "Couldn't download URL: {}. Error: {:?}",
                url,
                resp.status(),
            ));
        }
    };

    let mut is_mb = false;

    let request = client.get(url);
    let pb = if let Some(mbc) = opb.mbc {
        is_mb = true;
        let pb = mbc.add(ProgressBar::new(total_size));
        let barsty = oma_style_pb(false)?;
        pb.set_style(barsty);

        pb
    } else {
        ProgressBar::new_spinner()
    };

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

    if console::measure_text_width(&msg) > 60 {
        msg = console::truncate_str(&msg, 57, "...").to_string();
    }

    let progress = if let Some((count, len)) = opb.progress {
        format!("({count}/{len}) ")
    } else {
        "".to_string()
    };

    pb.set_message(format!("{progress}{msg}"));
    pb.enable_steady_tick(Duration::from_millis(1000));

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
                return Ok(());
            } else {
                tokio::fs::remove_file(&file).await?;
            }
        }
    }

    let mut source = request.send().await?;

    let mut dest = fs::File::create(&file).await?;
    while let Some(chunk) = source.chunk().await? {
        dest.write_all(&chunk).await?;
        pb.inc(chunk.len() as u64);

        if let Some(ref global_bar) = opb.global_bar {
            global_bar.inc(chunk.len() as u64);
        }
    }

    if is_mb {
        pb.finish_and_clear();
    } else {
        pb.finish();
    }

    dest.flush().await?;
    drop(dest);

    if let Some(hash) = hash {
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

    Ok(())
}

pub fn oma_style_pb(is_global: bool) -> Result<ProgressStyle> {
    let bar_template = {
        let max_len = WRITER.get_max_len();
        if is_global {
            if max_len < 90 {
                " {wide_msg:.blue.bold} {total_bytes:>10:.blue.bold} {binary_bytes_per_sec:>12.blue.bold} {eta:>4.blue.bold} {percent%:>3.blue}".to_owned() + &style("%").blue().to_string()
            } else {
                " {msg:<48.blue.bold} {total_bytes:>10.blue.bold} {binary_bytes_per_sec:>12.blue.bold} {eta:>4.blue.bold} [{wide_bar:.blue.bold}] {percent:>3.blue}".to_owned() + &style("%").blue().to_string()
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
