use std::{
    borrow::Cow,
    path::{Path, PathBuf},
};

use ahash::AHashMap;
use aho_corasick::BuildError;
use futures::{
    future::{join_all, BoxFuture},
    FutureExt, StreamExt,
};
use oma_apt::config::Config;
use oma_apt_sources_lists::SourceError;
use oma_fetch::{
    checksum::{Checksum, ChecksumError},
    reqwest::{self, Client},
    CompressFile, DownloadEntry, DownloadEvent, DownloadResult, DownloadSource, DownloadSourceType,
    OmaFetcher, Summary,
};

use oma_fetch::DownloadError;

#[cfg(feature = "aosc")]
use reqwest::StatusCode;

use smallvec::SmallVec;
use tokio::{fs, process::Command};
use tracing::{debug, warn};

use crate::{
    auth::{AuthConfig, AuthConfigError},
    config::{fiilter_download_list, get_config, ChecksumDownloadEntry},
    inrelease::{
        file_is_compress, split_ext_and_filename, ChecksumType, InRelease, InReleaseParser,
        InReleaseParserError,
    },
    util::{
        get_sources, human_download_url, DatabaseFilenameReplacer, OmaSourceEntry,
        OmaSourceEntryFrom,
    },
};

#[cfg(feature = "aosc")]
#[derive(Debug, thiserror::Error)]
pub enum RefreshError {
    #[error("Invalid URL: {0}")]
    InvaildUrl(String),
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
    ChecksumError(#[from] ChecksumError),
    #[error("Failed to operate dir or file {0}: {1}")]
    FailedToOperateDirOrFile(String, tokio::io::Error),
    #[error("Failed to parse InRelease file: {0}")]
    InReleaseParseError(String, InReleaseParserError),
    #[error("Failed to read download dir: {0}")]
    ReadDownloadDir(String, std::io::Error),
    #[error(transparent)]
    AhoCorasickBuilder(#[from] BuildError),
    #[error("stream_replace_all failed")]
    ReplaceAll(std::io::Error),
    #[error(transparent)]
    AuthConfig(#[from] AuthConfigError),
}

#[cfg(not(feature = "aosc"))]
#[derive(Debug, thiserror::Error)]
pub enum RefreshError {
    #[error("Invalid URL: {0}")]
    InvaildUrl(String),
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
    ChecksumError(#[from] ChecksumError),
    #[error("Failed to operate dir or file {0}: {1}")]
    FailedToOperateDirOrFile(String, tokio::io::Error),
    #[error("Failed to parse InRelease file: {0}")]
    InReleaseParseError(String, InReleaseParserError),
    #[error("Failed to read download dir: {0}")]
    ReadDownloadDir(String, std::io::Error),
    #[error(transparent)]
    AhoCorasickBuilder(#[from] BuildError),
    #[error("stream_replace_all failed")]
    ReplaceAll(std::io::Error),
    #[error(transparent)]
    AuthConfig(#[from] AuthConfigError),
}

type Result<T> = std::result::Result<T, RefreshError>;

pub enum Event {
    Info(String),
}

pub struct OmaRefreshBuilder<'a> {
    pub source: PathBuf,
    pub limit: Option<usize>,
    pub arch: String,
    pub download_dir: PathBuf,
    pub client: &'a Client,
    #[cfg(feature = "aosc")]
    pub refresh_topics: bool,
    pub apt_config: &'a Config,
}

pub struct OmaRefresh<'a> {
    source: PathBuf,
    limit: Option<usize>,
    arch: String,
    download_dir: PathBuf,
    client: &'a Client,
    flat_repo_no_release: Vec<usize>,
    #[cfg(feature = "aosc")]
    refresh_topics: bool,
    apt_config: &'a Config,
    auth_config: Option<AuthConfig>,
}

impl<'a> From<OmaRefreshBuilder<'a>> for OmaRefresh<'a> {
    fn from(builder: OmaRefreshBuilder<'a>) -> Self {
        let auth = AuthConfig::system(&builder.source).ok();
        Self {
            source: builder.source,
            limit: builder.limit,
            arch: builder.arch,
            download_dir: builder.download_dir,
            client: builder.client,
            flat_repo_no_release: vec![],
            #[cfg(feature = "aosc")]
            refresh_topics: builder.refresh_topics,
            apt_config: builder.apt_config,
            auth_config: auth,
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

        let is_inrelease_map = self.get_is_inrelease_map(&sourcelist, &_callback).await?;

        let mut download_list = vec![];

        let replacer = DatabaseFilenameReplacer::new()?;
        let (tasks, soueces_map) =
            self.collect_download_release_tasks(&sourcelist, is_inrelease_map, &replacer)?;

        for i in &tasks {
            download_list.push(i.filename.to_string());
        }

        let release_results = OmaFetcher::new(self.client, tasks, self.limit)?
            .start_download(|c, event| _callback(c, RefreshEvent::from(event), None))
            .await;

        let all_inrelease = self
            .handle_downloaded_release_result(release_results, _callback.clone(), _handle_topic_msg)
            .await?;

        let config_tree = get_config(self.apt_config);

        let (tasks, total) = self
            .collect_all_release_entry(
                all_inrelease,
                &sourcelist,
                &replacer,
                soueces_map,
                &config_tree,
            )
            .await?;

        for i in &tasks {
            download_list.push(i.filename.to_string());
        }

        let download_dir = self.download_dir.clone();
        let remove_task =
            tokio::spawn(async move { remove_unused_db(download_dir, download_list).await });

        let res = OmaFetcher::new(self.client, tasks, self.limit)?
            .start_download(|count, event| _callback(count, RefreshEvent::from(event), Some(total)))
            .await;

        res.into_iter().collect::<DownloadResult<Vec<_>>>()?;

        // Finally, run success post invoke
        let _ = remove_task.await;
        Self::run_success_post_invoke(&config_tree).await;

        Ok(())
    }

    async fn run_success_post_invoke(config_tree: &[(String, String)]) {
        let cmds = config_tree
            .iter()
            .filter(|x| x.0 == "APT::Update::Post-Invoke-Success::");

        for (_, cmd) in cmds {
            debug!("Running post-invoke script: {cmd}");
            let output = Command::new("sh").arg("-c").arg(cmd).output().await;

            match output {
                Ok(output) => {
                    if !output.status.success() {
                        warn!(
                            "Run {cmd} return non-zero code: {}",
                            output.status.code().unwrap_or(1)
                        );
                        continue;
                    }
                    debug!("Run {cmd} success");
                }
                Err(e) => {
                    warn!("Run {cmd} failed: {e}");
                }
            }
        }
    }

    async fn get_is_inrelease_map<F>(
        &mut self,
        sourcelist: &[OmaSourceEntry],
        callback: &F,
    ) -> Result<AHashMap<usize, RepoType>>
    where
        F: Fn(usize, RefreshEvent, Option<u64>) + Clone + Send + Sync,
    {
        let mut tasks = vec![];

        let mut mirrors_inrelease = AHashMap::new();

        for (i, c) in sourcelist.iter().enumerate() {
            let mut tasks1 = vec![];
            match c.from {
                OmaSourceEntryFrom::Http => {
                    let resp1: BoxFuture<'_, _> = Box::pin({
                        let mut client = self.client.head(format!("{}/InRelease", c.dist_path));

                        if let Some(ac) = &self.auth_config.as_ref().and_then(|x| x.find(&c.url)) {
                            client = client.basic_auth(&ac.user, Some(&ac.password));
                        }

                        client
                            .send()
                            .map(move |x| (x.and_then(|x| x.error_for_status()), i))
                    });

                    let resp2 = Box::pin({
                        let mut client = self.client.head(format!("{}/Release", c.dist_path));

                        if let Some(auth) = &self.auth_config.as_ref().and_then(|x| x.find(&c.url))
                        {
                            client = client.basic_auth(&auth.user, Some(&auth.password));
                        }

                        client
                            .send()
                            .map(move |x| (x.and_then(|x| x.error_for_status()), i))
                    });

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
                            human_download_url(c, None)?
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
                            human_download_url(c, None)?
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
                        self.flat_repo_no_release.push(i);
                        mirrors_inrelease.insert(i, RepoType::FlatNoRelease);
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
        is_inrelease_map: AHashMap<usize, RepoType>,
        replacer: &DatabaseFilenameReplacer,
    ) -> Result<(Vec<DownloadEntry>, AHashMap<String, OmaSourceEntry>)> {
        let mut tasks = Vec::new();
        let mut map = AHashMap::new();

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

            let msg = human_download_url(source_entry, Some(repo_type_str))?;

            let sources = vec![DownloadSource::new(
                uri.clone(),
                match source_entry.from {
                    OmaSourceEntryFrom::Http => DownloadSourceType::Http,
                    // 为保持与 apt 行为一致，本地源 symlink Release 文件
                    OmaSourceEntryFrom::Local => DownloadSourceType::Local {
                        as_symlink: source_entry.is_flat,
                    },
                },
            )];

            let task = DownloadEntry::builder()
                .source(sources)
                .filename(replacer.replace(&uri)?)
                .dir(self.download_dir.clone())
                .allow_resume(false)
                .msg(msg)
                .build();

            debug!("oma will fetch {} InRelease", source_entry.url);
            map.insert(task.filename.to_string(), source_entry.clone());
            tasks.push(task);
        }

        Ok((tasks, map))
    }

    async fn handle_downloaded_release_result<F>(
        &self,
        res: Vec<std::result::Result<Summary, DownloadError>>,
        _callback: F,
        _handle_topic_msg: impl Fn() -> String + Copy,
    ) -> Result<Vec<Summary>>
    where
        F: Fn(usize, RefreshEvent, Option<u64>) + Clone + Send + Sync,
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
                                    .unwrap_or(false)
                                    && self.refresh_topics =>
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
            if self.refresh_topics {
                _callback(0, RefreshEvent::ScanningTopic, None);
                let removed_suites = oma_topics::scan_closed_topic(
                    _handle_topic_msg,
                    |topic, mirror| {
                        _callback(
                            0,
                            RefreshEvent::TopicNotInMirror(topic.to_string(), mirror.to_string()),
                            None,
                        );
                    },
                    &self.source,
                    &self.arch,
                )
                .await?;

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
        }

        Ok(all_inrelease)
    }

    async fn collect_all_release_entry(
        &self,
        all_inrelease: Vec<Summary>,
        sourcelist: &[OmaSourceEntry],
        replacer: &DatabaseFilenameReplacer,
        sources_map: AHashMap<String, OmaSourceEntry>,
        config_tree: &[(String, String)],
    ) -> Result<(Vec<DownloadEntry>, u64)> {
        let mut total = 0;
        let mut tasks = vec![];
        for inrelease_summary in all_inrelease {
            // 源数据确保是存在的，所以直接 unwrap
            let ose = sources_map.get(&inrelease_summary.filename).unwrap();
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
                is_flat: ose.is_flat,
                p: &inrelease_path,
                rootfs: &self.source,
                trusted: ose.trusted,
            };

            let inrelease = InReleaseParser::new(inrelease).map_err(|err| {
                RefreshError::InReleaseParseError(inrelease_path.display().to_string(), err)
            })?;

            let mut handle = vec![];
            let filter_checksums = fiilter_download_list(
                &inrelease.checksums,
                self.apt_config,
                config_tree,
                &archs,
                &ose.components,
                &ose.native_arch,
                ose.is_flat,
            );

            get_all_need_db_from_config(filter_checksums, &mut total, &inrelease, &mut handle);

            for i in &self.flat_repo_no_release {
                download_flat_repo_no_release(
                    sourcelist.get(*i).unwrap(),
                    &self.download_dir,
                    &mut tasks,
                    replacer,
                )?;
            }

            for c in handle {
                collect_download_task(
                    &c,
                    ose,
                    &self.download_dir,
                    &mut tasks,
                    &inrelease,
                    replacer,
                )?;
            }
        }

        Ok((tasks, total))
    }
}

fn get_all_need_db_from_config(
    filter_checksums: SmallVec<[ChecksumDownloadEntry; 32]>,
    total: &mut u64,
    inrelease: &InReleaseParser,
    handle: &mut Vec<ChecksumDownloadEntry>,
) {
    for i in filter_checksums {
        if i.keep_compress {
            *total += i.item.size;
        } else {
            let size = if file_is_compress(&i.item.name) {
                let (_, name_without_compress) = split_ext_and_filename(&i.item.name);

                inrelease
                    .checksums
                    .iter()
                    .find_map(|x| {
                        if x.name == name_without_compress {
                            Some(x.size)
                        } else {
                            None
                        }
                    })
                    .unwrap_or(i.item.size)
            } else {
                i.item.size
            };

            *total += size;
        }

        handle.push(i);
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
    ScanningTopic,
    TopicNotInMirror(String, String),
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
    replacer: &DatabaseFilenameReplacer,
) -> Result<()> {
    let msg = human_download_url(source_index, Some("Packages"))?;

    let dist_url = &source_index.dist_path;

    let from = match source_index.from {
        OmaSourceEntryFrom::Http => DownloadSourceType::Http,
        OmaSourceEntryFrom::Local => DownloadSourceType::Local {
            as_symlink: source_index.is_flat,
        },
    };

    let download_url = format!("{}/Packages", dist_url);
    let file_path = format!("{}Packages", dist_url);

    let sources = vec![DownloadSource::new(download_url.clone(), from)];

    let task = DownloadEntry::builder()
        .source(sources)
        .filename(replacer.replace(&file_path)?)
        .dir(download_dir.to_path_buf())
        .allow_resume(false)
        .msg(msg)
        .file_type(CompressFile::Nothing)
        .build();

    debug!("oma will download source database: {download_url}");
    tasks.push(task);

    Ok(())
}

fn collect_download_task(
    c: &ChecksumDownloadEntry,
    source_index: &OmaSourceEntry,
    download_dir: &Path,
    tasks: &mut Vec<DownloadEntry>,
    inrelease: &InReleaseParser,
    replacer: &DatabaseFilenameReplacer,
) -> Result<()> {
    let typ = &c.msg;

    let msg = human_download_url(source_index, Some(typ))?;

    let dist_url = &source_index.dist_path;

    let from = match source_index.from {
        OmaSourceEntryFrom::Http => DownloadSourceType::Http,
        OmaSourceEntryFrom::Local => DownloadSourceType::Local {
            as_symlink: source_index.is_flat,
        },
    };

    let not_compress_filename_before = if file_is_compress(&c.item.name) {
        Cow::Owned(split_ext_and_filename(&c.item.name).1)
    } else {
        Cow::Borrowed(&c.item.name)
    };

    let checksum = if c.keep_compress {
        Some(&c.item.checksum)
    } else {
        inrelease
            .checksums
            .iter()
            .find(|x| x.name == *not_compress_filename_before)
            .as_ref()
            .map(|c| &c.checksum)
    };

    let download_url = if inrelease.acquire_by_hash {
        let path = Path::new(&c.item.name);
        let parent = path.parent().unwrap_or(path);
        let dir = match inrelease.checksum_type {
            ChecksumType::Sha256 => "SHA256",
            ChecksumType::Sha512 => "SHA512",
            ChecksumType::Md5 => "MD5Sum",
        };

        let path = parent.join("by-hash").join(dir).join(&c.item.checksum);

        format!("{}/{}", dist_url, path.display())
    } else if dist_url.ends_with('/') {
        format!("{}{}", dist_url, c.item.name)
    } else {
        format!("{}/{}", dist_url, c.item.name)
    };

    let sources = vec![DownloadSource::new(download_url.clone(), from)];

    let file_path = if c.keep_compress {
        if inrelease.acquire_by_hash {
            format!("{}/{}", dist_url, c.item.name)
        } else {
            download_url.clone()
        }
    } else if dist_url.ends_with('/') {
        format!("{}{}", dist_url, not_compress_filename_before)
    } else {
        format!("{}/{}", dist_url, not_compress_filename_before)
    };

    let task = DownloadEntry::builder()
        .source(sources)
        .filename(replacer.replace(&file_path)?)
        .dir(download_dir.to_path_buf())
        .allow_resume(false)
        .msg(msg)
        .file_type({
            if c.keep_compress {
                CompressFile::Nothing
            } else {
                match Path::new(&c.item.name).extension().and_then(|x| x.to_str()) {
                    Some("gz") => CompressFile::Gzip,
                    Some("xz") => CompressFile::Xz,
                    Some("bz2") => CompressFile::Bz2,
                    Some("zst") => CompressFile::Zstd,
                    _ => CompressFile::Nothing,
                }
            }
        })
        .maybe_hash(if let Some(checksum) = checksum {
            match inrelease.checksum_type {
                ChecksumType::Sha256 => Some(Checksum::from_sha256_str(checksum)?),
                ChecksumType::Sha512 => Some(Checksum::from_sha512_str(checksum)?),
                ChecksumType::Md5 => Some(Checksum::from_md5_str(checksum)?),
            }
        } else {
            None
        })
        .build();

    debug!("oma will download source database: {download_url}");
    tasks.push(task);

    Ok(())
}
