use std::{
    collections::HashMap,
    path::{Path, PathBuf},
};

use apt_sources_lists::{SourceEntry, SourcesLists};
use futures::future::BoxFuture;
use once_cell::sync::Lazy;
use serde::Deserialize;
use url::Url;

#[derive(Deserialize)]
struct MirrorMapItem {
    url: String,
}

static MIRROR: Lazy<PathBuf> =
    Lazy::new(|| PathBuf::from("/usr/share/distro-repository-data/mirrors.yml"));

#[derive(Debug, thiserror::Error)]
pub enum RefreshError {
    #[error("Invalid URL: {0}")]
    InvaildUrl(String),
    #[error("Can not parse distro repo data: {0}")]
    ParseDistroRepoDataErrpr(String),
    #[error("Can not read file {}: {}", .path, .error)]
    ReadFileError { path: String, error: std::io::Error },
}

type Result<T> = std::result::Result<T, RefreshError>;

pub(crate) async fn get_url_short_and_branch(url: &str) -> Result<String> {
    let url = Url::parse(url).map_err(|_| RefreshError::InvaildUrl(url.to_string()))?;
    let host = url
        .host_str()
        .ok_or_else(|| RefreshError::InvaildUrl(url.to_string()))?;

    let schema = url.scheme();
    let branch = url
        .path()
        .split('/')
        .nth_back(1)
        .ok_or_else(|| RefreshError::InvaildUrl(url.to_string()))?;
    let url = format!("{schema}://{host}/");

    // MIRROR 文件为 AOSC 独有，为兼容其他 .deb 系统，这里不直接返回错误
    if let Ok(mirror_map_f) = tokio::fs::read(&*MIRROR).await {
        let mirror_map: HashMap<String, MirrorMapItem> = serde_yaml::from_slice(&mirror_map_f)
            .map_err(|_| RefreshError::ParseDistroRepoDataErrpr(MIRROR.display().to_string()))?;

        for (k, v) in mirror_map.iter() {
            let mirror_url =
                Url::parse(&v.url).map_err(|_| RefreshError::InvaildUrl(v.url.to_string()))?;
            let mirror_url_host = mirror_url
                .host_str()
                .ok_or_else(|| RefreshError::InvaildUrl(v.url.to_string()))?;

            let schema = mirror_url.scheme();
            let mirror_url = format!("{schema}://{mirror_url_host}/");

            if mirror_url == url {
                return Ok(format!("{k}:{branch}"));
            }
        }
    }

    Ok(format!("{host}:{branch}"))
}

/// Get /etc/apt/sources.list and /etc/apt/sources.list.d
pub fn get_sources() -> Result<Vec<SourceEntry>> {
    let mut res = Vec::new();
    let list = SourcesLists::scan()
        .map_err(|e| anyhow!(fl!("can-not-parse-sources-list", e = e.to_string())))?;

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
        error!("{}", fl!("unsupport-cdrom", url = i.url()));
    }

    if !cdrom.is_empty() {
        bail!(fl!("unsupport-some-mirror"));
    }

    Ok(res)
}

#[derive(Debug)]
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

#[derive(PartialEq, Eq, Debug)]
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
            bail!("{} {v:?}", fl!("unsupport-sourceentry"))
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
            bail!("{} {v:?}", fl!("unsupport-sourceentry"))
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

// Update database
async fn update_db(sources: &[SourceEntry], client: &Client, limit: Option<usize>) -> Result<()> {
    info!("{}", fl!("refreshing-repo-metadata"));

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

                tracing::debug!("oma will fetch {} InRelease", c.url);
                tasks.push(task);
            }
            OmaSourceEntryFrom::Local => {
                let task: BoxFuture<
                    '_,
                    std::result::Result<(FileName, usize, bool), DownloadError>,
                > = Box::pin(download_db_local(
                    &c.inrelease_path,
                    i,
                    OmaProgressBar::new(None, Some((i + 1, sources.len())), MB.clone(), None),
                ));
                tracing::debug!("oma will fetch {} InRelease", c.url);
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
                Ok(i) => {
                    tracing::debug!("{} fetched", &i.0 .0);
                    res_2.push(i)
                }
                Err(e) => match e {
                    DownloadError::NotFound(url) => {
                        let removed_suites = topics::scan_closed_topic(client).await?;

                        tracing::debug!("Removed topics: {removed_suites:?}");

                        let suite = url
                            .split('/')
                            .nth_back(1)
                            .context(fl!("can-not-get-suite", url = url.to_string()))?
                            .to_string();

                        if !removed_suites.contains(&suite) {
                            return Err(anyhow!(fl!("not-found", url = url.to_string())));
                        }
                    }
                    _ => return Err(e.into()),
                },
            }
        } else {
            let i = i?;
            tracing::debug!("{} fetched", &i.0 .0);
            res_2.push(i);
        }
    }

    for (name, index, _) in res_2 {
        let ose = sources.get(index).unwrap();

        tracing::debug!("Getted Oma source entry: {:?}", ose);

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
            tracing::debug!("{} is flat repo", ose.url);
            // Flat repo
            let mut handle = vec![];
            for i in &inrelease.checksums {
                if i.file_type == DistFileType::PackageList {
                    tracing::debug!("oma will download package list: {}", i.name);
                    handle.push(i);
                    total += i.size;
                }
            }

            handle
        } else {
            let mut handle = vec![];
            for i in &checksums {
                match i.file_type {
                    DistFileType::BinaryContents => {
                        tracing::debug!("oma will download Binary Contents: {}", i.name);
                        handle.push(i);
                        total += i.size;
                    }
                    DistFileType::Contents | DistFileType::PackageList => {
                        if ARCH.get() == Some(&"mips64r6el".to_string()) {
                            tracing::debug!("oma will download Package List/Contetns: {}", i.name);
                            handle.push(i);
                            total += i.size;
                        }
                    }
                    DistFileType::CompressContents | DistFileType::CompressPackageList => {
                        if ARCH.get() != Some(&"mips64r6el".to_string()) {
                            tracing::debug!(
                                "oma will download compress Package List/compress Contetns: {}",
                                i.name
                            );

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
                DistFileType::CompressContents => fl!("contents"),
                DistFileType::CompressPackageList | DistFileType::PackageList => fl!("pkg_list"),
                DistFileType::BinaryContents => fl!("bincontents"),
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

                    tracing::debug!("oma will download http source database: {p}");
                    let task: BoxFuture<'_, Result<()>> = Box::pin(download_and_extract_db(
                        p,
                        c,
                        client,
                        not_compress_filename.0,
                        typ.to_string(),
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

                    tracing::debug!("oma will download local source database: {p} {}", c.name);
                    let task: BoxFuture<'_, Result<()>> = Box::pin(download_and_extract_db_local(
                        p,
                        not_compress_filename.0,
                        c,
                        opb,
                        typ.to_string(),
                    ));

                    tasks.push(task);
                }
            }
        }

        // 默认限制一次最多下载八个包，减少服务器负担
        let stream = futures::stream::iter(tasks).buffer_unordered(limit.unwrap_or(4));

        stream
            .collect::<Vec<_>>()
            .await
            .into_iter()
            .collect::<Result<Vec<_>>>()?;

        global_bar.finish_and_clear();
    }

    Ok(())
}
