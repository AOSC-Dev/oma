use std::{
    borrow::Cow,
    fs::DirEntry,
    path::{Path, PathBuf},
    sync::Arc,
};

use ahash::{AHashMap, HashSet, HashSetExt};
use aho_corasick::BuildError;
use bon::Builder;
use jiff::Timestamp;

use flume::Sender;
use oma_apt_pkg::AptConfig;
use oma_apt_sources_lists::SourcesListError;
use oma_fetch::{
    CompressType, DownloadEntry, DownloadManager, DownloadSource, DownloadSourceType,
    checksum::{Checksum, ChecksumError},
    download::{BuilderError, SuccessSummary},
    reqwest::{
        Response,
        header::{CONTENT_LENGTH, HeaderValue},
    },
};

use oma_fetch::{SingleDownloadError, Summary};
#[cfg(feature = "aosc")]
use oma_topics::TopicManager;

#[cfg(feature = "aosc")]
use oma_fetch::reqwest::StatusCode;

use oma_utils::{GetLockError, get_file_lock, is_termux};
use reqwest_middleware::ClientWithMiddleware;
use serde::{Deserialize, Serialize};
use spdlog::{debug, warn};
use url::Url;

use oma_apt_pkg::apt_sources::{SourceLookup, scan_sources_list_paths};

use crate::sourceslist::{MirrorSource, MirrorSources};
use crate::{
    config::{ChecksumDownloadEntry, IndexTargetConfig},
    inrelease::{
        ChecksumItem, InReleaseChecksum, InReleaseError, Release, file_is_compress,
        split_ext_and_filename, verify_inrelease,
    },
    sourceslist::{OmaSourceEntry, OmaSourceEntryFrom},
    util::url_to_list_filename,
};

#[derive(Debug, thiserror::Error)]
pub enum RefreshError {
    #[error("Failed to create tokio runtime")]
    CreateTokioRuntime(std::io::Error),
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
    #[error(transparent)]
    SetLock(GetLockError),
    #[error("duplicate components")]
    DuplicateComponents(Box<str>, String),
    #[error("sources.list is empty")]
    SourceListsEmpty,
    #[error("Failed to operate file: {0}")]
    OperateFile(PathBuf, std::io::Error),
    #[error("thread count is not illegal: {0}")]
    WrongThreadCount(usize),
    #[error("Failed to build download manager")]
    DownloadManagerBuilderError(BuilderError),
    #[error("No metadata file to download")]
    NoMetadataToDownload,
}

type Result<T> = std::result::Result<T, RefreshError>;

#[derive(Builder)]
pub struct OmaRefresh {
    source: PathBuf,
    #[builder(default = 4)]
    threads: usize,
    arch: String,
    download_dir: PathBuf,
    client: ClientWithMiddleware,
    #[cfg(feature = "aosc")]
    refresh_topics: bool,
    #[cfg(feature = "aosc")]
    topic_msg: Cow<'static, str>,
    sources_lists_paths: Option<Vec<PathBuf>>,
}

#[derive(Debug, Serialize, Deserialize)]
pub enum Event {
    DownloadEvent(oma_fetch::Event),
    ScanningTopic,
    ClosingTopic(String),
    TopicNotInMirror { topic: String, mirror: String },
    RunInvokeScript,
    SourceListFileNotSupport { path: PathBuf },
    Done,
}

impl OmaRefresh {
    pub fn start(self, mut callback: impl FnMut(Event) + 'static) -> Result<Vec<SuccessSummary>> {
        if self.threads == 0 || self.threads > 255 {
            return Err(RefreshError::WrongThreadCount(self.threads));
        }

        let apt_cfg = self.init_apt_config();

        let ignores = crate::sourceslist::ignores(&apt_cfg);

        let paths: Vec<PathBuf> = if let Some(ref p) = self.sources_lists_paths {
            p.clone()
        } else {
            let list_file = if is_termux() {
                "/data/data/com.termux/files/usr/etc/apt/sources.list".to_string()
            } else {
                apt_cfg.get_file("Dir::Etc::sourcelist", "etc/apt/sources.list")
            };

            let list_dir = if is_termux() {
                "/data/data/com.termux/files/usr/etc/apt/sources.list.d".to_string()
            } else {
                apt_cfg.get_dir("Dir::Etc::sourceparts", "etc/apt/sources.list.d")
            };

            debug!("sources.list is: {list_file}");
            debug!("sources.list.d is: {list_dir}");

            scan_sources_list_paths(&list_file, &list_dir)
        };

        let source_lookup = SourceLookup::from_paths(&paths, |path| {
            let name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
            if ignores.iter().any(|re| re.is_match(name).unwrap_or(false)) {
                return;
            }
            callback(Event::SourceListFileNotSupport {
                path: path.to_path_buf(),
            });
        });

        let sourcelist: Vec<OmaSourceEntry> = source_lookup
            .entries()
            .iter()
            .map(|entry| OmaSourceEntry::new(entry.clone(), Arc::from(self.arch.as_str())))
            .collect();

        if !self.download_dir.is_dir() {
            std::fs::create_dir_all(&self.download_dir).map_err(|e| {
                RefreshError::FailedToOperateDirOrFile(self.download_dir.display().to_string(), e)
            })?;
        }

        // Create `apt update` file lock
        let _fd = get_file_lock(&self.download_dir.join("lock")).map_err(RefreshError::SetLock)?;

        detect_duplicate_repositories(&sourcelist)?;

        let mut download_list = HashSet::new();

        let self_arc = Arc::new(self);
        let sc = self_arc.clone();

        let (tx, rx) = flume::unbounded::<Event>();

        let _async_rt_keep_alive;
        let async_rt_handle = if let Ok(h) = tokio::runtime::Handle::try_current() {
            h
        } else {
            let rt = tokio::runtime::Builder::new_multi_thread()
                .enable_all()
                .build()
                .map_err(RefreshError::CreateTokioRuntime)?;
            let h = rt.handle().clone();
            _async_rt_keep_alive = Some(rt);
            h
        };

        let mirror_sources = MirrorSources::from_sourcelist(&sourcelist)?;
        let (mut mirror_sources, not_found) =
            run_task_with_pump(&async_rt_handle, &rx, &mut callback, async move {
                sc.download_releases(mirror_sources, tx).await
            })?;

        self_arc.refresh_topics(not_found, &mut mirror_sources, &mut callback)?;

        download_list.extend(
            mirror_sources
                .0
                .iter()
                .flat_map(|x| x.file_name().map(|s| s.to_string())),
        );

        let (tasks, total, optional_index_files) =
            self_arc.collect_all_release_entry(&apt_cfg, mirror_sources)?;

        debug!("oma will download source metadata: {tasks:#?}");

        if tasks.is_empty() {
            return Err(RefreshError::NoMetadataToDownload);
        }

        for i in &tasks {
            download_list.insert(i.filename.clone());
        }

        // 退出 topic / 移除源时，列表状态的变化可能只是清理掉失效的列表文件
        // （不涉及任何下载）。记录是否真的删了文件，供下面的 success invoke
        // 判断使用——否则下游（如 amo 的搜索索引）永远不会得知源集合变了。
        let removed_unused =
            remove_unused_db(&self_arc.download_dir, download_list).unwrap_or(false);

        let sc2 = self_arc.clone();
        let (tx, rx) = flume::unbounded::<Event>();
        let res = run_task_with_pump(&async_rt_handle, &rx, &mut callback, async move {
            sc2.download_release_data(tx, tasks, total, optional_index_files)
                .await
        })?;

        // 有元数据更新、或清理了失效的列表文件（如退出 topic），
        // 才执行 success invoke；否则列表状态虽然变了，下游缓存
        // （如 amo 的搜索索引）却收不到失效通知。
        let should_run_invoke = res.has_wrote() || removed_unused;

        if should_run_invoke {
            callback(Event::RunInvokeScript);
            self_arc.run_success_post_invoke(&apt_cfg);
        }

        callback(Event::Done);

        Ok(res.success)
    }

    fn init_apt_config(&self) -> AptConfig {
        let mut cfg = AptConfig::new();
        cfg.init_defaults()
            .expect("failed to initialize APT configuration");
        let _ = cfg.load_system();

        if !is_termux() {
            cfg.set("Dir", &self.source.to_string_lossy());
        }

        // Set default compression order if not configured
        let has_order = cfg
            .keys_under("Acquire::CompressionTypes::Order")
            .next()
            .is_some();
        if !has_order {
            for c in &["zst", "xz", "bz2", "lzma", "gz", "lz4"] {
                cfg.set_list("Acquire::CompressionTypes::Order", c);
            }
        }

        cfg
    }

    async fn download_release_data(
        &self,
        tx: Sender<Event>,
        tasks: Vec<DownloadEntry>,
        total: u64,
        optional_index_files: HashSet<String>,
    ) -> Result<Summary> {
        let dm = DownloadManager::builder()
            .client(self.client.clone())
            .download_list(tasks.into())
            .threads(self.threads)
            .total_size(total)
            .build();

        let optional_index_files = Arc::new(optional_index_files);
        let optional_files_ref = optional_index_files.clone();

        let res = dm
            .start_download(move |event| {
                let tx = tx.clone();
                let optional_files_ref = optional_files_ref.clone();
                async move {
                    let mut optional = false;

                    if let oma_fetch::Event::Failed { file_name, .. } = &event
                        && optional_files_ref.contains(file_name)
                    {
                        optional = true;
                    }

                    if !optional {
                        let _ = tx.send_async(Event::DownloadEvent(event)).await;
                    }
                }
            })
            .await
            .map_err(RefreshError::DownloadManagerBuilderError)?;

        let mut raise_err = false;

        for fail in &res.failed {
            if optional_index_files.contains(fail) {
                debug!("Failed to download optional metadata file {fail}, ignoring.");
            } else {
                raise_err = true;
            }
        }

        if raise_err {
            return Err(RefreshError::DownloadFailed(None));
        }

        Ok(res)
    }

    fn run_success_post_invoke(&self, cfg: &AptConfig) {
        // 本 crate 的配置解析器把列表项存为 `KEY::{item}`（见
        // `config_parser::handle_list_value`），不是 apt 的 `KEY#N` 约定，
        // 因此要像 `sourceslist::ignores` 一样用 `keys_under` + `get(KEY::{k})`
        // 读取，否则永远取不到任何命令。
        let cmds: Vec<String> = cfg
            .keys_under("APT::Update::Post-Invoke-Success")
            .map(|k| cfg.get(&format!("APT::Update::Post-Invoke-Success::{k}"), ""))
            .filter(|s| !s.is_empty())
            .collect();

        for cmd in &cmds {
            use std::process::Command;

            debug!("Running post-invoke script: {cmd}");
            let output = Command::new("sh").arg("-c").arg(cmd).output();

            match output {
                Ok(output) => {
                    if !output.status.success() {
                        warn!(
                            "Command {cmd} returned non-zero exit code: {}",
                            output.status.code().unwrap_or(1)
                        );
                        continue;
                    }
                    debug!("Command {cmd} completed successfully.");
                }
                Err(e) => {
                    warn!("Command {cmd} exited with error: {e}");
                }
            }
        }
    }

    async fn download_releases(
        &self,
        mut mirror_sources: MirrorSources,
        sender: Sender<Event>,
    ) -> Result<(MirrorSources, Vec<Url>)> {
        #[cfg(feature = "aosc")]
        let mut not_found = vec![];

        #[cfg(not(feature = "aosc"))]
        let not_found = vec![];

        let results = mirror_sources
            .fetch_all_release(
                self.client.clone(),
                Arc::from(self.download_dir.as_ref()),
                self.threads,
                sender.clone(),
            )
            .await;

        debug!("download_releases returned: {:?}", results);

        #[cfg(feature = "aosc")]
        for result in results {
            if let Err(e) = result {
                match e {
                    RefreshError::DownloadFailed(Some(
                        SingleDownloadError::ReqwestMiddlewareError { source },
                    )) if source
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

        Ok((mirror_sources, not_found))
    }

    #[cfg(feature = "aosc")]
    fn refresh_topics(
        &self,
        not_found: Vec<url::Url>,
        sources: &mut MirrorSources,
        callback: &mut (impl FnMut(Event) + 'static),
    ) -> Result<()> {
        use std::cell::RefCell;

        if !self.refresh_topics || not_found.is_empty() {
            return Ok(());
        }

        let mut tm = TopicManager::new(
            self.client.clone(),
            &self.source,
            self.arch.to_string(),
            false,
        )?;

        tm.refresh()?;
        let removed_suites = tm.remove_closed_topics()?;

        debug!("Removed suites: {:?}", removed_suites);

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

            callback(Event::ClosingTopic(suite));
        }

        tm.write_enabled(false)?;

        let cb_cell = RefCell::new(&mut *callback);

        tm.write_sources_list(&self.topic_msg, false, |topic, mirror| {
            let mut cb = cb_cell.borrow_mut();
            cb(Event::TopicNotInMirror { topic, mirror });
        })?;

        callback(Event::DownloadEvent(oma_fetch::Event::ProgressDone(1)));

        Ok(())
    }

    #[cfg(not(feature = "aosc"))]
    fn refresh_topics(
        &self,
        _not_found: Vec<url::Url>,
        _sources: &mut MirrorSources,
        _callback: &mut (impl FnMut(Event) + 'static),
    ) -> Result<()> {
        Ok(())
    }

    fn collect_all_release_entry(
        &self,
        apt_cfg: &AptConfig,
        mirror_sources: MirrorSources,
    ) -> Result<(Vec<DownloadEntry>, u64, HashSet<String>)> {
        let mut total = 0;
        let mut tasks = vec![];

        let index_target_config = IndexTargetConfig::new_from_apt_config(apt_cfg, &self.arch);

        let archs_from_file = std::fs::read_to_string("/var/lib/dpkg/arch")
            .ok()
            .map(|file| file.lines().map(|x| x.to_string()).collect::<Vec<_>>())
            .filter(|res| !res.is_empty());

        let mut flat_repo_no_release = vec![];
        let mut optional_index_files = HashSet::with_hasher(ahash::RandomState::new());

        for m in &mirror_sources.0 {
            if m.file_name().is_none() {
                flat_repo_no_release.push(m);
            }
        }
        for i in flat_repo_no_release {
            collect_flat_repo_no_release(i, &self.download_dir, &mut tasks)?;
        }

        for m in &mirror_sources.0 {
            let Some(file_name) = m.file_name() else {
                continue;
            };
            let inrelease_path = self.download_dir.join(file_name);
            let mut handle = HashSet::with_hasher(ahash::RandomState::new());

            let inrelease = std::fs::read_to_string(&inrelease_path).map_err(|e| {
                RefreshError::FailedToOperateDirOrFile(inrelease_path.display().to_string(), e)
            })?;

            let inrelease = verify_inrelease(
                &inrelease,
                m.signed_by(),
                &self.source,
                &inrelease_path,
                m.trusted(),
            )
            .map_err(|e| RefreshError::InReleaseParseError(inrelease_path.clone(), e))?;

            let release: Release = inrelease
                .parse()
                .map_err(|e| RefreshError::InReleaseParseError(inrelease_path.clone(), e))?;

            if !m.is_flat() {
                let now = Timestamp::now();
                release
                    .check_date(&now)
                    .map_err(|e| RefreshError::InReleaseParseError(inrelease_path.clone(), e))?;
                release
                    .check_valid_until(&now)
                    .map_err(|e| RefreshError::InReleaseParseError(inrelease_path.clone(), e))?;
            }

            let checksums = &release
                .get_or_try_init_checksum_type_and_list()
                .map_err(|e| RefreshError::InReleaseParseError(inrelease_path.clone(), e))?
                .1;

            // 仓库在 `Architectures:` 字段中声明的架构；若字段缺失则视为支持全部架构。
            let supported_archs = release.supported_architectures();

            let arch_from_local_configure = if let Some(ref f) = archs_from_file {
                f.iter().map(|x| x.as_str()).collect::<Vec<_>>()
            } else {
                vec![self.arch.as_str()]
            };

            for ose in &m.sources {
                let archs = if let Some(archs) = ose.archs()
                    && !archs.is_empty()
                {
                    let archs = archs.iter().map(|x| x.as_str()).collect::<Vec<_>>();
                    if arch_from_local_configure.iter().all(|x| !archs.contains(x))
                        && !archs.contains(&"all")
                        && !archs.contains(&"any")
                    {
                        warn!(
                            "Mirror {} does not contain architectures enabled in local configuration...",
                            ose.url()
                        );
                    }
                    archs
                } else {
                    arch_from_local_configure.clone()
                };

                let download_list = index_target_config.get_download_list(
                    ose.suite(),
                    checksums,
                    ose.is_source(),
                    ose.is_flat(),
                    archs,
                    ose.components(),
                    supported_archs.as_deref(),
                )?;
                get_all_need_db_from_config(download_list, &mut total, checksums, &mut handle);
            }

            for c in &handle {
                collect_download_task(
                    c,
                    m,
                    &self.download_dir,
                    &mut tasks,
                    &release,
                    &mut optional_index_files,
                )?;
            }
        }

        Ok((tasks, total, optional_index_files))
    }
}

pub fn content_length(resp: &Response) -> u64 {
    let content_length = resp
        .headers()
        .get(CONTENT_LENGTH)
        .map(Cow::Borrowed)
        .unwrap_or(Cow::Owned(HeaderValue::from(0)));

    content_length
        .to_str()
        .ok()
        .and_then(|x| x.parse::<u64>().ok())
        .unwrap_or_default()
}

fn detect_duplicate_repositories(sourcelist: &[OmaSourceEntry]) -> Result<()> {
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

/// Remove lists files no longer produced by any configured source (e.g.
/// after opting out of a topic), returning whether anything was removed.
fn remove_unused_db(download_dir: &Path, download_list: HashSet<String>) -> Result<bool> {
    let download_dir = std::fs::read_dir(download_dir)
        .map_err(|e| RefreshError::ReadDownloadDir(download_dir.display().to_string(), e))?;

    fn should_keep(entry: &DirEntry, download_list: &HashSet<String>) -> bool {
        if let Some(s) = entry.file_name().to_str() {
            download_list.contains(s)
        } else {
            download_list.contains(&entry.file_name().to_string_lossy().into_owned())
        }
    }

    let mut removed = false;
    for x in download_dir {
        if let Ok(x) = x
            && x.path().is_file()
            && !should_keep(&x, &download_list)
            && x.file_name() != "lock"
        {
            debug!("Removing {:?}", x.file_name());
            if let Err(e) = std::fs::remove_file(x.path()) {
                debug!("Failed to remove file {:?}: {e}", x.file_name());
            } else {
                removed = true;
            }
        }
    }

    Ok(removed)
}

fn collect_flat_repo_no_release(
    mirror_source: &MirrorSource,
    download_dir: &Path,
    tasks: &mut Vec<DownloadEntry>,
) -> Result<()> {
    let msg = mirror_source.get_human_download_message(Some("Packages"))?;

    let dist_url = mirror_source.dist_path();

    let from = match mirror_source.from()? {
        OmaSourceEntryFrom::Http => DownloadSourceType::Http,
        OmaSourceEntryFrom::Local => DownloadSourceType::Local(mirror_source.is_flat()),
    };

    let download_url = format!("{dist_url}/Packages");
    let file_path = format!("{dist_url}Packages");

    let sources = vec![DownloadSource {
        url: download_url.clone(),
        source_type: from,
    }];

    let task = DownloadEntry::builder()
        .source(sources)
        .filename(url_to_list_filename(&file_path)?)
        .dir(download_dir.to_path_buf())
        .allow_resume(false)
        .msg(msg.into())
        .file_type(CompressType::None)
        .build();

    tasks.push(task);

    Ok(())
}

fn collect_download_task(
    c: &ChecksumDownloadEntry,
    mirror_source: &MirrorSource,
    download_dir: &Path,
    tasks: &mut Vec<DownloadEntry>,
    release: &Release,
    optional_set: &mut HashSet<String>,
) -> Result<()> {
    let file_type = &c.msg;

    let msg = mirror_source.get_human_download_message(Some(file_type))?;

    let from = match mirror_source.from()? {
        OmaSourceEntryFrom::Http => DownloadSourceType::Http,
        OmaSourceEntryFrom::Local => DownloadSourceType::Local(
            mirror_source.is_flat()
                && (!file_is_compress(&c.item.name)
                    || (file_is_compress(&c.item.name) && c.keep_compress)),
        ),
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

    // When `Acquire-By-Hash: yes` is set, prefer the by-hash path, but fall
    // back to the traditional by-name path if the by-hash file is missing
    // (e.g. HTTP 404). The download manager tries the sources in order and
    // moves on to the next one if the current one fails.
    let mut sources = vec![];

    if release.acquire_by_hash() {
        let path = Path::new(&c.item.name);
        let parent = path.parent().unwrap_or(path);
        let dir = match release.checksum_type_and_list().0 {
            InReleaseChecksum::Sha256 => "SHA256",
            InReleaseChecksum::Sha512 => "SHA512",
            InReleaseChecksum::Md5 => "MD5Sum",
        };

        let path = parent.join("by-hash").join(dir).join(&c.item.checksum);

        sources.push(DownloadSource {
            url: mirror_source.get_download_url(&path.display().to_string()),
            source_type: from.clone(),
        });
    }

    sources.push(DownloadSource {
        url: mirror_source.get_download_url(&c.item.name),
        source_type: from,
    });

    let file_name = if c.keep_compress {
        mirror_source.get_download_file_name(Some(&c.item.name))?
    } else {
        mirror_source.get_download_file_name(Some(&not_compress_filename_before))?
    };

    if c.optional {
        optional_set.insert(file_name.clone());
    }

    let task = DownloadEntry::builder()
        .source(sources)
        .filename(file_name)
        .dir(download_dir.join("partial"))
        .allow_resume(false)
        .msg(msg.into())
        .final_dir(download_dir.to_path_buf())
        .by_hash_fallback(release.acquire_by_hash())
        .file_type({
            if c.keep_compress {
                CompressType::None
            } else {
                match Path::new(&c.item.name).extension().and_then(|x| x.to_str()) {
                    Some("gz") => CompressType::Gzip,
                    Some("xz") => CompressType::Xz,
                    Some("bz2") => CompressType::Bz2,
                    Some("zst") => CompressType::Zstd,
                    Some("lzma") => CompressType::Lzma,
                    Some("lz4") => CompressType::Lz4,
                    _ => CompressType::None,
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

    tasks.push(task);

    Ok(())
}

fn run_task_with_pump<Fut, T>(
    handle: &tokio::runtime::Handle,
    rx: &flume::Receiver<Event>,
    callback: &mut (impl FnMut(Event) + 'static),
    task: Fut,
) -> Result<T>
where
    Fut: std::future::Future<Output = Result<T>> + Send + 'static,
    T: Send + 'static,
{
    let (result_tx, result_rx) = flume::bounded(1);
    handle.spawn(async move {
        let res = task.await;
        let _ = result_tx.send(res);
    });

    while let Ok(event) = rx.recv() {
        callback(event);
    }

    result_rx
        .recv()
        .map_err(|_| RefreshError::DownloadFailed(None))?
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn remove_unused_db_reports_and_removes_stale_files() {
        let dir =
            std::env::temp_dir().join(format!("oma-remove-unused-test-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        // 需要保留的（仍在 download_list 里）
        std::fs::write(dir.join("stable_Packages"), b"x").unwrap();
        // 需要删除的（不在 download_list 里）
        std::fs::write(dir.join("core-14-preview_Packages"), b"x").unwrap();
        // lock 文件永远保留
        std::fs::write(dir.join("lock"), b"x").unwrap();

        let keep: HashSet<String> = ["stable_Packages".to_string()].into_iter().collect();
        let removed = remove_unused_db(&dir, keep).unwrap();

        assert!(removed, "should report that stale files were removed");
        assert!(!dir.join("core-14-preview_Packages").exists());
        assert!(dir.join("stable_Packages").exists());
        assert!(dir.join("lock").exists());

        // 再次调用：没有可删的文件了，必须返回 false
        let keep: HashSet<String> = ["stable_Packages".to_string()].into_iter().collect();
        let removed_again = remove_unused_db(&dir, keep).unwrap();
        assert!(!removed_again, "nothing left to remove");

        std::fs::remove_dir_all(&dir).ok();
    }
}
