use std::{
    io::{Read, Write},
    path::Path,
    time::Duration,
};

use anyhow::{anyhow, Context, Result};
use indicatif::{ProgressBar, ProgressStyle};
use log::info;
use progress_streams::ProgressReader;
use reqwest::blocking::Client;
use sha2::{Digest, Sha256};

use crate::{msg, update::DOWNLOAD_DIR, WRITER};

/// Download a package
pub fn download_package(
    url: &str,
    download_dir: Option<&str>,
    client: &Client,
    hash: &str,
    version: &str,
) -> Result<String> {
    fn download_inner(
        download_dir: Option<&str>,
        filename: &str,
        url: &str,
        client: &Client,
    ) -> Result<()> {
        info!(
            "Downloading {} to dir {}",
            filename,
            download_dir.unwrap_or(DOWNLOAD_DIR)
        );

        if download_dir.is_none() {
            std::fs::create_dir_all(DOWNLOAD_DIR)?;
        }

        download(
            url,
            client,
            filename,
            Path::new(download_dir.unwrap_or(DOWNLOAD_DIR)),
        )?;

        // if download_dir.is_none() {
        //     std::fs::create_dir_all(DOWNLOAD_DIR)?;
        //     std::fs::write(p, v)?;
        // } else {
        //     std::fs::write(Path::new(download_dir.unwrap()).join(filename), v)?;
        // }

        Ok(())
    }

    let filename = url
        .split('/')
        .last()
        .take()
        .context(format!("Can not parse url {url}"))?;

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

    let p = Path::new(download_dir.unwrap_or(DOWNLOAD_DIR)).join(&filename);
    if p.exists() {
        let mut f = std::fs::File::open(&p)?;
        let mut buf = Vec::new();
        f.read_to_end(&mut buf)?;

        if checksum(&buf, hash).is_err() {
            download_inner(download_dir, &filename, url, client)?;
        } else {
            return Ok(filename.to_string());
        }
    } else {
        download_inner(download_dir, &filename, url, client)?;
    }

    Ok(filename.to_string())
}

/// Download file to buffer
pub fn download(url: &str, client: &Client, filename: &str, dir: &Path) -> Result<Vec<u8>> {
    msg!("Getting {} ...", filename);
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

    let p = dir.join(filename);

    let mut v = if p.is_file() {
        let mut f = std::fs::File::open(&p)?;
        let mut buf = Vec::new();
        f.read_to_end(&mut buf)?;
        let len = buf.len();

        client
            .get(url)
            .header("Range", format!("bytes={}-", len))
            .send()?
            .error_for_status()?
    } else {
        client.get(url).send()?.error_for_status()?
    };

    // let mut v = client.get(url).send()?.error_for_status()?;

    let length = v.content_length().unwrap_or(0);
    let pb = ProgressBar::new(length);
    pb.set_style(barsty);
    pb.enable_steady_tick(Duration::from_millis(50));

    let mut reader = ProgressReader::new(&mut v, |progress: usize| {
        pb.inc(progress as u64);
    });
    
    let mut f = std::fs::File::create(p)?;
    std::io::copy(&mut reader, &mut f)?;
    let mut buf = Vec::new();
    f.read_to_end(&mut buf)?;

    pb.finish_and_clear();

    Ok(buf)
}

pub fn checksum(buf: &[u8], hash: &str) -> Result<()> {
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

    Ok(())
}
