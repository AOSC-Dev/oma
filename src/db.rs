use core::panic;
use std::{
    collections::HashMap,
    io::Read,
    path::{Path, PathBuf},
    sync::Arc,
    time::Duration,
};

use anyhow::{anyhow, bail, Context, Result};
use apt_sources_lists::*;
use flate2::bufread::GzDecoder;
use futures::StreamExt;
use indexmap::IndexMap;
use indicatif::{MultiProgress, ProgressBar};
use reqwest::{Client, Url};
use serde::Deserialize;
use sha2::{Digest, Sha256};
use std::io::Write;
use tokio::io::AsyncReadExt;
use xz2::read::XzDecoder;

use crate::{
    download::{download, download_package, oma_style_pb},
    info,
    success,
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
    global_bar: Option<ProgressBar>,
    progress: Option<(usize, usize)>,
    mbc: Option<Arc<MultiProgress>>,
) -> Result<(FileName, FileBuf)> {
    let filename = FileName::new(&url)?.0;

    let url_short = get_url_short_and_branch(&url)?;

    download(
        &url,
        client,
        filename.to_string(),
        Path::new(APT_LIST_DISTS),
        Some(format!("Getting {url_short} {typ} database ...")),
        None,
        mbc,
        progress,
        global_bar,
    )
    .await?;

    let mut v = tokio::fs::File::open(Path::new(APT_LIST_DISTS).join(filename)).await?;
    let mut buf = Vec::new();
    v.read_to_end(&mut buf).await?;

    Ok((FileName::new(&url)?, FileBuf(buf)))
}

#[derive(Deserialize)]
struct MirrorMapItem {
    url: String,
}

fn get_url_short_and_branch(url: &str) -> Result<String> {
    let url = Url::parse(&url)?;
    let host = url.host_str().context("Can not parse {url} host!")?;
    let schema = url.scheme();
    let branch = url
        .path()
        .split('/')
        .nth_back(1)
        .context("Can not get {url} branch!")?;
    let url = format!("{schema}://{host}/");

    let mut mirror_map_f = std::fs::File::open(MIRROR)?;
    let mut buf = Vec::new();
    mirror_map_f.read_to_end(&mut buf)?;

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

    return Ok(format!("{host}:{branch}"));
}

// fn get_url_branch(url: &str) -> Result<String> {
//     let url = Url::parse(&url)?;

// }

#[derive(Debug)]
struct InReleaseParser {
    _source: Vec<IndexMap<String, String>>,
    checksums: Vec<ChecksumItem>,
}

#[derive(Debug)]
struct ChecksumItem {
    name: String,
    size: u64,
    checksum: String,
    file_type: DistFileType,
}

#[derive(Debug, PartialEq)]
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

pub fn debcontrol_from_file(p: &Path) -> Result<Vec<IndexMap<String, String>>> {
    let mut f = std::fs::File::open(p)?;
    let mut s = String::new();
    f.read_to_string(&mut s)?;

    debcontrol_from_str(&s)
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

pub async fn get_sources_dists_filename(
    sources: &[SourceEntry],
    client: &Client,
) -> Result<Vec<String>> {
    let dist_urls = sources.iter().map(|x| x.dist_path()).collect::<Vec<_>>();
    let dists_in_releases = dist_urls.iter().map(|x| {
        (
            x.to_owned(),
            FileName::new(&format!("{}/{}", x, "InRelease")),
        )
    });

    let mut res = vec![];

    let components = sources
        .iter()
        .map(|x| x.components.to_owned())
        .collect::<Vec<_>>();

    for (i, c) in dists_in_releases.enumerate() {
        let filename = c.1?.0;
        let in_release = InReleaseParser::new(&Path::new(APT_LIST_DISTS).join(&filename));

        let in_release = if in_release.is_err() {
            update_db(sources, client, None).await?;
            let in_release = InReleaseParser::new(&Path::new(APT_LIST_DISTS).join(filename))?;

            in_release
        } else {
            in_release?
        };

        let checksums = in_release
            .checksums
            .iter()
            .filter(|x| components[i].contains(&x.name.split('/').next().unwrap().to_string()))
            .collect::<Vec<_>>();

        for j in checksums {
            if j.name.ends_with("Packages") {
                res.push(FileName::new(&format!("{}/{}", c.0, j.name))?);
            }
        }
    }

    let res = res.into_iter().map(|x| x.0).collect();

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

    Ok(res)
}

/// Download packages
pub async fn packages_download(
    list: &[String],
    apt: &[IndexMap<String, String>],
    sources: &[SourceEntry],
    client: &Client,
    limit: Option<usize>,
) -> Result<()> {
    let mut task = vec![];
    let mb = Arc::new(MultiProgress::new());
    let mut total = 0;

    if list.len() == 0 {
        return Ok(());
    }

    info!("Downloading {} packages ...", list.len());

    for i in list.iter() {
        let v = apt.iter().find(|x| x.get("Package") == Some(i));
        let Some(v) = v else { bail!("Can not get package {} from list", i) };

        let size = v["Size"].clone().parse::<u64>()?;
        total += size;
    }

    let global_bar = mb.insert(0, ProgressBar::new(total as u64));
    global_bar.set_style(oma_style_pb()?);
    global_bar.enable_steady_tick(Duration::from_millis(1000));
    global_bar.set_message("Progress");

    let list_len = list.len();

    for (i, c) in list.iter().enumerate() {
        let v = apt.iter().find(|x| x.get("Package") == Some(c));

        let Some(v) = v else { bail!("Can not get package {} from list", c) };
        let file_name = v["Filename"].clone();
        let checksum = v["SHA256"].clone();
        let version = v["Version"].clone();
        let mut file_name_split = file_name.split('/');

        let branch = file_name_split
            .nth(1)
            .take()
            .context(format!("Can not parse package {c} Filename field!"))?;

        let component = file_name_split
            .next()
            .take()
            .context(format!("Can not parse package {c} Filename field!"))?;

        let mirrors = sources
            .iter()
            .filter(|x| x.components.contains(&component.to_string()))
            .filter(|x| x.suite == branch)
            .map(|x| x.url.to_owned())
            .collect::<Vec<_>>();

        let mbc = mb.clone();

        task.push(download_package(
            file_name,
            mirrors,
            None,
            client,
            checksum,
            version,
            mbc,
            i,
            list_len,
            global_bar.clone(),
        ));
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

pub async fn package_list(
    db_file_paths: Vec<PathBuf>,
    sources: &[SourceEntry],
    client: &Client,
) -> Result<Vec<IndexMap<String, String>>> {
    let mut apt = vec![];
    for i in db_file_paths {
        if !i.is_file() {
            update_db(sources, client, None).await?;
        }
        let parse = debcontrol_from_file(&i)?;
        apt.extend(parse);
    }

    apt.sort_by(|x, y| x["Package"].cmp(&y["Package"]));

    Ok(apt)
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
        .map(|x| format!("{}/{}", x, "InRelease"));

    let mut dist_files = Vec::new();

    for i in dists_in_releases {
        dist_files.push(download_db(i, client, "InRelease".to_owned(), None, None, None).await?);
    }

    let components = sources
        .iter()
        .map(|x| x.components.to_owned())
        .collect::<Vec<_>>();

    for (index, (name, file)) in dist_files.into_iter().enumerate() {
        let p = Path::new(APT_LIST_DISTS).join(name.0);

        if !p.exists() || !p.is_file() {
            tokio::fs::create_dir_all(APT_LIST_DISTS).await?;
            tokio::fs::write(&p, &file.0).await?;
        } else {
            let mut f = tokio::fs::File::open(&p).await?;
            let mut buf = Vec::new();
            f.read_to_end(&mut buf).await?;

            if buf != file.0 {
                std::fs::write(&p, &file.0)?;
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
        let global_bar = mb.insert(0, ProgressBar::new(total as u64));
        global_bar.set_style(oma_style_pb()?);
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
                let mut f = std::fs::File::open(&p)?;
                let mut buf = Vec::new();
                f.read_to_end(&mut buf)?;

                let checksums_index = checksums
                    .iter()
                    .position(|x| x.name == not_compress_file)
                    .unwrap();

                let hash = checksums[checksums_index].checksum.to_owned();

                if checksum(&buf, &hash).is_err() {
                    task.push(download_and_extract(
                        &dist_urls[index],
                        c,
                        client,
                        file_name.0,
                        typ,
                        global_bar.clone(),
                        (i, len),
                        mb.clone(),
                    ));
                } else {
                    continue;
                }
            } else {
                task.push(download_and_extract(
                    &dist_urls[index],
                    c,
                    client,
                    file_name.0,
                    typ,
                    global_bar.clone(),
                    (i, len),
                    mb.clone(),
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
    global_bar: ProgressBar,
    progress: (usize, usize),
    mbc: Arc<MultiProgress>,
) -> Result<()> {
    let (name, buf) = download_db(
        format!("{}/{}", dist_url, i.name),
        client,
        typ.to_owned(),
        Some(global_bar),
        Some(progress),
        Some(mbc),
    )
    .await?;
    checksum(&buf.0, &i.checksum)?;

    let buf = decompress(&buf.0, &name.0)?;
    let p = Path::new(APT_LIST_DISTS).join(not_compress_file);
    std::fs::write(p, buf)?;

    Ok(())
}

/// Check download is success
fn checksum(buf: &[u8], hash: &str) -> Result<()> {
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

// Read dpkg status
pub fn dpkg_status() -> Result<Vec<IndexMap<String, String>>> {
    let status = debcontrol_from_file(Path::new("/var/lib/dpkg/status"))?;

    Ok(status)
}
