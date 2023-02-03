use std::{
    io::{Read, Write},
    path::Path,
};

use anyhow::{anyhow, Context, Result};
use log::info;
use reqwest::blocking::Client;
use sha2::{Digest, Sha256};

use crate::update::DOWNLOAD_DIR;

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
        p: &Path,
        filename: &str,
        url: &str,
        client: &Client,
    ) -> Result<()> {
        info!(
            "Downloading {} to dir {}",
            filename,
            download_dir.unwrap_or(DOWNLOAD_DIR)
        );
        let v = download(url, client)?;

        if download_dir.is_none() {
            std::fs::create_dir_all(DOWNLOAD_DIR)?;
            std::fs::write(p, v)?;
        } else {
            std::fs::write(Path::new(download_dir.unwrap()).join(filename), v)?;
        }

        Ok(())
    }

    let filename = url
        .split('/')
        .last()
        .take()
        .context(format!("Can not parse url {url}"))?;

    // sb apt 会把下载的文件重命名成 url 网址的样子，为保持兼容这里也这么做
    let mut filename_split = filename.split("_");
    let package = filename_split.next().take().context("Can not parse filename")?;
    let arch_deb = filename_split.nth(1).take().context("Can not parse version")?;

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
            download_inner(download_dir, &p, &filename, url, client)?;
        } else {
            return Ok(filename.to_string());
        }
    } else {
        download_inner(download_dir, &p, &filename, url, client)?;
    }

    Ok(filename.to_string())
}

/// Download file to buffer
pub fn download(url: &str, client: &Client) -> Result<Vec<u8>> {
    let v = client
        .get(url)
        .send()?
        .error_for_status()?
        .bytes()?
        .to_vec();

    Ok(v)
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
