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
use oma_apt_sources_lists::SourcesListError;
use oma_fetch::{
    checksum::{Checksum, ChecksumError},
    reqwest::{
        header::{HeaderValue, CONTENT_LENGTH},
        Client, Response,
    },
    CompressFile, DownloadEntry, DownloadManager, DownloadSource, DownloadSourceType,
};

use oma_fetch::{SingleDownloadError, Summary};
#[cfg(feature = "aosc")]
use oma_topics::TopicManager;

use oma_utils::dpkg::dpkg_arch;

#[cfg(feature = "aosc")]
use oma_fetch::reqwest::StatusCode;

use sysinfo::{Pid, System};
use tokio::{
    fs::{self},
    process::Command,
    task::spawn_blocking,
};
use tracing::{debug, warn};

use crate::sourceslist::{MirrorSource, MirrorSources};
use crate::{
    config::{ChecksumDownloadEntry, IndexTargetConfig},
    inrelease::{
        file_is_compress, split_ext_and_filename, verify_inrelease, ChecksumItem,
        InReleaseChecksum, InReleaseError, Release,
    },
    sourceslist::{sources_lists, OmaSourceEntry, OmaSourceEntryFrom},
    util::DatabaseFilenameReplacer,
};

#[derive(Debug, thiserror::Error)]
pub enum RefreshError {
    #[error("Invalid URL: {0}")]
    InvalidUrl(String),
    #[error("Scan sources.list failed: {0}")]
    ScanSourceError(SourcesListError),
    #[error("Unsupported Protocol: {0}")]
    UnsupportedProtocol(String),
    #[error("Failed to download some metadata")]
    DownloadFailed(Option<SingleDownloadError>),
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
    #[error("sources.list is empty")]
    SourceListsEmpty,
    #[error("Failed to operate file: {0}")]
    OperateFile(PathBuf, std::io::Error),
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
    #[cfg(feature = "aosc")]
    refresh_topics: bool,
    apt_config: &'a Config,
    topic_msg: &'a str,
    auth_config: Option<&'a AuthConfig>,
}

/// Create `apt update` file lock
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
    SourceListFileNotSupport { path: PathBuf },
    Done,
}

impl<'a> OmaRefresh<'a> {
    pub async fn start<F, Fut>(mut self, callback: F) -> Result<()>
    where
        F: Fn(Event) -> Fut,
        Fut: Future<Output = ()>,
    {
        let arch = dpkg_arch(&self.source)?;
        let sourcelist = sources_lists(&self.source, &arch, &callback)
            .await
            .map_err(RefreshError::ScanSourceError)?;

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

        let mut download_list = vec![];

        let replacer = DatabaseFilenameReplacer::new()?;
        let mirror_sources = self
            .download_releases(&sourcelist, &replacer, &callback)
            .await?;

        download_list.extend(mirror_sources.0.iter().flat_map(|x| x.file_name()));

        let (tasks, total) = self
            .collect_all_release_entry(&replacer, &mirror_sources)
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
        let should_run_invoke = res?.has_wrote();

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
    ) -> Result<Summary>
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
            .await
            .unwrap();

        if !res.is_download_success() {
            return Err(RefreshError::DownloadFailed(None));
        }

        // 有元数据更新才执行 success invoke
        let should_run_invoke = res.has_wrote();

        if should_run_invoke {
            callback(Event::RunInvokeScript).await;
            self.run_success_post_invoke().await;
        }

        callback(Event::Done).await;

        Ok(res)
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
    ) -> Result<MirrorSources<'b, 'a>>
    where
        F: Fn(Event) -> Fut,
        Fut: Future<Output = ()>,
    {
        #[cfg(feature = "aosc")]
        let mut not_found = vec![];

        #[cfg(not(feature = "aosc"))]
        let not_found = vec![];

        let mut mirror_sources =
            MirrorSources::from_sourcelist(sourcelist, replacer, self.auth_config)?;

        let results = mirror_sources
            .fetch_all_release(
                self.client,
                replacer,
                &self.download_dir,
                self.threads,
                callback,
            )
            .await;

        debug!("download_releases: results: {:?}", results);

        #[cfg(feature = "aosc")]
        for result in results {
            if let Err(e) = result {
                match e {
                    RefreshError::DownloadFailed(Some(SingleDownloadError::ReqwestError {
                        source,
                    })) if source
                        .status()
                        .map(|x| x == StatusCode::NOT_FOUND)
                        .unwrap_or(false)
                        && self.refresh_topics =>
                    {
                        let url = source.url().map(|x| x.to_owned());
                        not_found.push(url.unwrap());
                    }
                    _ => return Err(e),
                }
            }
        }

        #[cfg(not(feature = "aosc"))]
        results.into_iter().collect::<Result<Vec<_>>>()?;

        #[cfg(not(feature = "aosc"))]
        let _ = self.topic_msg;

        self.refresh_topics(callback, not_found, &mut mirror_sources)
            .await?;

        Ok(mirror_sources)
    }

    #[cfg(feature = "aosc")]
    async fn refresh_topics<'b, F, Fut>(
        &self,
        callback: &F,
        not_found: Vec<url::Url>,
        sources: &mut MirrorSources<'b, 'a>,
    ) -> Result<()>
    where
        F: Fn(Event) -> Fut,
        Fut: Future<Output = ()>,
    {
        if !self.refresh_topics || not_found.is_empty() {
            return Ok(());
        }

        callback(Event::ScanningTopic).await;
        let mut tm = TopicManager::new(self.client, &self.source, &self.arch, false).await?;
        tm.refresh().await?;
        let removed_suites = tm.remove_closed_topics()?;

        debug!("removed suites: {:?}", removed_suites);

        for url in not_found {
            let suite = url
                .path_segments()
                .and_then(|mut x| x.nth_back(1).map(|x| x.to_string()))
                .ok_or_else(|| RefreshError::InvalidUrl(url.to_string()))?;

            if !removed_suites.contains(&suite)
                && !tm.enabled_topics().iter().any(|x| x.name == suite)
            {
                return Err(RefreshError::NoInReleaseFile(url.to_string()));
            }

            let pos = sources.0.iter().position(|x| x.suite() == suite).unwrap();
            sources.0.remove(pos);

            callback(Event::ClosingTopic(suite)).await;
        }

        tm.write_enabled().await?;
        tm.write_sources_list(self.topic_msg, false, |topic, mirror| async move {
            callback(Event::TopicNotInMirror {
                topic: topic.to_string(),
                mirror: mirror.to_string(),
            })
            .await
        })
        .await?;

        callback(Event::DownloadEvent(oma_fetch::Event::ProgressDone(1))).await;

        Ok(())
    }

    #[cfg(not(feature = "aosc"))]
    async fn refresh_topics<'b, F, Fut>(
        &self,
        _callback: &F,
        _not_found: Vec<url::Url>,
        _sources: &mut MirrorSources<'b, 'a>,
    ) -> Result<()>
    where
        F: Fn(Event) -> Fut,
        Fut: Future<Output = ()>,
    {
        Ok(())
    }

    async fn collect_all_release_entry<'b>(
        &self,
        replacer: &DatabaseFilenameReplacer,
        mirror_sources: &MirrorSources<'b, 'a>,
    ) -> Result<(Vec<DownloadEntry>, u64)> {
        let mut total = 0;
        let mut tasks = vec![];

        let index_target_config = IndexTargetConfig::new(self.apt_config, &self.arch);

        let archs_from_file = fs::read_to_string("/var/lib/dpkg/arch").await;

        let archs_from_file = if let Ok(file) = archs_from_file {
            let res = file.lines().map(|x| x.to_string()).collect::<Vec<_>>();

            if res.is_empty() {
                None
            } else {
                Some(res)
            }
        } else {
            None
        };

        let mut flat_repo_no_release = vec![];

        for m in &mirror_sources.0 {
            let file_name = match m.file_name() {
                Some(name) => name,
                None => {
                    flat_repo_no_release.push(m);
                    continue;
                }
            };

            let inrelease_path = self.download_dir.join(file_name);

            let mut handle = HashSet::with_hasher(ahash::RandomState::new());

            let inrelease = fs::read_to_string(&inrelease_path).await.map_err(|e| {
                RefreshError::FailedToOperateDirOrFile(inrelease_path.display().to_string(), e)
            })?;

            let inrelease = verify_inrelease(
                &inrelease,
                m.signed_by(),
                &self.source,
                &inrelease_path,
                m.trusted(),
            )
            .map_err(|e| RefreshError::InReleaseParseError(inrelease_path.to_path_buf(), e))?;

            let release: Release = inrelease
                .parse()
                .map_err(|e| RefreshError::InReleaseParseError(inrelease_path.to_path_buf(), e))?;

            if !m.is_flat() {
                let now = Utc::now();

                release.check_date(&now).map_err(|e| {
                    RefreshError::InReleaseParseError(inrelease_path.to_path_buf(), e)
                })?;

                release.check_valid_until(&now).map_err(|e| {
                    RefreshError::InReleaseParseError(inrelease_path.to_path_buf(), e)
                })?;
            }

            let checksums = &release
                .get_or_try_init_checksum_type_and_list()
                .map_err(|e| RefreshError::InReleaseParseError(inrelease_path.to_path_buf(), e))?
                .1;

            for ose in &m.sources {
                debug!("Getted oma source entry: {:#?}", ose);

                let mut archs = if let Some(archs) = ose.archs() {
                    archs.iter().map(|x| x.as_str()).collect::<Vec<_>>()
                } else if let Some(ref f) = archs_from_file {
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
            }

            for i in &flat_repo_no_release {
                collect_flat_repo_no_release(i, &self.download_dir, &mut tasks, replacer)?;
            }

            for c in &handle {
                collect_download_task(c, m, &self.download_dir, &mut tasks, &release, replacer)?;
            }
        }

        Ok((tasks, total))
    }
}

pub fn content_length(resp: &Response) -> u64 {
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

fn collect_flat_repo_no_release(
    mirror_source: &MirrorSource,
    download_dir: &Path,
    tasks: &mut Vec<DownloadEntry>,
    replacer: &DatabaseFilenameReplacer,
) -> Result<()> {
    let msg = mirror_source.get_human_download_message(Some("Packages"))?;

    let dist_url = mirror_source.dist_path();

    let from = match mirror_source.from()? {
        OmaSourceEntryFrom::Http => DownloadSourceType::Http {
            auth: mirror_source
                .auth()
                .as_ref()
                .map(|auth| (auth.login.clone(), auth.password.clone())),
        },
        OmaSourceEntryFrom::Local => DownloadSourceType::Local(mirror_source.is_flat()),
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
    mirror_source: &MirrorSource<'_, '_>,
    download_dir: &Path,
    tasks: &mut Vec<DownloadEntry>,
    release: &Release,
    replacer: &DatabaseFilenameReplacer,
) -> Result<()> {
    let file_type = &c.msg;

    let msg = mirror_source.get_human_download_message(Some(file_type))?;

    let dist_url = &mirror_source.dist_path();

    let from = match mirror_source.from()? {
        OmaSourceEntryFrom::Http => DownloadSourceType::Http {
            auth: mirror_source
                .auth()
                .as_ref()
                .map(|auth| (auth.login.clone(), auth.password.clone())),
        },
        OmaSourceEntryFrom::Local => DownloadSourceType::Local(mirror_source.is_flat()),
    };

    let not_compress_filename_before = if file_is_compress(&c.item.name) {
        Cow::Owned(split_ext_and_filename(&c.item.name).1)
    } else {
        Cow::Borrowed(&c.item.name)
    };

    let checksum = if c.keep_compress {
        Some(&c.item.checksum)
    } else {
        release
            .checksum_type_and_list()
            .1
            .iter()
            .find(|x| x.name == *not_compress_filename_before)
            .as_ref()
            .map(|c| &c.checksum)
    };

    let download_url = if release.acquire_by_hash() {
        let path = Path::new(&c.item.name);
        let parent = path.parent().unwrap_or(path);
        let dir = match release.checksum_type_and_list().0 {
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
        if release.acquire_by_hash() {
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
            match release.checksum_type_and_list().0 {
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
