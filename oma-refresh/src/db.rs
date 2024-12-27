use std::future::Future;
use std::{
    borrow::Cow,
    fs::Permissions,
    os::{fd::AsRawFd, unix::fs::PermissionsExt},
    path::{Path, PathBuf},
};

use ahash::{AHashMap, HashSet};
use aho_corasick::BuildError;
use apt_auth_config::AuthConfig;
use bon::{builder, Builder};
use chrono::Utc;
use futures::StreamExt;
use nix::{
    errno::Errno,
    fcntl::{
        fcntl, open,
        FcntlArg::{F_GETLK, F_SETFD, F_SETLK},
        FdFlag, OFlag,
    },
    libc::{flock, F_WRLCK, SEEK_SET},
    sys::stat::Mode,
    unistd::close,
};
use oma_apt::config::Config;
use oma_apt_sources_lists::SourceError;
use oma_fetch::{
    checksum::{Checksum, ChecksumError},
    reqwest::{
        self,
        header::{HeaderValue, CONTENT_LENGTH},
        Client, Response,
    },
    CompressFile, DownloadEntry, DownloadManager, DownloadResult, DownloadSource,
    DownloadSourceType,
};

use oma_fetch::{DownloadError, Summary};

#[cfg(feature = "aosc")]
use oma_topics::TopicManager;

use oma_utils::dpkg::dpkg_arch;
#[cfg(feature = "aosc")]
use reqwest::StatusCode;

use sysinfo::{Pid, System};
use tokio::{
    fs::{self, File},
    io::AsyncWriteExt,
    process::Command,
    task::spawn_blocking,
};
use tracing::{debug, warn};

use crate::{
    config::{ChecksumDownloadEntry, IndexTargetConfig},
    inrelease::{
        file_is_compress, split_ext_and_filename, verify_inrelease, ChecksumItem, InRelease,
        InReleaseChecksum, InReleaseError,
    },
    sourceslist::{sources_lists, OmaSourceEntry, OmaSourceEntryFrom},
    util::DatabaseFilenameReplacer,
};

#[derive(Debug, thiserror::Error)]
pub enum RefreshError {
    #[error("Invalid URL: {0}")]
    InvalidUrl(String),
    #[error("Scan sources.list failed: {0}")]
    ScanSourceError(SourceError),
    #[error("Unsupported Protocol: {0}")]
    UnsupportedProtocol(String),
    #[error(transparent)]
    FetcherError(#[from] oma_fetch::DownloadError),
    #[error(transparent)]
    ReqwestError(#[from] reqwest::Error),
    #[cfg(feature = "aosc")]
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
    InReleaseParseError(PathBuf, InReleaseError),
    #[error("Failed to read download dir: {0}")]
    ReadDownloadDir(String, std::io::Error),
    #[error(transparent)]
    AhoCorasickBuilder(#[from] BuildError),
    #[error("stream_replace_all failed")]
    ReplaceAll(std::io::Error),
    #[error("Set lock failed")]
    SetLock(Errno),
    #[error("Set lock failed: process {0} ({1}) is using.")]
    SetLockWithProcess(String, i32),
    #[error("duplicate components")]
    DuplicateComponents(Box<str>, String),
}

type Result<T> = std::result::Result<T, RefreshError>;

#[derive(Builder)]
pub struct OmaRefresh<'a> {
    source: PathBuf,
    #[builder(default = 4)]
    threads: usize,
    arch: String,
    download_dir: PathBuf,
    client: &'a Client,
    #[builder(skip)]
    flat_repo_no_release: Vec<usize>,
    #[cfg(feature = "aosc")]
    refresh_topics: bool,
    apt_config: &'a Config,
    topic_msg: &'a str,
    auth_config: &'a AuthConfig,
}

type SourceMap<'a> = AHashMap<String, Vec<&'a OmaSourceEntry<'a>>>;

fn get_apt_update_lock(download_dir: &Path) -> Result<()> {
    let lock_path = download_dir.join("lock");

    let fd = open(
        &lock_path,
        OFlag::O_RDWR | OFlag::O_CREAT | OFlag::O_NOFOLLOW,
        Mode::from_bits_truncate(0o640),
    )
    .map_err(RefreshError::SetLock)?;

    fcntl(fd, F_SETFD(FdFlag::FD_CLOEXEC)).map_err(|e| {
        close(fd).ok();
        RefreshError::SetLock(e)
    })?;

    // From apt libapt-pkg/fileutil.cc:287
    let mut fl = flock {
        l_type: F_WRLCK as i16,
        l_whence: SEEK_SET as i16,
        l_start: 0,
        l_len: 0,
        l_pid: -1,
    };

    if let Err(e) = fcntl(fd.as_raw_fd(), F_SETLK(&fl)) {
        debug!("{e}");

        if e == Errno::EACCES || e == Errno::EAGAIN {
            fl.l_type = F_WRLCK as i16;
            fl.l_whence = SEEK_SET as i16;
            fl.l_len = 0;
            fl.l_start = 0;
            fl.l_pid = -1;
            fcntl(fd.as_raw_fd(), F_GETLK(&mut fl)).ok();
        } else {
            fl.l_pid = -1;
        }

        close(fd).map_err(RefreshError::SetLock)?;

        if fl.l_pid != -1 {
            let mut sys = System::new();
            sys.refresh_processes(sysinfo::ProcessesToUpdate::All, true);
            let Some(process) = sys.process(Pid::from(fl.l_pid as usize)) else {
                return Err(RefreshError::SetLock(e));
            };

            return Err(RefreshError::SetLockWithProcess(
                process.name().to_string_lossy().to_string(),
                fl.l_pid,
            ));
        }

        return Err(RefreshError::SetLock(e));
    }

    Ok(())
}

#[derive(Debug)]
pub enum Event {
    DownloadEvent(oma_fetch::Event),
    ScanningTopic,
    ClosingTopic(String),
    TopicNotInMirror { topic: String, mirror: String },
    RunInvokeScript,
    Done,
}

#[derive(Debug)]
enum KeyOrIndex<'a> {
    Key(&'a str),
    Index(usize),
}

impl<'a> OmaRefresh<'a> {
    pub async fn start<F, Fut>(mut self, callback: F) -> Result<()>
    where
        F: Fn(Event) -> Fut,
        Fut: Future<Output = ()>,
    {
        let arch = dpkg_arch(&self.source)?;

        self.update_db(sources_lists(&self.source, &arch)?, callback)
            .await
    }

    async fn update_db<F, Fut>(
        &mut self,
        mut sourcelist: Vec<OmaSourceEntry<'_>>,
        callback: F,
    ) -> Result<()>
    where
        F: Fn(Event) -> Fut,
        Fut: Future<Output = ()>,
    {
        if !self.download_dir.is_dir() {
            fs::create_dir_all(&self.download_dir).await.map_err(|e| {
                RefreshError::FailedToOperateDirOrFile(self.download_dir.display().to_string(), e)
            })?;
        }

        debug!("Setting {} permission as 0755", self.download_dir.display());

        fs::set_permissions(&self.download_dir, Permissions::from_mode(0o755))
            .await
            .map_err(|e| {
                RefreshError::FailedToOperateDirOrFile(self.download_dir.display().to_string(), e)
            })?;

        let download_dir: Box<Path> = Box::from(self.download_dir.as_path());

        spawn_blocking(move || get_apt_update_lock(&download_dir))
            .await
            .unwrap()?;

        detect_duplicate_repositories(&sourcelist)?;

        self.set_auth(&mut sourcelist);

        let mut download_list = vec![];

        let replacer = DatabaseFilenameReplacer::new()?;
        let source_map = self
            .download_releases(&sourcelist, &replacer, &callback)
            .await?;

        download_list.extend(source_map.keys().map(|x| x.as_str()));

        let (tasks, total) = self
            .collect_all_release_entry(&sourcelist, &replacer, &source_map)
            .await?;

        for i in &tasks {
            download_list.push(i.filename.as_str());
        }

        let download_dir = self.download_dir.clone();

        let (_, res) = tokio::join!(
            remove_unused_db(&download_dir, download_list),
            self.download_release_data(&callback, &tasks, total)
        );

        // 有元数据更新才执行 success invoke
        let should_run_invoke = res?.iter().any(|x| x.wrote);

        if should_run_invoke {
            callback(Event::RunInvokeScript).await;
            self.run_success_post_invoke().await;
        }

        callback(Event::Done).await;

        Ok(())
    }

    async fn download_release_data<F, Fut>(
        &self,
        callback: &F,
        tasks: &[DownloadEntry],
        total: u64,
    ) -> Result<Vec<Summary>>
    where
        F: Fn(Event) -> Fut,
        Fut: futures::Future,
    {
        let dm = DownloadManager::builder()
            .client(self.client)
            .download_list(tasks)
            .threads(self.threads)
            .set_permission(0o644)
            .total_size(total)
            .build();

        let res = dm
            .start_download(|event| async {
                callback(Event::DownloadEvent(event)).await;
            })
            .await;

        let res = res.into_iter().collect::<DownloadResult<Vec<_>>>()?;

        Ok(res)
    }

    fn set_auth(&self, sourcelist: &mut [OmaSourceEntry<'_>]) {
        for i in sourcelist {
            let auth = self.auth_config.find(i.url());
            if let Some(auth) = auth {
                i.set_auth(auth.to_owned());
            }
        }
    }

    async fn run_success_post_invoke(&self) {
        let cmds = self
            .apt_config
            .find_vector("APT::Update::Post-Invoke-Success");

        for cmd in &cmds {
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

    async fn download_releases<'b, F, Fut>(
        &mut self,
        sourcelist: &'b [OmaSourceEntry<'b>],
        replacer: &DatabaseFilenameReplacer,
        callback: &F,
    ) -> Result<SourceMap<'b>>
    where
        F: Fn(Event) -> Fut,
        Fut: Future<Output = ()>,
    {
        let mut source_map = SourceMap::new();

        #[cfg(feature = "aosc")]
        let mut not_found = vec![];

        let mut mirror_ose_map: AHashMap<String, Vec<(&OmaSourceEntry, usize)>> = AHashMap::new();

        for (i, c) in sourcelist.iter().enumerate() {
            let name = replacer.replace(c.dist_path())?;
            mirror_ose_map.entry(name).or_default().push((c, i));
        }

        let tasks = mirror_ose_map.iter().enumerate().map(|(index, (k, v))| {
            self.get_release_file(v[0], replacer, index, mirror_ose_map.len(), k, callback)
        });

        let results = futures::stream::iter(tasks)
            .buffer_unordered(self.threads)
            .collect::<Vec<_>>()
            .await;

        debug!("download_releases: results: {:?}", results);

        for result in results {
            match result {
                Ok((Some(file_name), key_or_index)) => {
                    let KeyOrIndex::Key(key) = key_or_index else {
                        unreachable!()
                    };

                    source_map.insert(
                        file_name,
                        mirror_ose_map
                            .get(key)
                            .unwrap()
                            .iter()
                            .map(|x| x.0)
                            .collect::<Vec<_>>(),
                    );
                }
                Ok((None, key_or_index)) => {
                    let KeyOrIndex::Index(index) = key_or_index else {
                        unreachable!()
                    };

                    self.flat_repo_no_release.push(index)
                }
                Err(e) => {
                    #[cfg(feature = "aosc")]
                    match e {
                        RefreshError::ReqwestError(e)
                            if e.status()
                                .map(|x| x == StatusCode::NOT_FOUND)
                                .unwrap_or(false)
                                && self.refresh_topics =>
                        {
                            let url = e.url().map(|x| x.to_owned());
                            not_found.push(url.unwrap());
                        }
                        _ => return Err(e),
                    }
                    #[cfg(not(feature = "aosc"))]
                    return Err(e.into());
                }
            }
        }

        #[cfg(not(feature = "aosc"))]
        let _ = self.topic_msg;

        #[cfg(feature = "aosc")]
        {
            if self.refresh_topics {
                callback(Event::ScanningTopic).await;
                let mut tm =
                    TopicManager::new(self.client, &self.source, &self.arch, false).await?;
                let removed_suites = tm.remove_closed_topics()?;

                for url in &not_found {
                    let suite = url
                        .path_segments()
                        .and_then(|mut x| x.nth_back(1).map(|x| x.to_string()))
                        .ok_or_else(|| RefreshError::InvalidUrl(url.to_string()))?;

                    if !removed_suites.contains(&suite)
                        && !tm.enabled_topics().iter().any(|x| x.name == suite)
                    {
                        return Err(RefreshError::NoInReleaseFile(url.to_string()));
                    }

                    callback(Event::ClosingTopic(suite)).await;
                }

                if !not_found.is_empty() {
                    tm.write_enabled().await?;
                    tm.write_sources_list(self.topic_msg, false, |topic, mirror| async move {
                        callback(Event::TopicNotInMirror {
                            topic: topic.to_string(),
                            mirror: mirror.to_string(),
                        })
                        .await
                    })
                    .await?;
                }

                callback(Event::DownloadEvent(oma_fetch::Event::ProgressDone(1))).await;
            }
        }

        Ok(source_map)
    }

    async fn get_release_file<'b, F, Fut>(
        &self,
        entry: (&OmaSourceEntry<'_>, usize),
        replacer: &DatabaseFilenameReplacer,
        progress_index: usize,
        total: usize,
        key: &'b str,
        callback: &F,
    ) -> Result<(Option<String>, KeyOrIndex<'b>)>
    where
        F: Fn(Event) -> Fut,
        Fut: Future<Output = ()>,
    {
        let (entry, index) = entry;
        match entry.from()? {
            OmaSourceEntryFrom::Http => {
                let dist_path = entry.dist_path();

                let mut r = None;
                let mut u = None;
                let mut is_release = false;

                let msg = entry.get_human_download_url(None)?;

                callback(Event::DownloadEvent(oma_fetch::Event::NewProgressSpinner {
                    index: progress_index,
                    msg: format!("({}/{}) {}", progress_index, total, msg),
                }))
                .await;

                for (index, file_name) in ["InRelease", "Release"].iter().enumerate() {
                    let url = format!("{}/{}", dist_path, file_name);
                    let request = self.request_get_builder(&url, entry);

                    let resp = request
                        .send()
                        .await
                        .and_then(|resp| resp.error_for_status());

                    r = Some(resp);

                    if r.as_ref().unwrap().is_ok() {
                        u = Some(url);
                        if index == 1 {
                            is_release = true;
                        }
                        break;
                    }
                }

                let r = r.unwrap();

                callback(Event::DownloadEvent(oma_fetch::Event::ProgressDone(
                    progress_index,
                )))
                .await;

                if r.is_err() && entry.is_flat() {
                    return Ok((None, KeyOrIndex::Index(index)));
                }

                let resp = r?;

                let url = u.unwrap();
                let file_name = replacer.replace(&url)?;

                self.download_file(&file_name, resp, entry, progress_index, total, &callback)
                    .await?;

                if is_release && !entry.trusted() {
                    let url = format!("{}/{}", dist_path, "Release.gpg");

                    let request = self.request_get_builder(&url, entry);
                    let resp = request
                        .send()
                        .await
                        .and_then(|resp| resp.error_for_status())?;

                    let file_name = replacer.replace(&url)?;

                    self.download_file(&file_name, resp, entry, progress_index, total, &callback)
                        .await?;
                }

                Ok((Some(file_name), KeyOrIndex::Key(key)))
            }
            OmaSourceEntryFrom::Local => {
                let dist_path_with_protocol = entry.dist_path();
                let dist_path = dist_path_with_protocol
                    .strip_prefix("file:")
                    .unwrap_or(dist_path_with_protocol);
                let dist_path = Path::new(dist_path);

                let mut name = None;

                let msg = entry.get_human_download_url(None)?;

                callback(Event::DownloadEvent(oma_fetch::Event::NewProgressSpinner {
                    index: progress_index,
                    msg: format!("({}/{}) {}", progress_index, total, msg),
                }))
                .await;

                let mut is_release = false;

                for (index, entry) in ["InRelease", "Release"].iter().enumerate() {
                    let p = dist_path.join(entry);

                    let dst = if dist_path_with_protocol.ends_with('/') {
                        format!("{}{}", dist_path_with_protocol, entry)
                    } else {
                        format!("{}/{}", dist_path_with_protocol, entry)
                    };

                    let file_name = replacer.replace(&dst)?;

                    let dst = self.download_dir.join(&file_name);

                    if p.exists() {
                        if dst.exists() {
                            debug!("get_release_file: Removing {}", dst.display());
                            fs::remove_file(&dst).await.map_err(|e| {
                                RefreshError::FetcherError(DownloadError::IOError(
                                    entry.to_string(),
                                    e,
                                ))
                            })?;
                        }

                        debug!("get_release_file: Symlink {}", dst.display());
                        fs::symlink(p, dst).await.map_err(|e| {
                            RefreshError::FetcherError(DownloadError::IOError(entry.to_string(), e))
                        })?;

                        if index == 1 {
                            is_release = true;
                        }

                        name = Some(file_name);
                        break;
                    }
                }

                if is_release {
                    let p = dist_path.join("Release.gpg");
                    let entry = "Release.gpg";

                    let dst = if dist_path_with_protocol.ends_with('/') {
                        format!("{}{}", dist_path_with_protocol, entry)
                    } else {
                        format!("{}/{}", dist_path_with_protocol, entry)
                    };

                    let file_name = replacer.replace(&dst)?;

                    let dst = self.download_dir.join(&file_name);

                    if p.exists() {
                        if dst.exists() {
                            fs::remove_file(&dst).await.map_err(|e| {
                                RefreshError::FetcherError(DownloadError::IOError(
                                    entry.to_string(),
                                    e,
                                ))
                            })?;
                        }

                        fs::symlink(p, self.download_dir.join(file_name))
                            .await
                            .map_err(|e| {
                                RefreshError::FetcherError(DownloadError::IOError(
                                    entry.to_string(),
                                    e,
                                ))
                            })?;
                    }
                }

                callback(Event::DownloadEvent(oma_fetch::Event::ProgressDone(
                    progress_index,
                )))
                .await;

                let name =
                    name.ok_or_else(|| RefreshError::NoInReleaseFile(entry.url().to_string()))?;

                Ok((Some(name), KeyOrIndex::Key(key)))
            }
        }
    }

    fn request_get_builder(
        &self,
        url: &str,
        source_index: &OmaSourceEntry<'_>,
    ) -> reqwest::RequestBuilder {
        let mut request = self.client.get(url);
        if let Some(auth) = &source_index.auth {
            request = request.basic_auth(&auth.user, Some(&auth.password))
        }

        request
    }

    async fn download_file<F, Fut>(
        &self,
        file_name: &str,
        mut resp: Response,
        source_index: &OmaSourceEntry<'_>,
        index: usize,
        total: usize,
        callback: &F,
    ) -> Result<()>
    where
        F: Fn(Event) -> Fut,
        Fut: Future<Output = ()>,
    {
        let total_size = content_length(&resp);

        callback(Event::DownloadEvent(oma_fetch::Event::NewProgressBar {
            index,
            msg: format!(
                "({}/{}) {}",
                index,
                total,
                source_index.get_human_download_url(Some(file_name))?
            ),
            size: total_size,
        }));

        let mut f = File::create(self.download_dir.join(file_name))
            .await
            .map_err(|e| {
                RefreshError::FetcherError(DownloadError::IOError(file_name.to_string(), e))
            })?;

        f.set_permissions(Permissions::from_mode(0o644))
            .await
            .map_err(|e| {
                RefreshError::FailedToOperateDirOrFile(self.download_dir.display().to_string(), e)
            })?;

        while let Some(chunk) = resp.chunk().await? {
            callback(Event::DownloadEvent(oma_fetch::Event::ProgressInc {
                index,
                size: chunk.len() as u64,
            }))
            .await;

            f.write_all(&chunk).await.map_err(|e| {
                RefreshError::FetcherError(DownloadError::IOError(file_name.to_string(), e))
            })?;
        }

        f.shutdown().await.map_err(|e| {
            RefreshError::FetcherError(DownloadError::IOError(file_name.to_string(), e))
        })?;

        callback(Event::DownloadEvent(oma_fetch::Event::ProgressDone(index))).await;

        Ok(())
    }

    async fn collect_all_release_entry(
        &self,
        sourcelist: &[OmaSourceEntry<'a>],
        replacer: &DatabaseFilenameReplacer,
        sources_map: &AHashMap<String, Vec<&'a OmaSourceEntry<'a>>>,
    ) -> Result<(Vec<DownloadEntry>, u64)> {
        let mut total = 0;
        let mut tasks = vec![];

        let index_target_config = IndexTargetConfig::new(self.apt_config, &self.arch);

        let archs_from_file = fs::read_to_string("/var/lib/dpkg/arch")
            .await
            .map(|f| f.lines().map(|x| x.to_string()).collect::<Vec<_>>());

        for (file_name, ose_list) in sources_map {
            let inrelease_path = self.download_dir.join(file_name);

            let mut handle = HashSet::with_hasher(ahash::RandomState::new());

            let inrelease = fs::read_to_string(&inrelease_path).await.map_err(|e| {
                RefreshError::FailedToOperateDirOrFile(inrelease_path.display().to_string(), e)
            })?;

            let inrelease = verify_inrelease(
                &inrelease,
                ose_list.iter().find_map(|x| {
                    if let Some(x) = x.signed_by() {
                        Some(x)
                    } else {
                        None
                    }
                }),
                &self.source,
                &inrelease_path,
                ose_list.iter().any(|x| x.trusted()),
            )
            .map_err(|e| RefreshError::InReleaseParseError(inrelease_path.to_path_buf(), e))?;

            let inrelease = InRelease::new(&inrelease)
                .map_err(|e| RefreshError::InReleaseParseError(inrelease_path.to_path_buf(), e))?;

            if ose_list[0].is_flat() {
                let now = Utc::now();

                inrelease.check_date(&now).map_err(|e| {
                    RefreshError::InReleaseParseError(inrelease_path.to_path_buf(), e)
                })?;

                inrelease.check_valid_until(&now).map_err(|e| {
                    RefreshError::InReleaseParseError(inrelease_path.to_path_buf(), e)
                })?;
            }

            let checksums = &inrelease
                .get_or_try_init_checksum_type_and_list()
                .map_err(|e| RefreshError::InReleaseParseError(inrelease_path.to_path_buf(), e))?
                .1;

            for ose in ose_list {
                debug!("Getted oma source entry: {:#?}", ose);

                let mut archs = if let Some(archs) = ose.archs() {
                    archs.iter().map(|x| x.as_str()).collect::<Vec<_>>()
                } else if let Ok(ref f) = archs_from_file {
                    f.iter().map(|x| x.as_str()).collect::<Vec<_>>()
                } else {
                    vec![self.arch.as_str()]
                };

                debug!("archs: {:?}", archs);

                let download_list = index_target_config.get_download_list(
                    checksums,
                    ose.is_source(),
                    ose.is_flat(),
                    &mut archs,
                    ose.components(),
                )?;

                get_all_need_db_from_config(download_list, &mut total, checksums, &mut handle);

                for i in &self.flat_repo_no_release {
                    download_flat_repo_no_release(
                        sourcelist.get(*i).unwrap(),
                        &self.download_dir,
                        &mut tasks,
                        replacer,
                    )?;
                }
            }

            for c in &handle {
                collect_download_task(
                    c,
                    ose_list[0],
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

fn content_length(resp: &Response) -> u64 {
    let content_length = resp
        .headers()
        .get(CONTENT_LENGTH)
        .map(Cow::Borrowed)
        .unwrap_or(Cow::Owned(HeaderValue::from(0)));

    let total_size = content_length
        .to_str()
        .ok()
        .and_then(|x| x.parse::<u64>().ok())
        .unwrap_or_default();

    total_size
}

fn detect_duplicate_repositories(sourcelist: &[OmaSourceEntry<'_>]) -> Result<()> {
    let mut map = AHashMap::new();

    for i in sourcelist {
        if !map.contains_key(&(i.url(), i.suite())) {
            map.insert((i.url(), i.suite()), vec![i]);
        } else {
            map.get_mut(&(i.url(), i.suite())).unwrap().push(i);
        }
    }

    // 查看源配置中是否有重复的源
    // 重复的源的定义：源地址相同 源类型相同 源 component 有重复项
    // 比如：
    // deb https://mirrors.bfsu.edu.cn/anthon/debs stable main
    // deb https://mirrors.bfsu.edu.cn/anthon/debs stable main contrib
    // 重复的项为：deb https://mirrors.bfsu.edu.cn/anthon/debs stable main
    for ose_list in map.values() {
        let mut no_dups_components = HashSet::with_hasher(ahash::RandomState::new());

        for ose in ose_list {
            for c in ose.components() {
                if !no_dups_components.contains(&(c, ose.is_source())) {
                    no_dups_components.insert((c, ose.is_source()));
                } else {
                    return Err(RefreshError::DuplicateComponents(
                        ose.url().into(),
                        c.to_string(),
                    ));
                }
            }
        }
    }

    Ok(())
}

fn get_all_need_db_from_config(
    filter_checksums: Vec<ChecksumDownloadEntry>,
    total: &mut u64,
    checksums: &[ChecksumItem],
    handle: &mut HashSet<ChecksumDownloadEntry>,
) {
    for i in filter_checksums {
        if handle.contains(&i) {
            continue;
        }

        if i.keep_compress {
            *total += i.item.size;
        } else {
            let size = if file_is_compress(&i.item.name) {
                let (_, name_without_compress) = split_ext_and_filename(&i.item.name);

                checksums
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

        handle.insert(i);
    }
}

async fn remove_unused_db(download_dir: &Path, download_list: Vec<&str>) -> Result<()> {
    let mut download_dir = fs::read_dir(&download_dir)
        .await
        .map_err(|e| RefreshError::ReadDownloadDir(download_dir.display().to_string(), e))?;

    while let Ok(Some(x)) = download_dir.next_entry().await {
        if x.path().is_file()
            && !download_list.contains(&&*x.file_name().to_string_lossy())
            && x.file_name() != "lock"
        {
            debug!("Removing {:?}", x.file_name());
            if let Err(e) = fs::remove_file(x.path()).await {
                debug!("Failed to remove file {:?}: {e}", x.file_name());
            }
        }
    }

    Ok(())
}

fn download_flat_repo_no_release(
    source_index: &OmaSourceEntry,
    download_dir: &Path,
    tasks: &mut Vec<DownloadEntry>,
    replacer: &DatabaseFilenameReplacer,
) -> Result<()> {
    let msg = source_index.get_human_download_url(Some("Packages"))?;

    let dist_url = source_index.dist_path();

    let from = match source_index.from()? {
        OmaSourceEntryFrom::Http => DownloadSourceType::Http {
            auth: source_index
                .auth
                .as_ref()
                .map(|auth| (auth.user.clone(), auth.password.clone())),
        },
        OmaSourceEntryFrom::Local => DownloadSourceType::Local(source_index.is_flat()),
    };

    let download_url = format!("{}/Packages", dist_url);
    let file_path = format!("{}Packages", dist_url);

    let sources = vec![DownloadSource {
        url: download_url.clone(),
        source_type: from,
    }];

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
    inrelease: &InRelease,
    replacer: &DatabaseFilenameReplacer,
) -> Result<()> {
    let file_type = &c.msg;

    let msg = source_index.get_human_download_url(Some(file_type))?;

    let dist_url = &source_index.dist_path();

    let from = match source_index.from()? {
        OmaSourceEntryFrom::Http => DownloadSourceType::Http {
            auth: source_index
                .auth
                .as_ref()
                .map(|auth| (auth.user.clone(), auth.password.clone())),
        },
        OmaSourceEntryFrom::Local => DownloadSourceType::Local(source_index.is_flat()),
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
            .checksum_type_and_list()
            .1
            .iter()
            .find(|x| x.name == *not_compress_filename_before)
            .as_ref()
            .map(|c| &c.checksum)
    };

    let download_url = if inrelease.acquire_by_hash() {
        let path = Path::new(&c.item.name);
        let parent = path.parent().unwrap_or(path);
        let dir = match inrelease.checksum_type_and_list().0 {
            InReleaseChecksum::Sha256 => "SHA256",
            InReleaseChecksum::Sha512 => "SHA512",
            InReleaseChecksum::Md5 => "MD5Sum",
        };

        let path = parent.join("by-hash").join(dir).join(&c.item.checksum);

        format!("{}/{}", dist_url, path.display())
    } else if dist_url.ends_with('/') {
        format!("{}{}", dist_url, c.item.name)
    } else {
        format!("{}/{}", dist_url, c.item.name)
    };

    let sources = vec![DownloadSource {
        url: download_url.clone(),
        source_type: from,
    }];

    let file_path = if c.keep_compress {
        if inrelease.acquire_by_hash() {
            Cow::Owned(format!("{}/{}", dist_url, c.item.name))
        } else {
            Cow::Borrowed(&download_url)
        }
    } else if dist_url.ends_with('/') {
        Cow::Owned(format!("{}{}", dist_url, not_compress_filename_before))
    } else {
        Cow::Owned(format!("{}/{}", dist_url, not_compress_filename_before))
    };

    let file_name = replacer.replace(&file_path)?;

    let task = DownloadEntry::builder()
        .source(sources)
        .filename(file_name)
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
            match inrelease.checksum_type_and_list().0 {
                InReleaseChecksum::Sha256 => Some(Checksum::from_sha256_str(checksum)?),
                InReleaseChecksum::Sha512 => Some(Checksum::from_sha512_str(checksum)?),
                InReleaseChecksum::Md5 => Some(Checksum::from_md5_str(checksum)?),
            }
        } else {
            None
        })
        .build();

    debug!("oma will download source database: {download_url}");
    tasks.push(task);

    Ok(())
}
