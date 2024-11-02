use std::{
    borrow::Cow,
    collections::hash_map::Entry,
    future::Future,
    os::fd::AsRawFd,
    path::{Path, PathBuf},
};

use ahash::{AHashMap, HashSet};
use aho_corasick::BuildError;
use apt_auth_config::AuthConfig;
use bon::{builder, Builder};
use chrono::Utc;
use futures::{
    future::{join_all, BoxFuture},
    FutureExt, StreamExt,
};
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
    reqwest::{self, Client},
    CompressFile, DownloadEntry, DownloadManager, DownloadProgressControl, DownloadResult,
    DownloadSource, DownloadSourceType, Summary,
};

#[cfg(feature = "aosc")]
use oma_fetch::DownloadError;

#[cfg(feature = "aosc")]
use oma_topics::TopicManager;

use oma_utils::dpkg::dpkg_arch;
#[cfg(feature = "aosc")]
use reqwest::StatusCode;

use smallvec::SmallVec;
use sysinfo::{Pid, System};
use tokio::{fs, process::Command, task::spawn_blocking};
use tracing::{debug, warn};

use crate::{
    config::{fiilter_download_list, get_config, ChecksumDownloadEntry, FilterDownloadList},
    inrelease::{
        file_is_compress, split_ext_and_filename, verify_inrelease, ChecksumItem, InRelease,
        InReleaseChecksum, InReleaseError,
    },
    sourceslist::{sources_lists, OmaSourceEntry, OmaSourceEntryFrom},
    util::DatabaseFilenameReplacer,
};

pub trait HandleRefresh: DownloadProgressControl + HandleTopicsControl {
    fn run_invoke_script(&self);
}

#[cfg(feature = "aosc")]
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
    InReleaseParseError(String, InReleaseError),
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

#[cfg(not(feature = "aosc"))]
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
    InReleaseParseError(String, InReleaseError),
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
    progress_manager: &'a dyn HandleRefresh,
    auth_config: &'a AuthConfig,
}

enum RepoType {
    InRelease,
    Release,
    FlatNoRelease,
}

type SourceMap<'a> = AHashMap<String, Vec<OmaSourceEntry<'a>>>;
type ResponseResult = std::result::Result<reqwest::Response, reqwest::Error>;

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

pub trait HandleTopicsControl {
    fn scanning_topic(&self);
    fn closing_topic(&self, topic: &str);
    fn topic_not_in_mirror(&self, topic: &str, mirror: &str);
}

impl<'a> OmaRefresh<'a> {
    pub async fn start(mut self) -> Result<()> {
        let arch = dpkg_arch(&self.source)?;
        self.update_db(
            sources_lists(&self.source, &arch)?,
            self.progress_manager,
            self.topic_msg,
        )
        .await
    }

    async fn update_db(
        &mut self,
        sourcelist: Vec<OmaSourceEntry<'_>>,
        progress_manager: &dyn HandleRefresh,
        topic_msg: &str,
    ) -> Result<()> {
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

        let download_dir: Box<Path> = Box::from(self.download_dir.as_path());

        spawn_blocking(move || get_apt_update_lock(&download_dir))
            .await
            .unwrap()?;

        detect_duplicate_repositories(&sourcelist)?;

        let is_inrelease_map = self
            .get_is_inrelease_map(&sourcelist, progress_manager)
            .await?;

        let mut download_list = vec![];

        let replacer = DatabaseFilenameReplacer::new()?;
        let (tasks, soueces_map) =
            self.collect_download_release_tasks(&sourcelist, is_inrelease_map, &replacer)?;

        for i in &tasks {
            download_list.push(i.filename.to_string());
        }

        let release_results = DownloadManager::builder()
            .client(self.client)
            .threads(self.threads)
            .download_list(tasks)
            .progress_manager(progress_manager.as_download_progress_control())
            .build()
            .start_download()
            .await;

        let all_inrelease = self
            .handle_downloaded_release_result(release_results, progress_manager, topic_msg)
            .await?;

        let config_tree = get_config(self.apt_config);

        let (tasks, total) = self
            .collect_all_release_entry(
                all_inrelease,
                &sourcelist,
                &replacer,
                &soueces_map,
                &config_tree,
            )
            .await?;

        for i in &tasks {
            download_list.push(i.filename.to_string());
        }

        let download_dir = self.download_dir.clone();
        let remove_task =
            tokio::spawn(async move { remove_unused_db(download_dir, download_list).await });

        let res = DownloadManager::builder()
            .client(self.client)
            .download_list(tasks)
            .threads(self.threads)
            .progress_manager(progress_manager.as_download_progress_control())
            .total_size(total)
            .build()
            .start_download()
            .await;

        res.into_iter().collect::<DownloadResult<Vec<_>>>()?;

        // Finally, run success post invoke
        let _ = remove_task.await;
        progress_manager.run_invoke_script();
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

    async fn get_is_inrelease_map(
        &mut self,
        sourcelist: &[OmaSourceEntry<'_>],
        progress_manager: &dyn HandleRefresh,
    ) -> Result<AHashMap<usize, RepoType>> {
        let mut tasks = vec![];

        let mut mirrors_inrelease = AHashMap::new();

        for (i, c) in sourcelist.iter().enumerate() {
            let mut tasks1 = vec![];
            let dist_path = c.dist_path();

            let auth = self.auth_config.find(c.url());

            match c.from()? {
                OmaSourceEntryFrom::Http => {
                    let resp1: BoxFuture<'_, _> = Box::pin(self.response(dist_path, auth, i));

                    let resp2 = Box::pin({
                        let mut client = self.client.head(format!("{}/Release", dist_path));

                        if let Some(auth) = auth {
                            client = client.basic_auth(&auth.user, Some(&auth.password))
                        }

                        client
                            .send()
                            .map(move |x| (x.and_then(|x| x.error_for_status()), i))
                    });

                    tasks1.push(resp1);
                    tasks1.push(resp2);

                    if c.is_flat() {
                        let resp3 = Box::pin(
                            self.client
                                .head(format!("{}/Packages", dist_path))
                                .send()
                                .map(move |x| (x.and_then(|x| x.error_for_status()), i)),
                        );
                        tasks1.push(resp3);
                    }

                    let s = c.get_human_download_url(None)?;

                    let task = async move {
                        progress_manager.new_progress_spinner(
                            i,
                            &format!("({}/{}) {}", i + 1, sourcelist.len(), s),
                        );
                        let res = join_all(tasks1).await;
                        progress_manager.progress_done(i);
                        res
                    };

                    tasks.push(task);
                }
                OmaSourceEntryFrom::Local => {
                    let msg = format!(
                        "({}/{}) {}",
                        i + 1,
                        sourcelist.len(),
                        c.get_human_download_url(None)?
                    );

                    progress_manager.new_progress_spinner(i, &msg);

                    let dist_path = dist_path.strip_prefix("file:").unwrap_or(dist_path);

                    let p1 = Path::new(dist_path).join("InRelease");
                    let p2 = Path::new(dist_path).join("Release");

                    if p1.exists() {
                        mirrors_inrelease.insert(i, RepoType::InRelease);
                    } else if p2.exists() {
                        mirrors_inrelease.insert(i, RepoType::Release);
                    } else if c.is_flat() {
                        // flat repo 可以没有 Release 文件
                        self.flat_repo_no_release.push(i);
                        mirrors_inrelease.insert(i, RepoType::FlatNoRelease);
                        continue;
                    } else {
                        progress_manager.progress_done(i);
                        #[cfg(feature = "aosc")]
                        // FIXME: 为了能让 oma refresh 正确关闭 topic，这里先忽略错误
                        mirrors_inrelease.insert(i, RepoType::InRelease);
                        #[cfg(not(feature = "aosc"))]
                        return Err(RefreshError::NoInReleaseFile(c.dist_path().to_string()));
                    }

                    progress_manager.progress_done(i);
                }
            }
        }

        let stream = futures::stream::iter(tasks).buffer_unordered(self.threads);
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
                sourcelist[i.first().unwrap().1].dist_path().to_string(),
            ));
        }

        Ok(mirrors_inrelease)
    }

    fn response(
        &mut self,
        dist_path: &str,
        auth: Option<&apt_auth_config::AuthConfigEntry>,
        i: usize,
    ) -> futures::future::Map<
        impl Future<Output = ResponseResult>,
        impl FnOnce(ResponseResult) -> (ResponseResult, usize),
    > {
        let mut client = self.client.head(format!("{}/InRelease", dist_path));
        if let Some(auth) = auth {
            client = client.basic_auth(&auth.user, Some(&auth.password))
        }
        client
            .send()
            .map(move |x| (x.and_then(|x| x.error_for_status()), i))
    }

    fn collect_download_release_tasks(
        &'a self,
        sourcelist: &[OmaSourceEntry<'a>],
        is_inrelease_map: AHashMap<usize, RepoType>,
        replacer: &DatabaseFilenameReplacer,
    ) -> Result<(Vec<DownloadEntry>, SourceMap)> {
        let mut tasks = Vec::new();
        let mut map: AHashMap<String, Vec<OmaSourceEntry<'a>>> = AHashMap::new();

        for (i, source_entry) in sourcelist.iter().enumerate() {
            let repo_type = is_inrelease_map.get(&i).unwrap();

            let repo_type_str = match repo_type {
                RepoType::InRelease => "InRelease",
                RepoType::Release => "Release",
                RepoType::FlatNoRelease => continue,
            };

            let uri = format!(
                "{}{}{}",
                source_entry.dist_path(),
                if !source_entry.dist_path().ends_with('/') {
                    "/"
                } else {
                    ""
                },
                repo_type_str
            );

            let msg = source_entry.get_human_download_url(Some(repo_type_str))?;

            let sources = vec![DownloadSource {
                url: uri.clone(),
                source_type: match source_entry.from()? {
                    OmaSourceEntryFrom::Http => DownloadSourceType::Http,
                    // 为保持与 apt 行为一致，本地源 symlink Release 文件
                    OmaSourceEntryFrom::Local => DownloadSourceType::Local(source_entry.is_flat()),
                },
            }];

            let task = DownloadEntry::builder()
                .source(sources)
                .filename(replacer.replace(&uri)?)
                .dir(self.download_dir.clone())
                .allow_resume(false)
                .msg(msg)
                .build();

            debug!("oma will fetch {} InRelease", source_entry.url());

            match map.entry(task.filename.to_string()) {
                Entry::Occupied(mut occupied_entry) => {
                    occupied_entry.get_mut().push(source_entry.to_owned());
                }
                Entry::Vacant(vacant_entry) => {
                    vacant_entry.insert(vec![source_entry.to_owned()]);
                    tasks.push(task);
                }
            }
        }

        Ok((tasks, map))
    }

    async fn handle_downloaded_release_result(
        &self,
        res: Vec<DownloadResult<Summary>>,
        _progress_manager: &dyn HandleRefresh,
        _handle_topic_msg: &str,
    ) -> Result<Vec<Summary>> {
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
                _progress_manager.scanning_topic();
                let mut tm =
                    TopicManager::new(self.client, &self.source, &self.arch, false).await?;
                let removed_suites =
                    oma_topics::scan_closed_topic(&mut tm, _handle_topic_msg, |topic, mirror| {
                        _progress_manager.topic_not_in_mirror(topic, mirror)
                    })
                    .await?;

                for url in not_found {
                    let suite = url
                        .path_segments()
                        .and_then(|mut x| x.nth_back(1).map(|x| x.to_string()))
                        .ok_or_else(|| RefreshError::InvalidUrl(url.to_string()))?;

                    if !removed_suites.contains(&suite) {
                        return Err(RefreshError::NoInReleaseFile(url.to_string()));
                    }

                    _progress_manager.closing_topic(&suite);
                }
            }
        }

        Ok(all_inrelease)
    }

    async fn collect_all_release_entry(
        &self,
        all_inrelease: Vec<Summary>,
        sourcelist: &[OmaSourceEntry<'a>],
        replacer: &DatabaseFilenameReplacer,
        sources_map: &AHashMap<String, Vec<OmaSourceEntry<'a>>>,
        config_tree: &[(String, String)],
    ) -> Result<(Vec<DownloadEntry>, u64)> {
        let mut total = 0;
        let mut tasks = vec![];
        for inrelease_summary in all_inrelease {
            // 源数据确保是存在的，所以直接 unwrap
            let ose_list = sources_map.get(&inrelease_summary.filename).unwrap();

            for ose in ose_list {
                debug!("Getted oma source entry: {:#?}", ose);
                let inrelease_path = self.download_dir.join(&*inrelease_summary.filename);

                let inrelease = tokio::fs::read_to_string(&inrelease_path)
                    .await
                    .map_err(|e| {
                        RefreshError::FailedToOperateDirOrFile(
                            inrelease_path.display().to_string(),
                            e,
                        )
                    })?;

                let archs = if let Some(archs) = ose.options().get("archs") {
                    archs.split(',').collect::<Vec<_>>()
                } else {
                    vec![self.arch.as_str()]
                };

                let inrelease = verify_inrelease(
                    &inrelease,
                    ose.options().get("signed-by").map(|x| x.as_str()),
                    &self.source,
                    ose.options().get("trusted").is_some_and(|x| x == "yes"),
                )
                .map_err(|e| {
                    RefreshError::InReleaseParseError(inrelease_path.display().to_string(), e)
                })?;

                let inrelease = InRelease::new(&inrelease).map_err(|e| {
                    RefreshError::InReleaseParseError(inrelease_path.display().to_string(), e)
                })?;

                if !ose.is_flat() {
                    let now = Utc::now();

                    inrelease.check_date(&now).map_err(|e| {
                        RefreshError::InReleaseParseError(inrelease_path.display().to_string(), e)
                    })?;

                    inrelease.check_valid_until(&now).map_err(|e| {
                        RefreshError::InReleaseParseError(inrelease_path.display().to_string(), e)
                    })?;
                }

                let checksums = &inrelease
                    .get_or_try_init_checksum_type_and_list()
                    .map_err(|e| {
                        RefreshError::InReleaseParseError(inrelease_path.display().to_string(), e)
                    })?
                    .1;

                let mut handle = vec![];
                let f = FilterDownloadList {
                    checksums,
                    config: self.apt_config,
                    config_tree,
                    archs: &archs,
                    components: ose.components(),
                    native_arch: &self.arch,
                    is_flat: ose.is_flat(),
                    is_source: ose.is_source(),
                };

                let filter_checksums = fiilter_download_list(f);

                get_all_need_db_from_config(filter_checksums, &mut total, checksums, &mut handle);

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
        }

        Ok((tasks, total))
    }
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
    filter_checksums: SmallVec<[ChecksumDownloadEntry; 32]>,
    total: &mut u64,
    checksums: &[ChecksumItem],
    handle: &mut Vec<ChecksumDownloadEntry>,
) {
    for i in filter_checksums {
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
            && x.file_name() != "lock"
        {
            debug!("Removing {:?}", x.file_name());
            if let Err(e) = tokio::fs::remove_file(x.path()).await {
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
        OmaSourceEntryFrom::Http => DownloadSourceType::Http,
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
        OmaSourceEntryFrom::Http => DownloadSourceType::Http,
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
