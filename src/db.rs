use core::panic;
use std::{collections::HashMap, io::Read, path::Path, sync::Arc, time::Duration};

use anyhow::{anyhow, bail, Context, Result};
use apt_sources_lists::*;
use console::style;
use flate2::bufread::GzDecoder;
use futures::StreamExt;
use indexmap::IndexMap;
use indicatif::{MultiProgress, ProgressBar};
use reqwest::{Client, Url};
use serde::Deserialize;
use tokio::{io::AsyncReadExt, task::spawn_blocking};
use xz2::read::XzDecoder;

use crate::{
    action::InstallRow,
    checksum::Checksum,
    download::{download, download_package, oma_style_pb, OmaProgressBar},
    error, info, success,
    utils::get_arch_name,
    verify,
};

pub const APT_LIST_DISTS: &str = "/var/lib/apt/lists";
pub const DOWNLOAD_DIR: &str = "/var/cache/apt/archives";
const MIRROR: &str = "/usr/share/distro-repository-data/mirrors.yml";

struct FileBuf(Vec<u8>);

#[derive(Debug)]
struct FileName(String);

impl FileName {
    fn new(s: &str) -> Result<Self> {
        let url = reqwest::Url::parse(s)?;
        let scheme = url.scheme();
        let url = s
            .strip_prefix(&format!("{scheme}://"))
            .ok_or_else(|| anyhow!("Can not get url without url scheme"))?
            .replace('/', "_");

        Ok(FileName(url))
    }
}

async fn download_db(
    url: String,
    client: &Client,
    typ: String,
    opb: OmaProgressBar,
    i: usize,
) -> Result<(FileName, FileBuf, usize)> {
    let filename = FileName::new(&url)?.0;
    let url_short = get_url_short_and_branch(&url).await?;

    let mut opb = opb;

    opb.msg = Some(format!("{url_short} {typ}"));
    let opb = opb.clone();

    download(
        &url,
        client,
        filename.to_string(),
        Path::new(APT_LIST_DISTS),
        None,
        opb,
    )
    .await?;

    let mut v = tokio::fs::File::open(Path::new(APT_LIST_DISTS).join(filename)).await?;
    let mut buf = Vec::new();
    v.read_to_end(&mut buf).await?;

    Ok((FileName::new(&url)?, FileBuf(buf), i))
}

#[derive(Deserialize)]
struct MirrorMapItem {
    url: String,
}

async fn get_url_short_and_branch(url: &str) -> Result<String> {
    let url = Url::parse(url)?;
    let host = url.host_str().context("Can not parse {url} host!")?;
    let schema = url.scheme();
    let branch = url
        .path()
        .split('/')
        .nth_back(1)
        .context("Can not get {url} branch!")?;
    let url = format!("{schema}://{host}/");

    // MIRROR 文件为 AOSC 独有，为兼容其他 .deb 系统，这里不直接返回错误
    if let Ok(mut mirror_map_f) = tokio::fs::File::open(MIRROR).await {
        let mut buf = Vec::new();
        mirror_map_f.read_to_end(&mut buf).await?;

        let mirror_map: HashMap<String, MirrorMapItem> = serde_yaml::from_slice(&buf)?;

        for (k, v) in mirror_map.iter() {
            let mirror_url = Url::parse(&v.url)?;
            let mirror_url_host = mirror_url
                .host_str()
                .context("Can not get host str from mirror map!")?;
            let schema = mirror_url.scheme();
            let mirror_url = format!("{schema}://{mirror_url_host}/");

            if mirror_url == url {
                return Ok(format!("{k}:{branch}"));
            }
        }
    }

    return Ok(format!("{host}:{branch}"));
}

#[derive(Debug)]
struct InReleaseParser {
    _source: Vec<IndexMap<String, String>>,
    checksums: Vec<ChecksumItem>,
}

#[derive(Debug, Clone)]
struct ChecksumItem {
    name: String,
    size: u64,
    checksum: String,
    file_type: DistFileType,
}

#[derive(Debug, PartialEq, Clone)]
enum DistFileType {
    BinaryContents,
    Contents,
    CompressContents,
    PackageList,
    CompressPackageList,
}

impl InReleaseParser {
    fn new(p: &Path) -> Result<Self> {
        let mut f = std::fs::File::open(p)?;
        let mut s = String::new();

        f.read_to_string(&mut s)?;

        let s = if s.starts_with("-----BEGIN PGP SIGNED MESSAGE-----") {
            verify::verify(&s)?
        } else {
            s
        };

        let source = debcontrol_from_str(&s)?;
        let sha256 = source
            .first()
            .and_then(|x| x.get("SHA256"))
            .take()
            .context("source is empty")?;

        let mut checksums = sha256.split('\n');

        // remove first item, It's empty
        checksums.next();

        let mut checksums_res = vec![];

        for i in checksums {
            let checksum = i.split_whitespace().collect::<Vec<_>>();
            let checksum = (checksum[2], checksum[1], checksum[0]);
            checksums_res.push(checksum);
        }

        let arch = get_arch_name().ok_or_else(|| anyhow!("Can not get arch!"))?;

        let mut res = vec![];

        let c = checksums_res
            .into_iter()
            .filter(|(name, _, _)| name.contains("all") || name.contains(arch));

        for i in c {
            let t = if i.0.contains("BinContents") {
                DistFileType::BinaryContents
            } else if i.0.contains("/Contents-") && i.0.contains('.') {
                DistFileType::CompressContents
            } else if i.0.contains("/Contents-") && !i.0.contains('.') {
                DistFileType::Contents
            } else if i.0.contains("Packages") && !i.0.contains('.') {
                DistFileType::PackageList
            } else if i.0.contains("Packages") && i.0.contains('.') {
                DistFileType::CompressPackageList
            } else {
                panic!("I Dont known why ...")
            };

            res.push(ChecksumItem {
                name: i.0.to_owned(),
                size: i.1.parse::<u64>()?,
                checksum: i.2.to_owned(),
                file_type: t,
            })
        }

        Ok(Self {
            _source: source,
            checksums: res,
        })
    }
}

fn debcontrol_from_str(s: &str) -> Result<Vec<IndexMap<String, String>>> {
    let mut res = vec![];

    let debcontrol = debcontrol::parse_str(s).map_err(|e| anyhow!("{}", e))?;

    for i in debcontrol {
        let mut item = IndexMap::new();
        let field = i.fields;

        for j in field {
            item.insert(j.name.to_string(), j.value.to_string());
        }

        res.push(item);
    }

    Ok(res)
}

/// Get /etc/apt/sources.list and /etc/apt/sources.list.d
pub fn get_sources() -> Result<Vec<SourceEntry>> {
    let mut res = Vec::new();
    let list = SourcesLists::scan()?;

    for file in list.iter() {
        for i in &file.lines {
            if let SourceLine::Entry(entry) = i {
                res.push(entry.to_owned());
            }
        }
    }

    // AOSC OS/Retro 以后也许会支持使用光盘源安装软件包，但目前因为没有实例，所以无法测试
    // 因此 Omakase 暂不支持 cdrom:// 源的安装
    let cdrom = res
        .iter()
        .filter(|x| x.url().starts_with("cdrom://"))
        .collect::<Vec<_>>();

    for i in &cdrom {
        error!("Omakase does not support cdrom protocol in url: {}", i.url);
    }

    if !cdrom.is_empty() {
        bail!("Omakase unsupport some mirror.");
    }

    Ok(res)
}

/// Download packages
pub async fn packages_download(
    list: &[InstallRow],
    client: &Client,
    limit: Option<usize>,
) -> Result<()> {
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
    global_bar.enable_steady_tick(Duration::from_millis(1000));
    global_bar.set_message(style("Progress").bold().to_string());

    let list_len = list.len();

    for (i, c) in list.iter().enumerate() {
        let mbc = mb.clone();

        if let Some(ref checksum) = c.checksum {
            task.push(download_package(
                c.pkg_urls.clone(),
                client,
                checksum.clone(),
                c.new_version.clone(),
                OmaProgressBar::new(
                    None,
                    Some((i + 1, list_len)),
                    Some(mbc.clone()),
                    Some(global_bar.clone()),
                ),
            ));
        } else {
            // 如果没有验证码，则应该是本地安装得来的，使用 apt 来处理这些软件包
            info!("Use apt to handle url {:?}", c.pkg_urls.first());
        }
    }
    // 默认限制一次最多下载八个包，减少服务器负担
    let stream = futures::stream::iter(task).buffer_unordered(limit.unwrap_or(4));
    let res = stream.collect::<Vec<_>>().await;

    global_bar.finish_and_clear();

    // 遍历结果看是否有下载出错
    for i in res {
        i?;
    }

    Ok(())
}

// Update database
pub async fn update_db(
    sources: &[SourceEntry],
    client: &Client,
    limit: Option<usize>,
) -> Result<()> {
    info!("Refreshing local repository metadata ...");
    let dist_urls = sources.iter().map(|x| x.dist_path()).collect::<Vec<_>>();
    let dists_in_releases = dist_urls
        .clone()
        .into_iter()
        .map(|x| format!("{}/{}", x, "InRelease"))
        .collect::<Vec<_>>();

    let mut tasks = Vec::new();

    let mb = Arc::new(MultiProgress::new());

    for (i, c) in dists_in_releases.iter().enumerate() {
        tasks.push(download_db(
            c.to_string(),
            client,
            "InRelease".to_owned(),
            OmaProgressBar::new(
                None,
                Some((i + 1, dists_in_releases.len())),
                Some(mb.clone()),
                None,
            )
            .clone(),
            i,
        ));
    }

    // 默认限制一次最多下载八个数据文件，减少服务器负担
    let stream = futures::stream::iter(tasks).buffer_unordered(limit.unwrap_or(4));
    let res = stream.collect::<Vec<_>>().await;

    let mut dist_files = vec![];

    for i in res {
        dist_files.push(i?);
    }

    let components = sources
        .iter()
        .map(|x| x.components.to_owned())
        .collect::<Vec<_>>();

    for (name, file, index) in dist_files {
        let p = Path::new(APT_LIST_DISTS).join(name.0);

        if !p.exists() || !p.is_file() {
            tokio::fs::create_dir_all(APT_LIST_DISTS).await?;
            tokio::fs::write(&p, &file.0).await?;
        } else {
            let mut f = tokio::fs::File::open(&p).await?;
            let mut buf = Vec::new();
            f.read_to_end(&mut buf).await?;

            if buf != file.0 {
                tokio::fs::write(&p, &file.0).await?;
            }
        }

        let in_release = InReleaseParser::new(&p)?;

        let checksums = in_release
            .checksums
            .iter()
            .filter(|x| components[index].contains(&x.name.split('/').next().unwrap().to_string()))
            .collect::<Vec<_>>();

        let mut handle = vec![];

        let mut total = 0;

        for i in &checksums {
            if i.file_type == DistFileType::CompressContents
                || i.file_type == DistFileType::CompressPackageList
            {
                handle.push(i);
                total += i.size;
            }
        }

        let mb = Arc::new(MultiProgress::new());
        let global_bar = mb.insert(0, ProgressBar::new(total));
        global_bar.set_style(oma_style_pb(true)?);
        global_bar.enable_steady_tick(Duration::from_millis(1000));
        global_bar.set_message("Progress");

        let mut task = vec![];

        let len = handle.len();

        for (i, c) in handle.iter().enumerate() {
            let not_compress_file = c.name.replace(".xz", "").replace(".gz", "");
            let file_name = FileName::new(&format!(
                "{}/{}",
                dist_urls.get(index).unwrap(),
                not_compress_file
            ))?;

            let p = Path::new(APT_LIST_DISTS).join(&file_name.0);

            let typ = match c.file_type {
                DistFileType::CompressContents => "Contents",
                DistFileType::CompressPackageList => "Package List",
                _ => unreachable!(),
            };

            if p.exists() {
                let checksums_index = checksums
                    .iter()
                    .position(|x| x.name == not_compress_file)
                    .unwrap();

                let hash = checksums[checksums_index].checksum.to_owned();

                let opb = OmaProgressBar::new(
                    None,
                    Some((i + 1, len)),
                    Some(mb.clone()),
                    Some(global_bar.clone()),
                )
                .clone();

                let hash_clone = hash.clone();
                let result = spawn_blocking(move || {
                    Checksum::from_sha256_str(&hash_clone).and_then(|x| x.cmp_file(&p))
                })
                .await??;

                if !result {
                    task.push(download_and_extract(
                        &dist_urls[index],
                        c,
                        client,
                        file_name.0,
                        typ,
                        opb,
                    ));
                } else {
                    continue;
                }
            } else {
                let opb = OmaProgressBar::new(
                    None,
                    Some((i + 1, len)),
                    Some(mb.clone()),
                    Some(global_bar.clone()),
                )
                .clone();

                task.push(download_and_extract(
                    &dist_urls[index],
                    c,
                    client,
                    file_name.0,
                    typ,
                    opb,
                ));
            }
        }

        // 默认限制一次最多下载八个数据文件，减少服务器负担
        let stream = futures::stream::iter(task).buffer_unordered(limit.unwrap_or(4));
        let res = stream.collect::<Vec<_>>().await;

        for i in res {
            i?;
        }

        global_bar.finish_and_clear();
    }

    success!("Package database already newest.");

    Ok(())
}

/// Download and extract package list database
async fn download_and_extract(
    dist_url: &str,
    i: &ChecksumItem,
    client: &Client,
    not_compress_file: String,
    typ: &str,
    opb: OmaProgressBar,
) -> Result<()> {
    let (name, buf, _) = download_db(
        format!("{}/{}", dist_url, i.name),
        client,
        typ.to_owned(),
        opb,
        0,
    )
    .await?;

    let p = Path::new(APT_LIST_DISTS).join(&name.0);

    let ic = i.clone();
    let result = spawn_blocking(move || {
        Checksum::from_sha256_str(&ic.checksum).and_then(|x| x.cmp_file(p.clone().as_path()))
    })
    .await??;

    if !result {
        bail!("Download {typ} Checksum mismatch! Please check your network connection.")
    }

    let buf = decompress(&buf.0, &name.0)?;
    let p = Path::new(APT_LIST_DISTS).join(not_compress_file);
    std::fs::write(p, buf)?;

    Ok(())
}

/// Extract database
fn decompress(buf: &[u8], name: &str) -> Result<Vec<u8>> {
    let buf = if name.ends_with(".gz") {
        let mut decompressor = GzDecoder::new(buf);
        let mut buf = Vec::new();
        decompressor.read_to_end(&mut buf)?;

        buf
    } else if name.ends_with(".xz") {
        let mut decompressor = XzDecoder::new(buf);
        let mut buf = Vec::new();
        decompressor.read_to_end(&mut buf)?;

        buf
    } else {
        return Err(anyhow!("Unsupported compression format."));
    };

    Ok(buf)
}
