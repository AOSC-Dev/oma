use std::{
    collections::HashMap,
    path::{Path, PathBuf},
};

use futures::{
    future::{join_all, BoxFuture},
    FutureExt, StreamExt,
};
use oma_apt_sources_lists::{SourceEntry, SourceError};
use oma_fetch::{
    checksum::ChecksumError,
    reqwest::{self, Client},
    CompressFile, DownloadEntry, DownloadEntryBuilder, DownloadEntryBuilderError, DownloadEvent,
    DownloadResult, DownloadSource, DownloadSourceType, OmaFetcher, Summary,
};

use oma_fetch::DownloadError;

#[cfg(feature = "aosc")]
use reqwest::StatusCode;

use tokio::fs;
use tracing::debug;

use crate::{
    inrelease::{ChecksumItem, DistFileType, InRelease, InReleaseParser, InReleaseParserError},
    util::{database_filename, get_sources, human_download_url, MirrorMapItem},
};

const AOSC_MIRROR_FILE: &str = "/usr/share/distro-repository-data/mirrors.yml";

#[cfg(feature = "aosc")]
#[derive(Debug, thiserror::Error)]
pub enum RefreshError {
    #[error("Invalid URL: {0}")]
    InvaildUrl(String),
    #[error("Can not parse distro repo data {0}: {1}")]
    ParseDistroRepoDataError(&'static str, serde_yaml::Error),
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
    #[error("Failed to read download dir: {0}")]
    ReadDownloadDir(String, std::io::Error),
}

#[cfg(not(feature = "aosc"))]
#[derive(Debug, thiserror::Error)]
pub enum RefreshError {
    #[error("Invalid URL: {0}")]
    InvaildUrl(String),
    #[error("Can not parse distro repo data {0}: {1}")]
    ParseDistroRepoDataError(&'static str, serde_yaml::Error),
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
    #[error("Failed to read download dir: {0}")]
    ReadDownloadDir(String, std::io::Error),
}

type Result<T> = std::result::Result<T, RefreshError>;

fn mirror_map(buf: &[u8]) -> Result<HashMap<String, MirrorMapItem>> {
    let mirror_map: HashMap<String, MirrorMapItem> = serde_yaml::from_slice(buf)
        .map_err(|e| RefreshError::ParseDistroRepoDataError(AOSC_MIRROR_FILE, e))?;

    Ok(mirror_map)
}

#[derive(Debug, Clone)]
pub struct OmaSourceEntry {
    from: OmaSourceEntryFrom,
    components: Vec<String>,
    url: String,
    _suite: String,
    dist_path: String,
    is_flat: bool,
    signed_by: Option<String>,
    archs: Vec<String>,
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
        } else if v.url().starts_with("file:") {
            OmaSourceEntryFrom::Local
        } else {
            return Err(RefreshError::UnsupportedProtocol(format!("{v:?}")));
        };

        let components = v.components.clone();
        let url = v.url.clone();
        let suite = v.suite.clone();
        let (dist_path, is_flat) = if components.is_empty() {
            // flat repo 后面一定有斜线
            if suite.starts_with('/') {
                (format!("{}{}", v.url(), suite), true)
            } else {
                (format!("{}/{}", v.url(), suite), true)
            }
        } else {
            (v.dist_path(), false)
        };

        let mut signed_by = None;
        let mut archs = vec![];

        for i in &v.options {
            if i.starts_with("arch=") {
                if let Some(v) = i.split_once('=').map(|x| x.1.to_string()) {
                    for i in v.split(',') {
                        archs.push(i.to_string());
                    }
                }
            }
            if i.starts_with("signed-by=") {
                signed_by = i.split_once('=').map(|x| x.1.to_string());
            }
        }

        Ok(Self {
            from,
            components,
            url,
            _suite: suite,
            is_flat,
            dist_path,
            signed_by,
            archs,
        })
    }
}

pub struct OmaRefreshBuilder<'a> {
    pub source: PathBuf,
    pub limit: Option<usize>,
    pub arch: String,
    pub download_dir: PathBuf,
    pub download_compress: bool,
    pub client: &'a Client,
}

pub struct OmaRefresh<'a> {
    source: PathBuf,
    limit: Option<usize>,
    arch: String,
    download_dir: PathBuf,
    download_compress: bool,
    client: &'a Client,
    flat_repo_no_release: Vec<usize>,
}

impl<'a> From<OmaRefreshBuilder<'a>> for OmaRefresh<'a> {
    fn from(builder: OmaRefreshBuilder<'a>) -> Self {
        Self {
            source: builder.source,
            limit: builder.limit,
            arch: builder.arch,
            download_dir: builder.download_dir,
            download_compress: builder.download_compress,
            client: builder.client,
            flat_repo_no_release: vec![],
        }
    }
}

enum RepoType {
    InRelease,
    Release,
    FlatNoRelease,
}

impl<'a> OmaRefresh<'a> {
    pub async fn start<F, F2>(mut self, _callback: F, _handle_topic_msg: F2) -> Result<()>
    where
        F: Fn(usize, RefreshEvent, Option<u64>) + Clone + Send + Sync,
        F2: Fn() -> String + Copy,
    {
        self.update_db(get_sources(&self.source)?, _callback, _handle_topic_msg)
            .await
    }

    async fn update_db<F, F2>(
        &mut self,
        sourcelist: Vec<OmaSourceEntry>,
        _callback: F,
        _handle_topic_msg: F2,
    ) -> Result<()>
    where
        F: Fn(usize, RefreshEvent, Option<u64>) + Clone + Send + Sync,
        F2: Fn() -> String + Copy,
    {
        let m = tokio::fs::read(AOSC_MIRROR_FILE)
            .await
            .ok()
            .and_then(|m| mirror_map(&m).ok());

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

        let is_inrelease_map = self
            .get_is_inrelease_map(&sourcelist, &m, &_callback)
            .await?;

        let mut download_list = vec![];

        let tasks = self.collect_download_release_tasks(&sourcelist, &m, is_inrelease_map)?;

        for i in &tasks {
            download_list.push(i.filename.to_string());
        }

        let release_results = OmaFetcher::new(self.client, tasks, self.limit)?
            .start_download(|c, event| _callback(c, RefreshEvent::from(event), None))
            .await;

        let all_inrelease = self
            .handle_downloaded_release_result(release_results, _callback.clone(), _handle_topic_msg)
            .await?;

        let (tasks, total) = self
            .collect_all_release_entry(all_inrelease, &sourcelist, &m)
            .await?;

        for i in &tasks {
            download_list.push(i.filename.to_string());
        }

        let download_dir = self.download_dir.clone();
        tokio::spawn(async move { remove_unused_db(download_dir, download_list).await });

        let res = OmaFetcher::new(self.client, tasks, self.limit)?
            .start_download(|count, event| _callback(count, RefreshEvent::from(event), Some(total)))
            .await;

        res.into_iter().collect::<DownloadResult<Vec<_>>>()?;

        Ok(())
    }

    async fn get_is_inrelease_map<F>(
        &mut self,
        sourcelist: &[OmaSourceEntry],
        m: &Option<HashMap<String, MirrorMapItem>>,
        callback: &F,
    ) -> Result<HashMap<usize, RepoType>>
    where
        F: Fn(usize, RefreshEvent, Option<u64>) + Clone + Send + Sync,
    {
        let mut tasks = vec![];

        let mut mirrors_inrelease = HashMap::new();

        for (i, c) in sourcelist.iter().enumerate() {
            let mut tasks1 = vec![];
            match c.from {
                OmaSourceEntryFrom::Http => {
                    let resp1: BoxFuture<'_, _> = Box::pin(
                        self.client
                            .head(format!("{}/InRelease", c.dist_path))
                            .send()
                            .map(move |x| (x.and_then(|x| x.error_for_status()), i)),
                    );

                    let resp2 = Box::pin(
                        self.client
                            .head(format!("{}/Release", c.dist_path))
                            .send()
                            .map(move |x| (x.and_then(|x| x.error_for_status()), i)),
                    );

                    tasks1.push(resp1);
                    tasks1.push(resp2);

                    if c.is_flat {
                        let resp3 = Box::pin(
                            self.client
                                .head(format!("{}/Packages", c.dist_path))
                                .send()
                                .map(move |x| (x.and_then(|x| x.error_for_status()), i)),
                        );
                        tasks1.push(resp3);
                    }

                    let event =
                        RefreshEvent::DownloadEvent(DownloadEvent::NewProgressSpinner(format!(
                            "({}/{}) {}",
                            i,
                            sourcelist.len(),
                            human_download_url(&c.dist_path, m)?
                        )));

                    let cc = callback.clone();

                    let task = async move {
                        cc(i, event, None);
                        let res = join_all(tasks1).await;
                        cc(
                            i,
                            RefreshEvent::DownloadEvent(DownloadEvent::ProgressDone),
                            None,
                        );
                        res
                    };

                    tasks.push(task);
                }
                OmaSourceEntryFrom::Local => {
                    let event =
                        RefreshEvent::DownloadEvent(DownloadEvent::NewProgressSpinner(format!(
                            "({}/{}) {}",
                            i,
                            sourcelist.len(),
                            human_download_url(&c.dist_path, m)?
                        )));

                    callback(i, event, None);

                    let dist_path = c.dist_path.strip_prefix("file:").unwrap_or(&c.dist_path);

                    let p1 = Path::new(dist_path).join("InRelease");
                    let p2 = Path::new(dist_path).join("Release");

                    if p1.exists() {
                        mirrors_inrelease.insert(i, RepoType::InRelease);
                    } else if p2.exists() {
                        mirrors_inrelease.insert(i, RepoType::Release);
                    } else if c.is_flat {
                        // flat repo 可以没有 Release 文件
                        mirrors_inrelease.insert(i, RepoType::FlatNoRelease);
                        self.flat_repo_no_release.push(i);
                        continue;
                    } else {
                        callback(
                            i,
                            RefreshEvent::DownloadEvent(DownloadEvent::ProgressDone),
                            None,
                        );
                        #[cfg(feature = "aosc")]
                        // FIXME: 为了能让 oma refresh 正确关闭 topic，这里先忽略错误
                        mirrors_inrelease.insert(i, RepoType::InRelease);
                        #[cfg(not(feature = "aosc"))]
                        return Err(RefreshError::NoInReleaseFile(c.dist_path.clone()));
                    }

                    callback(
                        i,
                        RefreshEvent::DownloadEvent(DownloadEvent::ProgressDone),
                        None,
                    );
                }
            }
        }

        let stream = futures::stream::iter(tasks).buffer_unordered(self.limit.unwrap_or(4));
        let res = stream.collect::<Vec<_>>().await;

        for i in res {
            if let Some((Ok(_), j)) = i.first() {
                mirrors_inrelease.insert(*j, RepoType::InRelease);
                continue;
            }

            if let Some((Ok(_), j)) = i.get(1) {
                mirrors_inrelease.insert(*j, RepoType::Release);
                continue;
            }

            if let Some((Ok(_), j)) = i.get(2) {
                mirrors_inrelease.insert(*j, RepoType::FlatNoRelease);
                self.flat_repo_no_release.push(*j);
                continue;
            }

            #[cfg(feature = "aosc")]
            // FIXME: 为了能让 oma refresh 正确关闭 topic，这里先忽略错误
            mirrors_inrelease.insert(i.first().unwrap().1, RepoType::InRelease);

            #[cfg(not(feature = "aosc"))]
            return Err(RefreshError::NoInReleaseFile(
                sourcelist[i.first().unwrap().1].dist_path.clone(),
            ));
        }

        Ok(mirrors_inrelease)
    }

    fn collect_download_release_tasks(
        &self,
        sourcelist: &[OmaSourceEntry],
        m: &Option<HashMap<String, MirrorMapItem>>,
        is_inrelease_map: HashMap<usize, RepoType>,
    ) -> Result<Vec<DownloadEntry>> {
        let mut tasks = Vec::new();
        for (i, source_entry) in sourcelist.iter().enumerate() {
            let repo_type = is_inrelease_map.get(&i).unwrap();

            let repo_type_str = match repo_type {
                RepoType::InRelease => "InRelease",
                RepoType::Release => "Release",
                RepoType::FlatNoRelease => continue,
            };

            let uri = format!(
                "{}{}{}",
                source_entry.dist_path,
                if !source_entry.dist_path.ends_with('/') {
                    "/"
                } else {
                    ""
                },
                repo_type_str
            );

            let msg = human_download_url(&uri, m)?;

            let sources = vec![DownloadSource::new(
                uri.clone(),
                match source_entry.from {
                    OmaSourceEntryFrom::Http => DownloadSourceType::Http,
                    // 为保持与 apt 行为一致，本地源 symlink Release 文件
                    OmaSourceEntryFrom::Local => DownloadSourceType::Local { as_symlink: true },
                },
            )];

            let task = DownloadEntryBuilder::default()
                .source(sources)
                .filename(database_filename(&uri)?.into())
                .dir(self.download_dir.clone())
                .allow_resume(false)
                .msg(format!("{msg} {}", repo_type_str))
                .build()?;

            debug!("oma will fetch {} InRelease", source_entry.url);
            tasks.push(task);
        }

        Ok(tasks)
    }

    async fn handle_downloaded_release_result<F, F2>(
        &self,
        res: Vec<std::result::Result<Summary, DownloadError>>,
        _callback: F,
        _handle_topic_msg: F2,
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
            let removed_suites =
                oma_topics::scan_closed_topic(_handle_topic_msg, &self.source, &self.arch).await?;

            for url in not_found {
                let suite = url
                    .path_segments()
                    .and_then(|mut x| x.nth_back(1).map(|x| x.to_string()))
                    .ok_or_else(|| RefreshError::InvaildUrl(url.to_string()))?;

                if !removed_suites.contains(&suite) {
                    return Err(RefreshError::NoInReleaseFile(url.to_string()));
                }

                _callback(0, RefreshEvent::ClosingTopic(suite), None);
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

            let inrelease = tokio::fs::read_to_string(&inrelease_path)
                .await
                .map_err(|e| {
                    RefreshError::FailedToOperateDirOrFile(inrelease_path.display().to_string(), e)
                })?;

            let archs = if ose.archs.is_empty() {
                vec![self.arch.clone()]
            } else {
                ose.archs.clone()
            };

            let inrelease = InRelease {
                inrelease: &inrelease,
                signed_by: ose.signed_by.as_deref(),
                mirror: urlc,
                archs: &archs,
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
                let mut handle_file_name = vec![];
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
                            debug!("oma will download Package List/Contents: {}", i.name);
                            handle.push(i);
                            total += i.size;
                        }
                        DistFileType::CompressContents(name, _) => {
                            if self.download_compress {
                                debug!(
                                    "oma will download compress Package List/compress Contetns: {}",
                                    i.name
                                );

                                if !handle_file_name.contains(&name) {
                                    handle.push(i);
                                    handle_file_name.push(name);
                                    total += i.size;
                                }
                            }
                        }
                        DistFileType::CompressPackageList(name, _) => {
                            if self.download_compress {
                                debug!(
                                    "oma will download compress Package List/compress Contetns: {}",
                                    i.name
                                );

                                if !handle_file_name.contains(&name) {
                                    handle.push(i);
                                    handle_file_name.push(name);
                                    let size = checksums
                                        .iter()
                                        .find_map(
                                            |x| if x.name == *name { Some(x.size) } else { None },
                                        )
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

            for i in &self.flat_repo_no_release {
                download_flat_repo_no_release(
                    sourcelist.get(*i).unwrap(),
                    &self.download_dir,
                    &mut tasks,
                )?;
            }

            for c in handle {
                collect_download_task(
                    c,
                    sourcelist.get(inrelease_summary.count).unwrap(),
                    &checksums,
                    &self.download_dir,
                    &mut tasks,
                    m,
                    inrelease.acquire_by_hash,
                )?;
            }
        }

        Ok((tasks, total))
    }
}

async fn remove_unused_db(download_dir: PathBuf, download_list: Vec<String>) -> Result<()> {
    let mut download_dir = fs::read_dir(&download_dir)
        .await
        .map_err(|e| RefreshError::ReadDownloadDir(download_dir.display().to_string(), e))?;

    while let Ok(Some(x)) = download_dir.next_entry().await {
        if x.path().is_file()
            && !download_list.contains(&x.file_name().to_string_lossy().to_string())
        {
            debug!("Removing {:?}", x.file_name());
            if let Err(e) = tokio::fs::remove_file(x.path()).await {
                debug!("Failed to remove file {:?}: {e}", x.file_name());
            }
        }
    }

    Ok(())
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

fn download_flat_repo_no_release(
    source_index: &OmaSourceEntry,
    download_dir: &Path,
    tasks: &mut Vec<DownloadEntry>,
) -> Result<()> {
    let msg = human_download_url(&source_index.dist_path, &None)?;

    let dist_url = &source_index.dist_path;

    let from = match source_index.from {
        OmaSourceEntryFrom::Http => DownloadSourceType::Http,
        OmaSourceEntryFrom::Local => DownloadSourceType::Local { as_symlink: true },
    };

    let download_url = format!("{}/Packages", dist_url);
    let file_path = format!("{}Packages", dist_url);

    let sources = vec![DownloadSource::new(download_url.clone(), from)];

    let mut task = DownloadEntryBuilder::default();
    task.source(sources);
    task.filename(database_filename(&file_path)?.into());
    task.dir(download_dir.to_path_buf());
    task.allow_resume(false);
    task.msg(format!("{msg} Packages"));
    task.file_type(CompressFile::Nothing);

    let task = task.build()?;
    debug!("oma will download source database: {download_url}");
    tasks.push(task);

    Ok(())
}

fn collect_download_task(
    c: &ChecksumItem,
    source_index: &OmaSourceEntry,
    checksums: &[ChecksumItem],
    download_dir: &Path,
    tasks: &mut Vec<DownloadEntry>,
    m: &Option<HashMap<String, MirrorMapItem>>,
    acquire_by_hash: bool,
) -> Result<()> {
    let (typ, not_compress_filename_before) = match &c.file_type {
        DistFileType::CompressContents(s, _) => ("Contents", s),
        DistFileType::Contents => ("Contents", &c.name),
        DistFileType::CompressPackageList(s, _) => ("Package List", s),
        DistFileType::PackageList => ("Package List", &c.name),
        DistFileType::BinaryContents => ("BinContents", &c.name),
        _ => unreachable!(),
    };

    let msg = human_download_url(&source_index.dist_path, m)?;

    let dist_url = &source_index.dist_path;

    let from = match source_index.from {
        OmaSourceEntryFrom::Http => DownloadSourceType::Http,
        OmaSourceEntryFrom::Local => DownloadSourceType::Local { as_symlink: true },
    };

    let checksum = if matches!(c.file_type, DistFileType::CompressContents(_, _)) {
        Some(&c.checksum)
    } else {
        checksums
            .iter()
            .find(|x| &x.name == not_compress_filename_before)
            .as_ref()
            .map(|c| &c.checksum)
    };

    let download_url = if acquire_by_hash {
        let path = Path::new(&c.name);
        let parent = path.parent().unwrap_or(path);
        let path = parent.join("by-hash").join("SHA256").join(&c.checksum);

        format!("{}/{}", dist_url, path.display())
    } else {
        format!("{}/{}", dist_url, c.name)
    };

    let sources = vec![DownloadSource::new(download_url.clone(), from)];

    let file_path = if let DistFileType::CompressContents(_, _) = c.file_type {
        download_url.clone()
    } else if dist_url.ends_with('/') {
        format!("{}{}", dist_url, not_compress_filename_before)
    } else {
        format!("{}/{}", dist_url, not_compress_filename_before)
    };

    let mut task = DownloadEntryBuilder::default();
    task.source(sources);
    task.filename(database_filename(&file_path)?.into());
    task.dir(download_dir.to_path_buf());
    task.allow_resume(false);
    task.msg(format!("{msg} {typ}"));
    task.file_type(match c.file_type {
        DistFileType::BinaryContents => CompressFile::Nothing,
        DistFileType::Contents => CompressFile::Nothing,
        // 不解压 Contents
        DistFileType::CompressContents(_, _) => CompressFile::Nothing,
        DistFileType::PackageList => CompressFile::Nothing,
        DistFileType::CompressPackageList(_, _) => {
            match Path::new(&c.name).extension().and_then(|x| x.to_str()) {
                Some("gz") => CompressFile::Gzip,
                Some("xz") => CompressFile::Xz,
                Some("bz2") => CompressFile::Bz2,
                Some(x) => {
                    debug!("unsupport file type: {x}");
                    return Ok(());
                }
                None => unreachable!(),
            }
        }
        DistFileType::Release => CompressFile::Nothing,
    });

    if let Some(checksum) = checksum {
        task.hash(checksum);
    }

    let task = task.build()?;
    debug!("oma will download source database: {download_url}");
    tasks.push(task);

    Ok(())
}
