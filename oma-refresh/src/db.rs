use std::{
    collections::HashMap,
    path::{Path, PathBuf},
};

use derive_builder::Builder;
use oma_apt_sources_lists::{SourceEntry, SourceLine, SourcesLists, SourceError};
use oma_console::debug;
use oma_fetch::{
    checksum::ChecksumError, DownloadEntry, DownloadEntryBuilder, DownloadEntryBuilderError,
    DownloadError, DownloadEvent, DownloadResult, DownloadSource, DownloadSourceType, OmaFetcher,
};
use oma_utils::dpkg::dpkg_arch;
use once_cell::sync::Lazy;
use reqwest::StatusCode;
use serde::Deserialize;
use url::Url;

use crate::{
    inrelease::{ChecksumItem, DistFileType, InReleaseParser, InReleaseParserError},
    util::database_filename,
};

#[derive(Deserialize)]
pub struct MirrorMapItem {
    url: String,
}

static MIRROR: Lazy<PathBuf> =
    Lazy::new(|| PathBuf::from("/usr/share/distro-repository-data/mirrors.yml"));

pub static APT_LIST_DISTS: Lazy<PathBuf> = Lazy::new(|| {
    let p = PathBuf::from("/var/lib/apt/lists");

    if !p.is_dir() {
        let _ = std::fs::create_dir_all(&p);
    }

    p
});

#[derive(Debug, thiserror::Error)]
pub enum RefreshError {
    #[error("Invalid URL: {0}")]
    InvaildUrl(String),
    #[error("Can not parse distro repo data {0}: {1}")]
    ParseDistroRepoDataError(String, serde_yaml::Error),
    #[error("Scan sources.list failed: {0}")]
    ScanSourceError(SourceError),
    #[error("Unsupport Protocol: {0}")]
    UnsupportedProtocol(String),
    #[error(transparent)]
    FetcherError(#[from] oma_fetch::DownloadError),
    #[error(transparent)]
    ReqwestError(#[from] reqwest::Error),
    #[error(transparent)]
    TopicsError(#[from] oma_topics::OmaTopicsError),
    #[error("Failed to download InRelease from URL {0}: Remote file not found (HTTP 404).")]
    NoInReleaseFile(String),
    #[error(transparent)]
    InReleaseParserError(#[from] InReleaseParserError),
    #[error(transparent)]
    DpkgArchError(#[from] oma_utils::dpkg::DpkgError),
    #[error(transparent)]
    JoinError(#[from] tokio::task::JoinError),
    #[error(transparent)]
    DownloadEntryBuilderError(#[from] DownloadEntryBuilderError),
    #[error(transparent)]
    ChecksumError(#[from] ChecksumError),
    #[error(transparent)]
    IOError(#[from] std::io::Error),
}

type Result<T> = std::result::Result<T, RefreshError>;

pub(crate) fn get_url_short_and_branch(
    url: &str,
    mirror_map: &Option<HashMap<String, MirrorMapItem>>,
) -> Result<String> {
    let url = Url::parse(url).map_err(|_| RefreshError::InvaildUrl(url.to_string()))?;

    let host = if url.scheme() == "file" {
        "Local Mirror"
    } else {
        url.host_str()
            .ok_or_else(|| RefreshError::InvaildUrl(url.to_string()))?
    };

    let schema = url.scheme();
    let branch = url
        .path()
        .split('/')
        .nth_back(1)
        .ok_or_else(|| RefreshError::InvaildUrl(url.to_string()))?;

    let url = format!("{schema}://{host}/");

    // MIRROR 文件为 AOSC 独有，为兼容其他 .deb 系统，这里不直接返回错误
    if let Some(mirror_map) = mirror_map {
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

fn mirror_map(buf: &[u8]) -> Result<HashMap<String, MirrorMapItem>> {
    let mirror_map: HashMap<String, MirrorMapItem> = serde_yaml::from_slice(buf)
        .map_err(|e| RefreshError::ParseDistroRepoDataError(MIRROR.display().to_string(), e))?;

    Ok(mirror_map)
}

/// Get /etc/apt/sources.list and /etc/apt/sources.list.d
pub fn get_sources<P: AsRef<Path>>(sysroot: P) -> Result<Vec<OmaSourceEntry>> {
    let mut res = Vec::new();
    let list = SourcesLists::scan_from_root(sysroot)
        .map_err(|e| RefreshError::ScanSourceError(e))?;

    for file in list.iter() {
        for i in &file.lines {
            if let SourceLine::Entry(entry) = i {
                res.push(OmaSourceEntry::new(entry)?);
            }
        }
    }

    Ok(res)
}

#[derive(Debug, Clone)]
pub struct OmaSourceEntry {
    from: OmaSourceEntryFrom,
    components: Vec<String>,
    url: String,
    _suite: String,
    inrelease_path: String,
    dist_path: String,
    is_flat: bool,
    signed_by: Option<String>,
}

#[derive(PartialEq, Eq, Debug, Clone)]
enum OmaSourceEntryFrom {
    Http,
    Local,
}

pub enum Event {
    Info(String),
}

impl OmaSourceEntry {
    fn new(v: &SourceEntry) -> Result<Self> {
        let from = if v.url().starts_with("http://") || v.url().starts_with("https://") {
            OmaSourceEntryFrom::Http
        } else if v.url().starts_with("file://") {
            OmaSourceEntryFrom::Local
        } else {
            return Err(RefreshError::UnsupportedProtocol(format!("{v:?}")));
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
            return Err(RefreshError::UnsupportedProtocol(format!("{v:?}")));
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
            _suite: suite,
            is_flat,
            inrelease_path,
            dist_path,
            signed_by,
        })
    }
}

impl OmaRefreshBuilder {
    fn default_sources(&self) -> PathBuf {
        PathBuf::from("/")
    }

    fn default_download_dir(&self) -> PathBuf {
        PathBuf::from("/var/lib/apt/lists")
    }

    fn default_arch(&self) -> std::result::Result<String, String> {
        match dpkg_arch() {
            Ok(a) => Ok(a),
            Err(e) => Err(e.to_string()),
        }
    }
}

#[derive(Builder, Default)]
pub struct OmaRefresh {
    #[builder(default = "self.default_sources()")]
    source: PathBuf,
    #[builder(default = "4")]
    limit: usize,
    #[builder(default = "self.default_arch()?")]
    arch: String,
    #[builder(default = "self.default_download_dir()")]
    download_dir: PathBuf,
    #[builder(default = "true")]
    download_compress: bool,
}

impl OmaRefresh {
    pub async fn start<F, F2>(self, callback: F, handle_topic_msg: F2) -> Result<()>
    where
        F: Fn(usize, RefreshEvent, Option<u64>) + Clone + Send + Sync,
        F2: Fn() -> String + Copy,
    {
        let source = get_sources(self.source)?;
        update_db(
            source,
            self.limit,
            self.arch,
            self.download_dir,
            self.download_compress,
            callback,
            handle_topic_msg,
        )
        .await
    }
}

#[derive(Debug)]
pub enum RefreshEvent {
    DownloadEvent(DownloadEvent),
    ClosingTopic(String),
}

impl From<DownloadEvent> for RefreshEvent {
    fn from(value: DownloadEvent) -> Self {
        RefreshEvent::DownloadEvent(value)
    }
}

// Update database
async fn update_db<F, F2>(
    sourceslist: Vec<OmaSourceEntry>,
    limit: usize,
    arch: String,
    download_dir: PathBuf,
    download_compress: bool,
    callback: F,
    handle_topic_msg: F2,
) -> Result<()>
where
    F: Fn(usize, RefreshEvent, Option<u64>) + Clone + Send + Sync,
    F2: Fn() -> String + Copy,
{
    let mut tasks = vec![];

    let m = tokio::fs::read(&*MIRROR).await;
    let m = match m {
        Ok(m) => mirror_map(&m).ok(),
        Err(_) => None,
    };

    for source_entry in &sourceslist {
        let msg = get_url_short_and_branch(&source_entry.inrelease_path, &m)?;

        let sources = vec![DownloadSource::new(
            source_entry.inrelease_path.clone(),
            match source_entry.from {
                OmaSourceEntryFrom::Http => DownloadSourceType::Http,
                OmaSourceEntryFrom::Local => DownloadSourceType::Local,
            },
        )];

        let task = DownloadEntryBuilder::default()
            .source(sources)
            .filename(database_filename(&source_entry.inrelease_path).into())
            .dir(download_dir.clone())
            .allow_resume(false)
            .msg(format!("{msg} InRelease"))
            .build()?;

        debug!("oma will fetch {} InRelease", source_entry.url);
        tasks.push(task);
    }

    let res = OmaFetcher::new(None, tasks, Some(limit))?
        .start_download(|c, event| callback(c, RefreshEvent::from(event), None))
        .await;

    let mut all_inrelease = vec![];

    let mut not_found = vec![];

    for inrelease in res {
        if cfg!(feature = "aosc") {
            match inrelease {
                Ok(i) => {
                    debug!("{} fetched", &i.filename);
                    all_inrelease.push(i)
                }
                Err(e) => {
                    #[cfg(feature = "aosc")]
                    match e {
                        DownloadError::ReqwestError(e)
                            if e.status()
                                .map(|x| x == StatusCode::NOT_FOUND)
                                .unwrap_or(false) =>
                        {
                            let url = e.url().map(|x| x.to_owned());
                            not_found.push(url.unwrap());
                        }
                        _ => return Err(e.into()),
                    }
                    #[cfg(not(feature = "aosc"))]
                    return Err(e.into());
                }
            }
        } else {
            let i = inrelease?;
            debug!("{} fetched", &i.filename);
            all_inrelease.push(i);
        }
    }

    #[cfg(feature = "aosc")]
    {
        if !not_found.is_empty() {
            let removed_suites = oma_topics::scan_closed_topic(handle_topic_msg).await?;
            for url in not_found {
                let suite = url
                    .path_segments()
                    .and_then(|mut x| x.nth_back(1).map(|x| x.to_string()))
                    .ok_or_else(|| RefreshError::InvaildUrl(url.to_string()))?;

                if !removed_suites.contains(&suite) {
                    return Err(RefreshError::NoInReleaseFile(url.to_string()));
                }

                callback(0, RefreshEvent::ClosingTopic(suite), None);
            }
        }
    }

    let mut total = 0;
    let mut tasks = vec![];

    for inrelease_summary in all_inrelease {
        // 源数据确保是存在的，所以直接 unwrap
        let ose = sourceslist.get(inrelease_summary.count).unwrap();
        let urlc = &ose.url;

        debug!("Getted Oma source entry: {:?}", ose);
        let inrelease_path = download_dir.join(&*inrelease_summary.filename);

        let s = tokio::fs::read_to_string(&inrelease_path).await?;

        let inrelease = InReleaseParser::new(
            &s,
            ose.signed_by.as_deref(),
            urlc,
            &arch,
            ose.is_flat,
            &inrelease_path,
        )?;

        let checksums = inrelease
            .checksums
            .iter()
            .filter(|x| {
                ose.components
                    .iter()
                    .any(|y| y.contains(x.name.split('/').next().unwrap_or(&x.name)))
            })
            .map(|x| x.to_owned())
            .collect::<Vec<_>>();

        let handle = if ose.is_flat {
            debug!("{} is flat repo", ose.url);
            // Flat repo
            let mut handle = vec![];
            for i in &inrelease.checksums {
                if i.file_type == DistFileType::PackageList {
                    debug!("oma will download package list: {}", i.name);
                    handle.push(i);
                    total += i.size;
                }
            }

            handle
        } else {
            let mut handle = vec![];
            for i in &checksums {
                match &i.file_type {
                    DistFileType::BinaryContents => {
                        debug!("oma will download Binary Contents: {}", i.name);
                        handle.push(i);
                        total += i.size;
                    }
                    DistFileType::Contents | DistFileType::PackageList if !download_compress => {
                        debug!("oma will download Package List/Contetns: {}", i.name);
                        handle.push(i);
                        total += i.size;
                    }
                    DistFileType::CompressContents(_) => {
                        if download_compress {
                            debug!(
                                "oma will download compress Package List/compress Contetns: {}",
                                i.name
                            );

                            if !handle.contains(&i) {
                                handle.push(i);
                                total += i.size;
                            }
                        }
                    }
                    DistFileType::CompressPackageList(s) => {
                        if download_compress {
                            debug!(
                                "oma will download compress Package List/compress Contetns: {}",
                                i.name
                            );

                            if !handle.contains(&i) {
                                handle.push(i);
                                let size = checksums
                                    .iter()
                                    .find(|x| &x.name == s)
                                    .map(|x| x.size)
                                    .unwrap_or(i.size);

                                total += size;
                            }
                        }
                    }
                    _ => continue,
                }
            }

            handle
        };

        for c in handle {
            collect_download_task(
                c,
                sourceslist.get(inrelease_summary.count).unwrap(),
                &checksums,
                &download_dir,
                &mut tasks,
                &m,
            )?;
        }
    }

    let res = OmaFetcher::new(None, tasks, Some(limit))?
        .start_download(|count, event| callback(count, RefreshEvent::from(event), Some(total)))
        .await;

    res.into_iter().collect::<DownloadResult<Vec<_>>>()?;

    Ok(())
}

fn collect_download_task(
    c: &ChecksumItem,
    source_index: &OmaSourceEntry,
    checksums: &[ChecksumItem],
    download_dir: &Path,
    tasks: &mut Vec<DownloadEntry>,
    m: &Option<HashMap<String, MirrorMapItem>>,
) -> Result<()> {
    let (typ, not_compress_filename_before) = match &c.file_type {
        DistFileType::CompressContents(s) => ("Contents", s),
        DistFileType::Contents => ("Contents", &c.name),
        DistFileType::CompressPackageList(s) => ("Package List", s),
        DistFileType::PackageList => ("Package List", &c.name),
        DistFileType::BinaryContents => ("BinContents", &c.name),
        _ => unreachable!(),
    };

    let msg = get_url_short_and_branch(&source_index.inrelease_path, m)?;

    let dist_url = source_index.dist_path.clone();
    let file_path = if matches!(c.file_type, DistFileType::CompressContents(_)) {
        format!("{}/{}", dist_url, c.name)
    } else {
        format!("{}/{}", dist_url, not_compress_filename_before)
    };

    let from = match source_index.from {
        OmaSourceEntryFrom::Http => DownloadSourceType::Http,
        OmaSourceEntryFrom::Local => DownloadSourceType::Local,
    };

    let sources = vec![DownloadSource::new(file_path.clone(), from)];

    let checksum = if matches!(c.file_type, DistFileType::CompressContents(_)) {
        Some(&c.checksum)
    } else {
        checksums
            .iter()
            .find(|x| &x.name == not_compress_filename_before)
            .as_ref()
            .map(|c| &c.checksum)
    };

    let mut task = DownloadEntryBuilder::default();
    task.source(sources);
    task.filename(database_filename(&file_path).into());
    task.dir(download_dir.to_path_buf());
    task.allow_resume(false);
    task.msg(format!("{msg} {typ}"));
    task.extract(!matches!(c.file_type, DistFileType::CompressContents(_)));

    if let Some(checksum) = checksum {
        task.hash(checksum);
    }

    let task = task.build()?;
    debug!("oma will download source database: {file_path}");
    tasks.push(task);

    Ok(())
}
