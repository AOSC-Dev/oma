use std::{
    io::{Read, Write},
    path::Path,
};

use tokio::task::spawn_blocking;

use anyhow::{anyhow, Context, Result};
use indicatif::{MultiProgress, ProgressBar, ProgressStyle};
use log::info;
use reqwest::Client;
use sha2::{Digest, Sha256};
use tokio::{
    fs,
    io::{AsyncReadExt, AsyncWriteExt},
};

use crate::{msg, update::DOWNLOAD_DIR, WRITER};

/// Download a package
pub async fn download_package(
    path: String,
    mirrors: Vec<String>,
    download_dir: Option<&str>,
    client: &Client,
    hash: String,
    version: String,
) -> Result<String> {
    async fn download_inner(
        download_dir: Option<&str>,
        filename: &str,
        url: &str,
        client: &Client,
        hash: &str,
    ) -> Result<()> {
        info!(
            "Downloading {} to dir {}",
            filename,
            download_dir.unwrap_or(DOWNLOAD_DIR)
        );

        if download_dir.is_none() {
            tokio::fs::create_dir_all(DOWNLOAD_DIR).await?;
        }

        download(
            url,
            client,
            filename,
            Path::new(download_dir.unwrap_or(DOWNLOAD_DIR)),
            None,
            Some(hash),
        )
        .await?;

        Ok(())
    }

    let filename = path
        .split('/')
        .last()
        .take()
        .context(format!("Can not parse url {path}"))?;

    // sb apt 会把下载的文件重命名成 url 网址的样子，为保持兼容这里也这么做
    let mut filename_split = filename.split("_");
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

    let version = version.replace(":", "%3a");
    let filename = format!("{package}_{version}_{arch_deb}");

    let mut all_is_err = true;

    let p = Path::new(download_dir.unwrap_or(DOWNLOAD_DIR)).join(&filename);
    if p.exists() {
        let mut f = std::fs::File::open(&p)?;
        let mut buf = Vec::new();
        f.read_to_end(&mut buf)?;

        if checksum(&buf, &hash).is_err() {
            for i in mirrors {
                if download_inner(
                    download_dir,
                    &filename,
                    &format!("{i}/{path}"),
                    client,
                    &hash,
                )
                .await
                .is_ok()
                {
                    all_is_err = false;
                    break;
                }
            }
        } else {
            return Ok(filename.to_string());
        }
    } else {
        for i in mirrors {
            if download_inner(
                download_dir,
                &filename,
                &format!("{i}/{path}"),
                client,
                &hash,
            )
            .await
            .is_ok()
            {
                all_is_err = false;
                break;
            }
        }
    }

    if all_is_err {
        return Err(anyhow!(
            "Can not download package: {}, Maybe your network connect is broken!",
            filename
        ));
    }

    Ok(filename.to_string())
}

/// Download file to buffer
pub async fn download(
    url: &str,
    client: &Client,
    filename: &str,
    dir: &Path,
    msg: Option<&str>,
    hash: Option<&str>,
) -> Result<Vec<u8>> {
    msg!("{}", msg.unwrap_or(filename));
    let bar_template = {
        let max_len = WRITER.get_max_len();
        if max_len < 90 {
            " {wide_msg} {total_bytes:>10} {binary_bytes_per_sec:>12} {eta:>4} {percent:>3}%"
        } else {
            " {msg:<48} {total_bytes:>10} {binary_bytes_per_sec:>12} {eta:>4} [{wide_bar:.white/black}] {percent:>3}%"
        }
    };

    let barsty = ProgressStyle::default_bar()
        .template(bar_template)?
        .progress_chars("=>-");

    // let p = dir.join(filename);
    let total_size = {
        let resp = client.get(url).send().await?;
        if resp.status().is_success() {
            let res = resp.content_length().unwrap_or(0);

            res
        } else {
            return Err(anyhow!(
                "Couldn't download URL: {}. Error: {:?}",
                url,
                resp.status(),
            ));
        }
    };

    let request = client.get(url);
    let pb = ProgressBar::new(total_size);
    pb.set_style(barsty);

    let file = dir.join(filename);

    if file.exists() {
        if let Some(hash) = hash {
            let mut f = std::fs::File::open(&file)?;
            let mut buf = Vec::new();
            let buf_clone = buf.clone();
            f.read_to_end(&mut buf)?;
            let hash = hash.to_owned();

            let result = spawn_blocking(move || checksum(&buf_clone, &hash)).await?;

            if result.is_ok() {
                return Ok(buf);
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
    }

    dest.flush().await?;
    drop(dest);

    let buf = if let Some(hash) = hash {
        let mut dest = tokio::fs::File::open(&file).await?;
        let mut buf = Vec::new();
        dest.read_to_end(&mut buf).await?;
        let buf_clone = buf.clone();
        let hash_clone = hash.to_string();

        let result = spawn_blocking(move || checksum(&buf_clone.clone(), &hash_clone)).await?;

        if result.is_err() {
            return Err(anyhow!(
                "Couldn't download URL: {}. Error: {:?}",
                url,
                result.err()
            ));
        }

        buf
    } else {
        let mut dest = tokio::fs::File::open(&file).await?;
        let mut buf = Vec::new();
        dest.read_to_end(&mut buf).await?;

        buf
    };

    Ok(buf)
}

pub fn checksum(buf: &[u8], hash: &str) -> Result<bool> {
    let mut hasher = Sha256::new();
    hasher.write_all(buf)?;
    let buf_hash = hasher.finalize();
    let buf_hash = format!("{buf_hash:2x}");

    if hash != buf_hash {
        return Err(anyhow!(
            "Checksum mismatch. Expected {}, got {}",
            hash,
            buf_hash
        ));
    }

    Ok(true)
}
