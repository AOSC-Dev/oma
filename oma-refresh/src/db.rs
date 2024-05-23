use std::{
    collections::HashMap,
    path::{Path, PathBuf},
};

use derive_builder::Builder;
use oma_apt_sources_lists::{SourceEntry, SourceError, SourceLine, SourceListType, SourcesLists};
use oma_fetch::{
    checksum::ChecksumError, reqwest, DownloadEntry, DownloadEntryBuilder,
    DownloadEntryBuilderError, DownloadEvent, DownloadResult, DownloadSource, DownloadSourceType,
    OmaFetcher, Summary,
};

#[cfg(feature = "aosc")]
use oma_fetch::DownloadError;

use oma_utils::dpkg::dpkg_arch;
use once_cell::sync::Lazy;

#[cfg(feature = "aosc")]
use reqwest::StatusCode;

use serde::Deserialize;
use tracing::debug;
use url::Url;

use crate::{
    inrelease::{ChecksumItem, DistFileType, InRelease, InReleaseParser, InReleaseParserError},
    util::database_filename,
};

#[derive(Deserialize)]
pub struct MirrorMapItem {
    url: String,
}

static MIRROR: Lazy<PathBuf> =
    Lazy::new(|| PathBuf::from("/usr/share/distro-repository-data/mirrors.yml"));

#[cfg(feature = "aosc")]
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
    DpkgArchError(#[from] oma_utils::dpkg::DpkgError),
    #[error(transparent)]
    JoinError(#[from] tokio::task::JoinError),
    #[error(transparent)]
    DownloadEntryBuilderError(#[from] DownloadEntryBuilderError),
    #[error(transparent)]
    ChecksumError(#[from] ChecksumError),
    #[error("Failed to operate dir or file {0}: {1}")]
    FailedToOperateDirOrFile(String, tokio::io::Error),
    #[error("Failed to parse InRelease file: {0}")]
    InReleaseParseError(String, InReleaseParserError),
}

#[cfg(not(feature = "aosc"))]
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
    #[error("Failed to download InRelease from URL {0}: Remote file not found (HTTP 404).")]
    NoInReleaseFile(String),
    #[error(transparent)]
    DpkgArchError(#[from] oma_utils::dpkg::DpkgError),
    #[error(transparent)]
    JoinError(#[from] tokio::task::JoinError),
    #[error(transparent)]
    DownloadEntryBuilderError(#[from] DownloadEntryBuilderError),
    #[error(transparent)]
    ChecksumError(#[from] ChecksumError),
    #[error("Failed to operate dir or file {0}: {1}")]
    FailedToOperateDirOrFile(String, tokio::io::Error),
    #[error("Failed to parse InRelease file: {0}")]
    InReleaseParseError(String, InReleaseParserError),
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
    let list = SourcesLists::scan_from_root(sysroot).map_err(RefreshError::ScanSourceError)?;

    for file in list.iter() {
        match file.entries {
            SourceListType::SourceLine(ref lines) => {
                for i in lines {
                    if let SourceLine::Entry(entry) = i {
                        res.push(OmaSourceEntry::try_from(entry)?);
                    }
                }
            }
            SourceListType::Deb822(ref e) => {
                for i in &e.entries {
                    res.push(OmaSourceEntry::try_from(i)?);
                }
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

impl TryFrom<&SourceEntry> for OmaSourceEntry {
    type Error = RefreshError;

    fn try_from(v: &SourceEntry) -> std::prelude::v1::Result<Self, Self::Error> {
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
        let (dist_path, is_flat) = if components.is_empty() {
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

        let signed_by = options.iter().find_map(|x| {
            x.split_once('=').and_then(|x| {
                if x.0.to_lowercase() == "signed-by" {
                    Some(x.1.to_string())
                } else {
                    None
                }
            })
        });

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
        match dpkg_arch("/") {
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
        self.update_db(get_sources(&self.source)?, callback, handle_topic_msg)
            .await
    }

    async fn update_db<F, F2>(
        &self,
        sourcelist: Vec<OmaSourceEntry>,
        callback: F,
        handle_topic_msg: F2,
    ) -> Result<()>
    where
        F: Fn(usize, RefreshEvent, Option<u64>) + Clone + Send + Sync,
        F2: Fn() -> String + Copy,
    {
        let m = tokio::fs::read(&*MIRROR).await;
        let m = match m {
            Ok(m) => mirror_map(&m).ok(),
            Err(_) => None,
        };

        if !self.download_dir.is_dir() {
            tokio::fs::create_dir_all(&self.download_dir)
                .await
                .map_err(|e| {
                    RefreshError::FailedToOperateDirOrFile(
                        self.download_dir.display().to_string(),
                        e,
                    )
                })?;
        }

        let tasks = self.collect_download_release_tasks(&sourcelist, &m)?;

        let release_results = OmaFetcher::new(None, tasks, Some(self.limit))?
            .start_download(|c, event| callback(c, RefreshEvent::from(event), None))
            .await;

        let all_inrelease = self
            .handle_downloaded_release_result(release_results, callback.clone(), handle_topic_msg)
            .await?;

        let (tasks, total) = self
            .collect_all_release_entry(all_inrelease, &sourcelist, &m)
            .await?;

        let res = OmaFetcher::new(None, tasks, Some(self.limit))?
            .start_download(|count, event| callback(count, RefreshEvent::from(event), Some(total)))
            .await;

        res.into_iter().collect::<DownloadResult<Vec<_>>>()?;

        Ok(())
    }

    fn collect_download_release_tasks(
        &self,
        sourcelist: &[OmaSourceEntry],
        m: &Option<HashMap<String, MirrorMapItem>>,
    ) -> Result<Vec<DownloadEntry>> {
        let mut tasks = Vec::new();
        for source_entry in sourcelist {
            let msg = get_url_short_and_branch(&source_entry.inrelease_path, m)?;

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
                .dir(self.download_dir.clone())
                .allow_resume(false)
                .msg(format!("{msg} InRelease"))
                .build()?;

            debug!("oma will fetch {} InRelease", source_entry.url);
            tasks.push(task);
        }

        Ok(tasks)
    }

    async fn handle_downloaded_release_result<F, F2>(
        &self,
        res: Vec<std::result::Result<Summary, DownloadError>>,
        callback: F,
        handle_topic_msg: F2,
    ) -> Result<Vec<Summary>>
    where
        F: Fn(usize, RefreshEvent, Option<u64>) + Clone + Send + Sync,
        F2: Fn() -> String + Copy,
    {
        let mut all_inrelease = vec![];

        #[cfg(feature = "aosc")]
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
                let removed_suites =
                    oma_topics::scan_closed_topic(handle_topic_msg, &self.source, &self.arch)
                        .await?;
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

        Ok(all_inrelease)
    }

    async fn collect_all_release_entry(
        &self,
        all_inrelease: Vec<Summary>,
        sourcelist: &[OmaSourceEntry],
        m: &Option<HashMap<String, MirrorMapItem>>,
    ) -> Result<(Vec<DownloadEntry>, u64)> {
        let mut total = 0;
        let mut tasks = vec![];
        for inrelease_summary in all_inrelease {
            // 源数据确保是存在的，所以直接 unwrap
            let ose = sourcelist.get(inrelease_summary.count).unwrap();
            let urlc = &ose.url;

            debug!("Getted oma source entry: {:#?}", ose);
            let inrelease_path = self.download_dir.join(&*inrelease_summary.filename);

            let s = tokio::fs::read_to_string(&inrelease_path)
                .await
                .map_err(|e| {
                    RefreshError::FailedToOperateDirOrFile(inrelease_path.display().to_string(), e)
                })?;

            let inrelease = InRelease {
                inrelease: &s,
                trust_files: ose.signed_by.as_deref(),
                mirror: urlc,
                arch: &self.arch,
                is_flat: ose.is_flat,
                p: &inrelease_path,
                rootfs: &self.source,
                components: &ose.components,
            };

            let inrelease = InReleaseParser::new(inrelease).map_err(|err| {
                RefreshError::InReleaseParseError(inrelease_path.display().to_string(), err)
            })?;

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
                let mut handle_file_types = vec![];
                for i in &checksums {
                    match &i.file_type {
                        DistFileType::BinaryContents => {
                            debug!("oma will download Binary Contents: {}", i.name);
                            handle.push(i);
                            total += i.size;
                        }
                        DistFileType::Contents | DistFileType::PackageList
                            if !self.download_compress =>
                        {
                            debug!("oma will download Package List/Contetns: {}", i.name);
                            handle.push(i);
                            total += i.size;
                        }
                        DistFileType::CompressContents(_) => {
                            if self.download_compress {
                                debug!(
                                    "oma will download compress Package List/compress Contetns: {}",
                                    i.name
                                );

                                if !handle.contains(&i)
                                    && !handle_file_types.contains(&&i.file_type)
                                {
                                    handle.push(i);
                                    handle_file_types.push(&i.file_type);
                                    total += i.size;
                                }
                            }
                        }
                        DistFileType::CompressPackageList(s) => {
                            if self.download_compress {
                                debug!(
                                    "oma will download compress Package List/compress Contetns: {}",
                                    i.name
                                );

                                if !handle.contains(&i)
                                    && !handle_file_types.contains(&&i.file_type)
                                {
                                    handle_file_types.push(&i.file_type);
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
                    sourcelist.get(inrelease_summary.count).unwrap(),
                    &checksums,
                    &self.download_dir,
                    &mut tasks,
                    m,
                )?;
            }
        }

        Ok((tasks, total))
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
    let download_url = format!("{}/{}", dist_url, c.name);

    let file_path = if let DistFileType::CompressContents(_) = c.file_type {
        download_url.clone()
    } else {
        format!("{}/{}", dist_url, not_compress_filename_before)
    };

    let from = match source_index.from {
        OmaSourceEntryFrom::Http => DownloadSourceType::Http,
        OmaSourceEntryFrom::Local => DownloadSourceType::Local,
    };

    let sources = vec![DownloadSource::new(download_url.clone(), from)];

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
    debug!("oma will download source database: {download_url}");
    tasks.push(task);

    Ok(())
}
