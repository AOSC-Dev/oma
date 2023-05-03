use std::{
    collections::HashMap,
    io::Read,
    path::{Path, PathBuf},
    time::Duration,
};

use anyhow::{anyhow, bail, Context, Result};
use apt_sources_lists::*;
use flate2::bufread::GzDecoder;
use futures::{future::BoxFuture, StreamExt};
use indicatif::ProgressBar;
use once_cell::sync::Lazy;
use reqwest::{Client, Url};
use rust_apt::config::Config;
use serde::Deserialize;
use time::{format_description::well_known::Rfc2822, OffsetDateTime};
use tokio::{runtime::Runtime, task::spawn_blocking};
use xz2::read::XzDecoder;

use crate::{
    checksum::Checksum,
    download::{download, oma_spinner, oma_style_pb, DownloadError, OmaProgressBar},
    error, info, verify, warn, ARCH, MB,
};

#[cfg(feature = "aosc")]
use crate::topics;

use std::sync::atomic::Ordering;

pub static APT_LIST_DISTS: Lazy<PathBuf> = Lazy::new(|| {
    let p = PathBuf::from("/var/lib/apt/lists");

    if !p.is_dir() {
        let _ = std::fs::create_dir_all(&p);
    }

    p
});

pub static DOWNLOAD_DIR: Lazy<PathBuf> = Lazy::new(|| {
    let config = Config::new();
    let archives_dir = config.get("Dir::Cache::Archives");

    let path = if let Some(archives_dir) = archives_dir {
        if !Path::new(&archives_dir).is_absolute() {
            PathBuf::from(format!("/var/cache/apt/{archives_dir}"))
        } else {
            PathBuf::from(archives_dir)
        }
    } else {
        PathBuf::from("/var/cache/apt/archives/")
    };

    if !path.is_dir() {
        warn!(
            "Dir::Cache::Archives path: {} does not exist! fallback to /var/cache/apt/archives",
            path.display()
        );

        let p = PathBuf::from("/var/cache/apt/archives/");

        if !p.is_dir() {
            let _ = std::fs::create_dir_all(&p);
        }

        p
    } else {
        path
    }
});

static MIRROR: Lazy<PathBuf> =
    Lazy::new(|| PathBuf::from("/usr/share/distro-repository-data/mirrors.yml"));

#[derive(Debug)]
struct FileName(String);

impl FileName {
    fn new(s: &str) -> Self {
        let s = s.split("://").nth(1).unwrap_or(s).replace('/', "_");

        FileName(s)
    }
}

async fn download_db(
    url: String,
    client: &Client,
    typ: String,
    opb: OmaProgressBar,
    i: usize,
    checksum: Option<&str>,
) -> std::result::Result<(FileName, usize, bool), DownloadError> {
    let filename = FileName::new(&url).0;
    let url_short = get_url_short_and_branch(&url).await?;

    let mut opb = opb;

    opb.msg = Some(format!("{url_short} {typ}"));
    let opb = opb.clone();

    let is_download = download(
        &url,
        client,
        filename.to_string(),
        &APT_LIST_DISTS,
        checksum,
        opb,
        false,
    )
    .await?;

    Ok((FileName::new(&url), i, is_download))
}

#[derive(Deserialize)]
struct MirrorMapItem {
    url: String,
}

pub async fn get_url_short_and_branch(url: &str) -> Result<String> {
    let url = Url::parse(url).map_err(|e| anyhow!("Invalid URL: {url}, why: {e}"))?;
    let host = url.host_str().context("Can not parse {url} host!")?;
    let schema = url.scheme();
    let branch = url
        .path()
        .split('/')
        .nth_back(1)
        .context("Can not get {url} branch!")?;
    let url = format!("{schema}://{host}/");

    // MIRROR 文件为 AOSC 独有，为兼容其他 .deb 系统，这里不直接返回错误
    if let Ok(mirror_map_f) = tokio::fs::read(&*MIRROR).await {
        let mirror_map: HashMap<String, MirrorMapItem> = serde_yaml::from_slice(&mirror_map_f)
            .map_err(|e| anyhow!("Can not parse distro-repository-data file: {}, why: {e}, maybe you environment is broken?", MIRROR.display()))?;

        for (k, v) in mirror_map.iter() {
            let mirror_url = Url::parse(&v.url).map_err(|e| {
                anyhow!("Distro repository data file have invalid URL: {url}, why: {e}")
            })?;
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

    Ok(format!("{host}:{branch}"))
}

#[derive(Debug)]
struct InReleaseParser {
    _source: Vec<HashMap<String, String>>,
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
    Release,
}

impl InReleaseParser {
    fn new(p: &Path, trust_files: Option<&str>, mirror: &str) -> Result<Self> {
        let s = std::fs::read_to_string(p)
            .map_err(|e| anyhow!("Can not read InRelease file, why: {e}"))?;

        let s = if s.starts_with("-----BEGIN PGP SIGNED MESSAGE-----") {
            verify::verify(&s, trust_files, mirror)?
        } else {
            s
        };

        let source = debcontrol_from_str(&s)
            .map_err(|e| anyhow!("Can not parse InRelease file: {}, why: {e}", p.display()))?;

        let source_first = source.first();

        let date = source_first
            .and_then(|x| x.get("Date"))
            .take()
            .context("InRelease Date entry is empty!")?;

        let valid_until = source_first
            .and_then(|x| x.get("Valid-Until"))
            .take()
            .context("InRelease Valid-Until entry is empty")?;

        let date = OffsetDateTime::parse(date, &Rfc2822)
            .context(format!("BUG: can not parse data field: {date} to Rfc2822, Please report this error to upstream: https://github.com/aosc-dev/oma"))?;

        let valid_until = OffsetDateTime::parse(valid_until, &Rfc2822)
            .context(format!("BUG: can not parse valid_until field: {valid_until} to Rfc2822, Please report this error to upstream: https://github.com/aosc-dev/oma"))?;

        let now = OffsetDateTime::now_utc();

        if now < date {
            bail!("The computer time is earlier than the signature time in InRelease.")
        }

        if now > valid_until {
            bail!("InRelease signature file has expired.")
        }

        let sha256 = source_first
            .and_then(|x| x.get("SHA256"))
            .take()
            .context("InRelease sha256 entry is empty!")?;

        let mut checksums = sha256.split('\n');

        // remove first item, It's empty
        checksums.next();

        let mut checksums_res = vec![];

        for i in checksums {
            let checksum = i.split_whitespace().collect::<Vec<_>>();
            let checksum = (checksum[2], checksum[1], checksum[0]);
            checksums_res.push(checksum);
        }

        let arch = ARCH.get().unwrap();

        let mut res = vec![];

        let c_res_clone = checksums_res.clone();

        let c = checksums_res
            .into_iter()
            .filter(|(name, _, _)| name.contains("all") || name.contains(arch))
            .collect::<Vec<_>>();

        let c = if c.is_empty() { c_res_clone } else { c };

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
            } else if i.0.contains("Release") {
                DistFileType::Release
            } else {
                bail!("BUG: InRelease Parser unsupport file type: {i:?}, Please report this to upstream: https://github.com/aosc-dev/oma");
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

fn debcontrol_from_str(s: &str) -> Result<Vec<HashMap<String, String>>> {
    let mut res = vec![];

    let debcontrol = debcontrol::parse_str(s).map_err(|e| anyhow!("{}", e))?;

    for i in debcontrol {
        let mut item = HashMap::new();
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
    let list = SourcesLists::scan()
        .map_err(|e| anyhow!("Can not scan and parse source.list file, why: {e}"))?;

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

pub struct OmaSourceEntry {
    from: OmaSourceEntryFrom,
    components: Vec<String>,
    url: String,
    pub suite: String,
    inrelease_path: String,
    dist_path: String,
    is_flat: bool,
    signed_by: Option<String>,
}

#[derive(PartialEq, Eq)]
enum OmaSourceEntryFrom {
    Http,
    Local,
}

impl OmaSourceEntry {
    fn new(v: &SourceEntry) -> Result<Self> {
        let from = if v.url().starts_with("http://") || v.url().starts_with("https://") {
            OmaSourceEntryFrom::Http
        } else if v.url().starts_with("file://") {
            OmaSourceEntryFrom::Local
        } else {
            bail!("Unsupport SourceEntry: {v:#?}")
        };

        let components = v.components.clone();
        let url = v.url.clone();
        let suite = v.suite.clone();
        let (dist_path, is_flat) = if components.is_empty() && suite == "/" {
            (v.url().to_string(), true)
        } else {
            (v.dist_path(), false)
        };

        let inrelease_path = if is_flat {
            // flat Repo
            format!("{url}/Release")
        } else if !components.is_empty() {
            // Normal Repo
            format!("{dist_path}/InRelease")
        } else {
            bail!("Unsupport SourceEntry: {v:?}")
        };

        let options = v.options.as_deref().unwrap_or_default();

        let options = options.split_whitespace().collect::<Vec<_>>();

        let signed_by = options
            .iter()
            .find(|x| x.strip_prefix("signed-by=").is_some())
            .map(|x| x.strip_prefix("signed-by=").unwrap().to_string());

        Ok(Self {
            from,
            components,
            url,
            suite,
            is_flat,
            inrelease_path,
            dist_path,
            signed_by,
        })
    }
}

fn hr_sources(sources: &[SourceEntry]) -> Result<Vec<OmaSourceEntry>> {
    let mut res = vec![];
    for i in sources {
        res.push(OmaSourceEntry::new(i)?);
    }

    Ok(res)
}

async fn download_db_local(
    db_path: &str,
    count: usize,
) -> std::result::Result<(FileName, usize, bool), DownloadError> {
    let db_path = db_path.split("://").nth(1).unwrap_or(db_path);
    let name = FileName::new(db_path);

    tokio::fs::copy(db_path, APT_LIST_DISTS.join(&name.0))
        .await
        .context(format!(
            "Can not copy {db_path} to {}",
            APT_LIST_DISTS.display()
        ))?;

    Ok((name, count, true))
}

pub fn update_db_runner(
    runtime: &Runtime,
    sources: &[SourceEntry],
    client: &Client,
    limit: Option<usize>,
) -> Result<()> {
    runtime.block_on(update_db(sources, client, limit))?;

    Ok(())
}

// Update database
async fn update_db(sources: &[SourceEntry], client: &Client, limit: Option<usize>) -> Result<()> {
    info!("Refreshing local repository metadata ...");

    let sources = hr_sources(sources)?;
    let mut tasks = vec![];

    for (i, c) in sources.iter().enumerate() {
        match c.from {
            OmaSourceEntryFrom::Http => {
                let task: BoxFuture<
                    '_,
                    std::result::Result<(FileName, usize, bool), DownloadError>,
                > = Box::pin(download_db(
                    c.inrelease_path.clone(),
                    client,
                    "InRelease".to_owned(),
                    OmaProgressBar::new(None, Some((i + 1, sources.len())), MB.clone(), None),
                    i,
                    None,
                ));

                tasks.push(task);
            }
            OmaSourceEntryFrom::Local => {
                let task: BoxFuture<
                    '_,
                    std::result::Result<(FileName, usize, bool), DownloadError>,
                > = Box::pin(download_db_local(&c.inrelease_path, i));
                tasks.push(task);
            }
        }
    }

    let stream = futures::stream::iter(tasks).buffer_unordered(limit.unwrap_or(4));
    let res = stream.collect::<Vec<_>>().await;

    let mut res_2 = vec![];

    for i in res {
        if cfg!(feature = "aosc") {
            match i {
                Ok(i) => res_2.push(i),
                Err(e) => match e {
                    DownloadError::NotFound(url) => {
                        let removed_suites = topics::scan_closed_topic(client).await?;

                        let suite = url
                            .split('/')
                            .nth_back(1)
                            .context(format!("Can not get suite: {url}"))?
                            .to_string();

                        if !removed_suites.contains(&suite) {
                            return Err(anyhow!(
                                "Could not get InRelease in url: {url}, Reason: 404 Not found."
                            ));
                        }
                    }
                    _ => return Err(e.into()),
                },
            }
        } else {
            res_2.push(i?);
        }
    }

    for (name, index, _) in res_2 {
        let ose = sources.get(index).unwrap();

        let inrelease = InReleaseParser::new(
            &APT_LIST_DISTS.join(name.0),
            ose.signed_by.as_deref(),
            &ose.url,
        )?;

        let checksums = inrelease
            .checksums
            .iter()
            .filter(|x| {
                ose.components
                    .contains(&x.name.split('/').next().unwrap_or(&x.name).to_owned())
            })
            .map(|x| x.to_owned())
            .collect::<Vec<_>>();

        let mut total = 0;
        let handle = if ose.is_flat {
            // Flat repo
            let mut handle = vec![];
            for i in &inrelease.checksums {
                if i.file_type == DistFileType::PackageList {
                    handle.push(i);
                    total += i.size;
                }
            }

            handle
        } else {
            let mut handle = vec![];
            for i in &checksums {
                match i.file_type {
                    DistFileType::Contents | DistFileType::PackageList => {
                        if ARCH.get() == Some(&"mips64r6el".to_string()) {
                            handle.push(i);
                            total += i.size;
                        }
                    }
                    DistFileType::CompressContents | DistFileType::CompressPackageList => {
                        if ARCH.get() != Some(&"mips64r6el".to_string()) {
                            handle.push(i);
                            total += i.size;
                        }
                    }
                    _ => continue,
                }
            }

            handle
        };

        let global_bar = MB.insert(0, ProgressBar::new(total));
        global_bar.set_style(oma_style_pb(true)?);
        global_bar.enable_steady_tick(Duration::from_millis(100));
        global_bar.set_message("Progress");

        let len = handle.len();

        let mut tasks = vec![];

        for (i, c) in handle.iter().enumerate() {
            let mut p_not_compress = Path::new(&c.name).to_path_buf();
            p_not_compress.set_extension("");
            let not_compress_filename_before = p_not_compress.to_string_lossy().to_string();

            let source_index = sources.get(index).unwrap();
            let not_compress_filename = FileName::new(&format!(
                "{}/{}",
                source_index.dist_path, not_compress_filename_before
            ));

            let typ = match c.file_type {
                DistFileType::CompressContents => "Contents",
                DistFileType::CompressPackageList | DistFileType::PackageList => "Package List",
                DistFileType::BinaryContents => "BinContents",
                _ => unreachable!(),
            };

            let opb = OmaProgressBar::new(
                None,
                Some((i + 1, len)),
                MB.clone(),
                Some(global_bar.clone()),
            );

            match source_index.from {
                OmaSourceEntryFrom::Http => {
                    let p = if !ose.is_flat {
                        source_index.dist_path.clone()
                    } else {
                        format!("{}/{}", source_index.dist_path, not_compress_filename.0)
                    };

                    let task: BoxFuture<'_, Result<()>> = Box::pin(download_and_extract_db(
                        p,
                        c,
                        client,
                        not_compress_filename.0,
                        typ,
                        opb,
                    ));

                    tasks.push(task);
                }
                OmaSourceEntryFrom::Local => {
                    let p = if !ose.is_flat {
                        source_index.dist_path.clone()
                    } else {
                        format!("{}/{}", source_index.dist_path, not_compress_filename.0)
                    };

                    let task: BoxFuture<'_, Result<()>> = Box::pin(download_and_extract_db_local(
                        p,
                        not_compress_filename.0,
                        c,
                        opb,
                    ));

                    tasks.push(task);
                }
            }
        }

        // 默认限制一次最多下载八个包，减少服务器负担
        let stream = futures::stream::iter(tasks).buffer_unordered(limit.unwrap_or(4));
        let res = stream.collect::<Vec<_>>().await;

        global_bar.finish_and_clear();

        // 遍历结果看是否有下载出错
        for i in res {
            i?;
        }
    }

    Ok(())
}

/// Download and extract package list database
async fn download_and_extract_db(
    dist_url: String,
    i: &ChecksumItem,
    client: &Client,
    not_compress_file: String,
    typ: &str,
    opb: OmaProgressBar,
) -> Result<()> {
    let (name, _, is_download) = download_db(
        format!("{}/{}", dist_url, i.name),
        client,
        typ.to_owned(),
        opb.clone(),
        0,
        Some(&i.checksum),
    )
    .await?;

    if !is_download {
        if let Some(ref gb) = opb.global_bar {
            gb.inc(i.size);
        }
        return Ok(());
    }

    let p = APT_LIST_DISTS.join(&name.0);

    let mbc = opb.mbc;
    let pb = mbc.add(ProgressBar::new_spinner());
    let progress = opb.progress;
    let progress = if let Some((cur, total)) = progress {
        format!("({cur}/{total}) ")
    } else {
        "".to_string()
    };

    pb.finish_and_clear();

    let buf = tokio::fs::read(&p)
        .await
        .map_err(|e| anyhow!("Can not read file: {}, why: {e}", p.display()))?;

    let pb = mbc.add(ProgressBar::new_spinner());
    pb.set_message(format!("{progress}Decompressing {typ}"));
    oma_spinner(&pb);
    let buf = spawn_blocking(move || decompress(&buf, &name.0)).await??;

    pb.finish_and_clear();

    let p = APT_LIST_DISTS.join(not_compress_file);
    tokio::fs::write(&p, buf)
        .await
        .map_err(|e| anyhow!("Can not write file: {}, why: {e}", p.display()))?;

    Ok(())
}

async fn download_and_extract_db_local(
    path: String,
    not_compress_file: String,
    i: &ChecksumItem,
    opb: OmaProgressBar,
) -> Result<()> {
    let path = path.split("://").nth(1).unwrap_or(&path).to_owned();

    let path = format!("{path}/{}", i.name);
    let pc = path.clone();
    let name = FileName::new(&i.name);

    let checksum = i.checksum.clone();

    let result =
        spawn_blocking(move || Checksum::from_sha256_str(&checksum)?.cmp_file(Path::new(&pc)))
            .await??;

    if result {
        return Ok(());
    }

    tokio::fs::copy(&path, APT_LIST_DISTS.join(&name.0))
        .await
        .context(format!(
            "Can not copy {path} to {}",
            APT_LIST_DISTS.display()
        ))?;

    let buf = tokio::fs::read(&path)
        .await
        .context(format!("Can not read file {path}"))?;

    let buf_len = buf.len();

    let extract_buf = if i.file_type == DistFileType::CompressContents
        || i.file_type == DistFileType::CompressPackageList
    {
        spawn_blocking(move || decompress(&buf, &name.0)).await??
    } else {
        buf
    };

    let p = APT_LIST_DISTS.join(not_compress_file);

    tokio::fs::write(&p, &extract_buf)
        .await
        .map_err(|e| anyhow!("Can not write file: {}, why: {e}", p.display()))?;

    if let Some(pb) = opb.global_bar {
        pb.inc(buf_len as u64);
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
