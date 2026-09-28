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

use oma_fetch::{SingleDownloadError, Summary, TaskTracker};
#[cfg(feature = "aosc")]
use oma_topics::TopicManager;

#[cfg(feature = "aosc")]
use oma_fetch::reqwest::StatusCode;

use oma_logger::{debug, warn};
use oma_utils::{GetLockError, get_file_lock, is_termux};
use reqwest_middleware::ClientWithMiddleware;
use serde::{Deserialize, Serialize};
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
    #[error("Refresh was canceled")]
    Canceled,
}

type Result<T> = std::result::Result<T, RefreshError>;

/// 取消通道的句柄：需要取消的一方持有它，调用 [`CancelHandle::cancel`]
/// 即可中止对应的刷新。句柄与令牌通过 [`cancel_channel`] 一起创建。
#[derive(Clone)]
pub struct CancelHandle(flume::Sender<()>);

impl CancelHandle {
    /// 请求取消刷新；重复调用无副作用。
    pub fn cancel(&self) {
        let _ = self.0.try_send(());
    }
}

/// 取消通道的令牌：交给 `OmaRefresh` 构建器的 `cancel_token`，刷新收到
/// 取消信号后会立即中止并返回 [`RefreshError::Canceled`]。句柄被丢弃但
/// 未发出信号时，刷新照常进行。
pub struct CancelToken(flume::Receiver<()>);

/// 创建一对取消句柄与令牌：句柄留给需要取消的一方，令牌交给刷新。
pub fn cancel_channel() -> (CancelHandle, CancelToken) {
    let (tx, rx) = flume::bounded(1);
    (CancelHandle(tx), CancelToken(rx))
}

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
    /// An externally-supplied APT configuration, shared via [`Arc`]. When
    /// omitted, a fresh one is built from the system defaults inside
    /// [`OmaRefresh`].
    apt_config: Option<Arc<AptConfig>>,
    /// 可选的取消令牌（见 [`cancel_channel`]）：一旦收到取消信号，正在
    /// 进行的刷新会立即中止（尚未完成的下载被丢弃）并返回
    /// [`RefreshError::Canceled`]。句柄被丢弃但未发出信号时刷新照常进行。
    cancel_token: Option<CancelToken>,
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
    pub fn start(
        mut self,
        mut callback: impl FnMut(Event) + 'static,
    ) -> Result<Vec<SuccessSummary>> {
        if self.threads == 0 || self.threads > 255 {
            return Err(RefreshError::WrongThreadCount(self.threads));
        }

        if is_canceled(self.cancel_token.as_ref()) {
            return Err(RefreshError::Canceled);
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

        // 下载子任务的追踪器（见 `run_task_with_pump` 的取消分支）：取消时
        // 要等所有子任务真正退出，列表目录锁的释放才不会早于子任务落盘。
        let tracker = TaskTracker::new();

        let mirror_sources = MirrorSources::from_sourcelist(&sourcelist)?;
        let tracker_for_release = tracker.clone();
        let (mirror_sources, not_found) = run_task_with_pump(
            &async_rt_handle,
            &rx,
            &mut callback,
            self_arc.cancel_token.as_ref(),
            Some(&tracker),
            async move {
                sc.download_releases(mirror_sources, tx, tracker_for_release)
                    .await
            },
        )?;

        // topic 刷新会联网、写 atm 状态与源列表文件：先检查一次取消，
        // 再放进和下载一样的可取消事件泵里，取消能中断网络请求。
        if is_canceled(self_arc.cancel_token.as_ref()) {
            return Err(RefreshError::Canceled);
        }

        let sc_topic = self_arc.clone();
        let (tx, rx) = flume::unbounded::<Event>();
        let mirror_sources = run_task_with_pump(
            &async_rt_handle,
            &rx,
            &mut callback,
            self_arc.cancel_token.as_ref(),
            None,
            async move { sc_topic.refresh_topics(not_found, mirror_sources, tx).await },
        )?;

        if is_canceled(self_arc.cancel_token.as_ref()) {
            return Err(RefreshError::Canceled);
        }

        download_list.extend(
            mirror_sources
                .0
                .iter()
                .flat_map(|x| x.file_name().map(|s| s.to_string())),
        );

        let (tasks, total, optional_index_files) =
            self_arc.collect_all_release_entry(&apt_cfg, mirror_sources)?;

        debug!("oma will download source metadata: {tasks:#?}");

        if is_canceled(self_arc.cancel_token.as_ref()) {
            return Err(RefreshError::Canceled);
        }

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
        let tracker_for_data = tracker.clone();
        let (tx, rx) = flume::unbounded::<Event>();
        let res = run_task_with_pump(
            &async_rt_handle,
            &rx,
            &mut callback,
            self_arc.cancel_token.as_ref(),
            Some(&tracker),
            async move {
                sc2.download_release_data(tx, tasks, total, optional_index_files, tracker_for_data)
                    .await
            },
        )?;

        // 结果就绪与取消信号可能同时到达，事件泵不保证优先选中取消；
        // 各阶段返回后都再确认一次，避免带着未处理的取消继续收尾。
        if is_canceled(self_arc.cancel_token.as_ref()) {
            return Err(RefreshError::Canceled);
        }

        // 有元数据更新、或清理了失效的列表文件（如退出 topic），
        // 才执行 success invoke；否则列表状态虽然变了，下游缓存
        // （如 amo 的搜索索引）却收不到失效通知。
        let should_run_invoke = res.has_wrote() || removed_unused;

        if should_run_invoke {
            callback(Event::RunInvokeScript);

            // 钩子执行期间会盯着取消信号（必要时终止子进程）；被取消
            // 就不再继续收尾。
            if !self_arc.run_success_post_invoke(&apt_cfg) {
                return Err(RefreshError::Canceled);
            }
        }

        // 报告完成前最后再确认一次取消（比如信号恰好在钩子收尾时到达）。
        if is_canceled(self_arc.cancel_token.as_ref()) {
            return Err(RefreshError::Canceled);
        }

        callback(Event::Done);

        Ok(res.success)
    }

    fn init_apt_config(&mut self) -> AptConfig {
        // 调用方传入的 Arc 配置在需要写入（Dir、压缩顺序）时按写时复制 clone
        // 一份，调用方本身无需深拷贝；未传入时新建一份并初始化默认值与系统配置。
        let mut cfg = match self.apt_config.take() {
            Some(arc) => Arc::unwrap_or_clone(arc),
            None => {
                let mut cfg = AptConfig::new();
                cfg.init_defaults()
                    .expect("failed to initialize APT configuration");
                let _ = cfg.load_system();
                cfg
            }
        };

        if !is_termux() {
            cfg.set("Dir", &self.source.to_string_lossy());
        }

        // AOSC 仓库默认优先 zst；非 aosc 构建沿用系统配置，不强加默认顺序。
        #[cfg(feature = "aosc")]
        {
            let has_order = cfg
                .keys_under("Acquire::CompressionTypes::Order")
                .next()
                .is_some();
            if !has_order {
                for c in crate::config::DECODABLE_COMPRESSION_FORMATS {
                    cfg.set_list("Acquire::CompressionTypes::Order", c);
                }
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
        tracker: TaskTracker,
    ) -> Result<Summary> {
        let dm = DownloadManager::builder()
            .client(self.client.clone())
            .download_list(tasks.into())
            .threads(self.threads)
            .total_size(total)
            .tracker(tracker)
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

    /// 执行 `APT::Update::Post-Invoke-Success` 里的钩子（失败只告警）。
    ///
    /// 返回 `false` 表示执行期间收到了取消信号（当前子进程已被终止）。
    fn run_success_post_invoke(&self, cfg: &AptConfig) -> bool {
        // 本 crate 的配置解析器把列表项存为 `KEY::{item}`（见
        // `config_parser::handle_list_value`），不是 apt 的 `KEY#N` 约定，
        // 因此要像 `sourceslist::ignores` 一样用 `keys_under` + `get(KEY::{k})`
        // 读取，否则永远取不到任何命令。
        let cmds: Vec<String> = cfg
            .keys_under("APT::Update::Post-Invoke-Success")
            .map(|k| cfg.get(&format!("APT::Update::Post-Invoke-Success::{k}"), ""))
            .filter(|s| !s.is_empty())
            .collect();

        run_post_invoke_commands(&cmds, self.cancel_token.as_ref())
    }

    async fn download_releases(
        &self,
        mut mirror_sources: MirrorSources,
        sender: Sender<Event>,
        tracker: TaskTracker,
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
                tracker,
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
    async fn refresh_topics(
        &self,
        not_found: Vec<url::Url>,
        mut sources: MirrorSources,
        tx: Sender<Event>,
    ) -> Result<MirrorSources> {
        if !self.refresh_topics || not_found.is_empty() {
            return Ok(sources);
        }

        let mut tm = TopicManager::new(
            self.client.clone(),
            &self.source,
            self.arch.to_string(),
            false,
        )?;

        tm.refresh_async().await?;
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

            let _ = tx.send(Event::ClosingTopic(suite));
        }

        tm.write_enabled(false)?;

        // 函数跑在事件泵的任务里，事件经 flume 通道交给 `start` 的回调。
        let tx_cb = tx.clone();
        tm.write_sources_list(&self.topic_msg, false, |topic, mirror| {
            let _ = tx_cb.send(Event::TopicNotInMirror { topic, mirror });
        })?;

        let _ = tx.send(Event::DownloadEvent(oma_fetch::Event::ProgressDone(1)));

        Ok(sources)
    }

    #[cfg(not(feature = "aosc"))]
    async fn refresh_topics(
        &self,
        _not_found: Vec<url::Url>,
        sources: MirrorSources,
        _tx: Sender<Event>,
    ) -> Result<MirrorSources> {
        Ok(sources)
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

        // Only the preferred (first) compression variant will actually be
        // downloaded; the rest are fallbacks tried on failure. Count only its
        // size toward the total so the progress bar reflects the expected
        // download size.
        let item = &i.items[0];

        if i.keep_compress {
            *total += item.size;
        } else {
            let size = if file_is_compress(&item.name) {
                let (_, name_without_compress) = split_ext_and_filename(&item.name);

                checksums
                    .iter()
                    .find_map(|x| {
                        if x.name == name_without_compress {
                            Some(x.size)
                        } else {
                            None
                        }
                    })
                    .unwrap_or(item.size)
            } else {
                item.size
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
        file_type: CompressType::None,
    }];

    let task = DownloadEntry::builder()
        .source(sources)
        .filename(url_to_list_filename(&file_path)?)
        .dir(download_dir.to_path_buf())
        .allow_resume(false)
        .msg(msg.into())
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
    // The preferred (first) compression variant drives the message, filename,
    // checksum and primary file type; the remaining variants are fallbacks
    // tried in order if the preferred one is unavailable.
    let item = &c.items[0];

    let file_type = &c.msg;

    let msg = mirror_source.get_human_download_message(Some(file_type))?;

    let from = match mirror_source.from()? {
        OmaSourceEntryFrom::Http => DownloadSourceType::Http,
        OmaSourceEntryFrom::Local => DownloadSourceType::Local(
            mirror_source.is_flat()
                && (!file_is_compress(&item.name)
                    || (file_is_compress(&item.name) && c.keep_compress)),
        ),
    };

    let not_compress_filename_before = if file_is_compress(&item.name) {
        Cow::Owned(split_ext_and_filename(&item.name).1)
    } else {
        Cow::Borrowed(&item.name)
    };

    let checksum = if c.keep_compress {
        Some(&item.checksum)
    } else {
        release
            .checksum_type_and_list()
            .1
            .iter()
            .find(|x| x.name == *not_compress_filename_before)
            .as_ref()
            .map(|c| &c.checksum)
    };

    // Build one source per compression variant, ordered best-first. The
    // download manager tries the sources in order and falls back to the next
    // one on failure (e.g. HTTP 404), matching apt's
    // `Acquire::CompressionTypes::Order` behavior. Each source carries its own
    // `file_type` so the correct decompressor is used for whichever variant
    // actually succeeds.
    //
    // When `keep_compress` is set the stored filename embeds the compression
    // extension and the checksum is that of the compressed file, so a
    // transparent fallback to a different compression format is impossible;
    // in that case only the preferred variant is tried.
    let variants: &[ChecksumItem] = if c.keep_compress {
        &c.items[..1]
    } else {
        &c.items
    };

    let mut sources = vec![];

    for variant in variants {
        // KeepCompressed means the downloaded bytes must be stored as-is.
        // The checksum in that mode is for the compressed variant, so passing
        // its decompressor here would both corrupt the stored file and make
        // checksum verification fail.
        let variant_file_type = if c.keep_compress {
            CompressType::None
        } else {
            compress_type_of(&variant.name)
        };

        // When `Acquire-By-Hash: yes` is set, prefer the by-hash path, but
        // fall back to the traditional by-name path if the by-hash file is
        // missing (e.g. HTTP 404).
        if release.acquire_by_hash() {
            let path = Path::new(&variant.name);
            let parent = path.parent().unwrap_or(path);
            let dir = match release.checksum_type_and_list().0 {
                InReleaseChecksum::Sha256 => "SHA256",
                InReleaseChecksum::Sha512 => "SHA512",
                InReleaseChecksum::Md5 => "MD5Sum",
            };

            let path = parent.join("by-hash").join(dir).join(&variant.checksum);

            sources.push(DownloadSource {
                url: mirror_source.get_download_url(&path.display().to_string()),
                source_type: from.clone(),
                file_type: variant_file_type,
            });
        }

        sources.push(DownloadSource {
            url: mirror_source.get_download_url(&variant.name),
            source_type: from.clone(),
            file_type: variant_file_type,
        });
    }

    let file_name = if c.keep_compress {
        mirror_source.get_download_file_name(Some(&item.name))?
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

/// Map a file name to its [`CompressType`], used to pick the correct
/// decompressor for a downloaded source.
fn compress_type_of(name: &str) -> CompressType {
    match Path::new(name).extension().and_then(|x| x.to_str()) {
        Some("gz") => CompressType::Gzip,
        Some("xz") => CompressType::Xz,
        Some("bz2") => CompressType::Bz2,
        Some("zst") => CompressType::Zstd,
        Some("lzma") => CompressType::Lzma,
        Some("lz4") => CompressType::Lz4,
        _ => CompressType::None,
    }
}

/// 令牌是否已收到取消信号；`None` 表示调用方没有提供令牌，永远不取消。
/// 句柄已断开但未发送信号的令牌不算取消。
fn is_canceled(cancel_token: Option<&CancelToken>) -> bool {
    cancel_token.is_some_and(|token| matches!(token.0.try_recv(), Ok(())))
}

/// 逐个执行钩子命令，期间盯着取消信号：收到就终止当前子进程、不再跑
/// 后续命令并返回 `false`；全部跑完返回 `true`。
///
/// 命令的输出与原来的 `Command::output()` 一样不落地；不用 `output()`
/// 是因为它阻塞等待、看不到取消信号，慢的或卡死的钩子会把取消卡住。
fn run_post_invoke_commands(cmds: &[String], cancel_token: Option<&CancelToken>) -> bool {
    use std::{
        process::{Command, Stdio},
        time::Duration,
    };

    for cmd in cmds {
        if is_canceled(cancel_token) {
            return false;
        }

        debug!("Running post-invoke script: {cmd}");

        let mut child = match Command::new("sh")
            .arg("-c")
            .arg(cmd)
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
        {
            Ok(child) => child,
            Err(e) => {
                warn!("Command {cmd} exited with error: {e}");
                continue;
            }
        };

        let canceled = loop {
            if is_canceled(cancel_token) {
                debug!("Command {cmd} canceled, terminating it");
                let _ = child.kill();
                let _ = child.wait();
                break true;
            }

            match child.try_wait() {
                Ok(Some(status)) => {
                    if status.success() {
                        debug!("Command {cmd} completed successfully.");
                    } else {
                        warn!(
                            "Command {cmd} returned non-zero exit code: {}",
                            status.code().unwrap_or(1)
                        );
                    }
                    break false;
                }
                Ok(None) => std::thread::sleep(Duration::from_millis(20)),
                Err(e) => {
                    warn!("Command {cmd} exited with error: {e}");
                    break false;
                }
            }
        };

        if canceled {
            return false;
        }
    }

    true
}

fn run_task_with_pump<Fut, T>(
    handle: &tokio::runtime::Handle,
    rx: &flume::Receiver<Event>,
    callback: &mut (impl FnMut(Event) + 'static),
    cancel_token: Option<&CancelToken>,
    tracker: Option<&TaskTracker>,
    task: Fut,
) -> Result<T>
where
    Fut: std::future::Future<Output = Result<T>> + Send + 'static,
    T: Send + 'static,
{
    // 事件泵可被唤醒的事件。
    enum Pumped<T> {
        Event(Event),
        // 事件发送端已全部断开：事件流结束，但任务可能还在收尾。
        EventsDone,
        // 任务结束，结果已就绪。
        TaskDone(Result<T>),
        // 结果通道断开但没有结果（任务被丢弃或 panic）。
        TaskResultGone,
        // 收到取消信号。
        Canceled,
        // 取消通道断开但未发送信号：之后只泵事件。
        CancelGone,
    }

    let (result_tx, result_rx) = flume::bounded(1);
    let task_handle = handle.spawn(async move {
        let res = task.await;
        let _ = result_tx.send(res);
    });

    // 事件流可能先于任务结束（例如最后一个事件发送端在任务收尾前就被
    // 丢弃）：之后改为等结果，但取消信号要一直盯着——只等结果会把这段
    // 窗口里的取消漏掉，刷新会照跑不误地返回成功。
    let mut events_done = false;
    let mut cancel_token = cancel_token;
    loop {
        // 在事件、结果和取消信号之间阻塞等待：任意一个到达都会唤醒，
        // 无需轮询。取消时 abort 丢弃包装 future，oma-fetch 放在
        // `JoinSet` 里的下载任务随之取消。
        let mut selector = if events_done {
            flume::Selector::new().recv(&result_rx, |result| match result {
                Ok(result) => Pumped::TaskDone(result),
                Err(_) => Pumped::TaskResultGone,
            })
        } else {
            flume::Selector::new().recv(rx, |result| match result {
                Ok(event) => Pumped::Event(event),
                Err(_) => Pumped::EventsDone,
            })
        };
        if let Some(token) = cancel_token {
            selector = selector.recv(&token.0, |result| match result {
                Ok(()) => Pumped::Canceled,
                Err(_) => Pumped::CancelGone,
            });
        }

        match selector.wait() {
            Pumped::Event(event) => callback(event),
            // 事件流结束：继续等结果（或等取消）。
            Pumped::EventsDone => events_done = true,
            Pumped::TaskDone(result) => return result,
            Pumped::TaskResultGone => return Err(RefreshError::DownloadFailed(None)),
            Pumped::Canceled => {
                task_handle.abort();
                // 取消不是立即生效的：`abort` 只发出信号，运行时要到下一次
                // 调度才会丢弃包装任务。若在这里直接返回，`start` 会先释放
                // `download_dir/lock`，而任务 future 里 `JoinSet` 持有的下载
                // 子任务可能还没收到取消信号，已经下载完成的文件仍会被
                // rename 进列表目录，覆盖随后启动的新刷新写入的元数据。
                // 这里是同步上下文，用 `futures::executor::block_on` 等
                // JoinHandle 结束：tokio 保证此时任务析构已完成、子任务的
                // 取消信号已全部发出。
                let _ = futures::executor::block_on(task_handle);
                // 但 `JoinSet` 析构只是给子任务发取消信号、不等它们退出，
                // 还要等 tracker 归零；子任务对列表目录的写入（rename /
                // symlink / 删除）都是同步系统调用，不会变成在途的后台
                // 操作，因此 tracker 归零后，锁的释放就不会早于子任务的
                // 收尾。
                if let Some(tracker) = tracker {
                    tracker.wait();
                }
                return Err(RefreshError::Canceled);
            }
            Pumped::CancelGone => cancel_token = None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
    use std::time::Duration;

    #[test]
    fn cancel_handle_aborts_pump() {
        let rt = tokio::runtime::Builder::new_multi_thread()
            .enable_all()
            .build()
            .unwrap();
        let (tx, rx) = flume::unbounded::<Event>();
        let (cancel_handle, cancel_token) = cancel_channel();

        let handle_for_thread = cancel_handle.clone();
        std::thread::spawn(move || {
            std::thread::sleep(Duration::from_millis(80));
            handle_for_thread.cancel();
        });

        // 任务永不完成、也不发事件：只有取消句柄能中止事件泵。
        let mut callback = |_event: Event| {};
        let result: Result<()> = run_task_with_pump(
            rt.handle(),
            &rx,
            &mut callback,
            Some(&cancel_token),
            None,
            async {
                std::future::pending::<()>().await;
                Ok(())
            },
        );

        assert!(matches!(result, Err(RefreshError::Canceled)));
        drop(tx);
    }

    #[test]
    fn cancel_waits_for_task_shutdown() {
        let rt = tokio::runtime::Builder::new_multi_thread()
            .enable_all()
            .build()
            .unwrap();
        let (tx, rx) = flume::unbounded::<Event>();
        let (cancel_handle, cancel_token) = cancel_channel();

        let handle_for_thread = cancel_handle.clone();
        std::thread::spawn(move || {
            std::thread::sleep(Duration::from_millis(80));
            handle_for_thread.cancel();
        });

        // 任务永不完成、也不发事件，但带一个析构标记。
        struct SetOnDrop(Arc<AtomicBool>);
        impl Drop for SetOnDrop {
            fn drop(&mut self) {
                self.0.store(true, Ordering::SeqCst);
            }
        }

        let dropped = Arc::new(AtomicBool::new(false));
        let dropped_in_task = dropped.clone();

        let mut callback = |_event: Event| {};
        let result: Result<()> = run_task_with_pump(
            rt.handle(),
            &rx,
            &mut callback,
            Some(&cancel_token),
            None,
            async move {
                let _guard = SetOnDrop(dropped_in_task);
                std::future::pending::<()>().await;
                Ok(())
            },
        );

        assert!(matches!(result, Err(RefreshError::Canceled)));
        // 返回时必须已完成任务析构，锁的释放才不会早于取消收尾。
        assert!(
            dropped.load(Ordering::SeqCst),
            "run_task_with_pump returned before the task finished dropping"
        );
        drop(tx);
    }

    #[test]
    fn cancel_waits_for_tracked_children() {
        let rt = tokio::runtime::Builder::new_multi_thread()
            .enable_all()
            .build()
            .unwrap();
        let (tx, rx) = flume::unbounded::<Event>();
        let (cancel_handle, cancel_token) = cancel_channel();
        let tracker = TaskTracker::new();

        let handle_for_thread = cancel_handle.clone();
        std::thread::spawn(move || {
            std::thread::sleep(Duration::from_millis(80));
            handle_for_thread.cancel();
        });

        // 子任务永不完成；每个子任务析构时打一个标记。字段声明顺序保证
        // 先打标记、后注销（guard 最后被丢弃）。
        struct MarkedChild {
            _mark: MarkOnDrop,
            _guard: oma_fetch::TaskGuard,
        }

        struct MarkOnDrop(Arc<AtomicUsize>);

        impl Drop for MarkOnDrop {
            fn drop(&mut self) {
                self.0.fetch_add(1, Ordering::SeqCst);
            }
        }

        const CHILDREN: usize = 4;

        let marked = Arc::new(AtomicUsize::new(0));
        let marked_in_task = marked.clone();
        let tracker_in_task = tracker.clone();

        let mut callback = |_event: Event| {};
        let result: Result<()> = run_task_with_pump(
            rt.handle(),
            &rx,
            &mut callback,
            Some(&cancel_token),
            Some(&tracker),
            async move {
                let mut set = tokio::task::JoinSet::new();

                for _ in 0..CHILDREN {
                    let guard = tracker_in_task.guard();
                    let mark = MarkOnDrop(marked_in_task.clone());

                    set.spawn(async move {
                        let _child = MarkedChild {
                            _mark: mark,
                            _guard: guard,
                        };
                        std::future::pending::<()>().await;
                    });
                }

                while set.join_next().await.is_some() {}
                Ok(())
            },
        );

        assert!(matches!(result, Err(RefreshError::Canceled)));
        // 返回时所有子任务都必须已析构（标记全部落地），否则它们还可能
        // 在锁释放后继续落盘。
        assert_eq!(marked.load(Ordering::SeqCst), CHILDREN);
        drop(tx);
    }

    #[test]
    fn cancel_observed_while_waiting_for_result() {
        let rt = tokio::runtime::Builder::new_multi_thread()
            .enable_all()
            .build()
            .unwrap();
        let (tx, rx) = flume::unbounded::<Event>();
        let (cancel_handle, cancel_token) = cancel_channel();

        let handle_for_thread = cancel_handle.clone();
        std::thread::spawn(move || {
            std::thread::sleep(Duration::from_millis(80));
            handle_for_thread.cancel();
        });

        // 任务先丢弃事件发送端（事件流就此结束），但要很晚才返回结果：
        // 80ms 时的取消落在「等结果」阶段，必须被观察到。
        let (finish_tx, finish_rx) = flume::bounded::<()>(1);
        std::thread::spawn(move || {
            std::thread::sleep(Duration::from_millis(500));
            let _ = finish_tx.send(());
        });

        let mut callback = |_event: Event| {};
        let result: Result<()> = run_task_with_pump(
            rt.handle(),
            &rx,
            &mut callback,
            Some(&cancel_token),
            None,
            async move {
                drop(tx);
                let _ = finish_rx.recv_async().await;
                Ok(())
            },
        );

        assert!(matches!(result, Err(RefreshError::Canceled)));
    }

    #[test]
    fn post_invoke_stops_on_cancel() {
        let (cancel_handle, cancel_token) = cancel_channel();

        let handle_for_thread = cancel_handle.clone();
        std::thread::spawn(move || {
            std::thread::sleep(Duration::from_millis(100));
            handle_for_thread.cancel();
        });

        // 慢钩子（sleep 5）：取消到达时应把它终止，而不是等它跑完；
        // 后面的命令也不应再执行。
        let marker = std::env::temp_dir().join(format!(
            "oma-post-invoke-test-marker-{}",
            std::process::id()
        ));
        let _ = std::fs::remove_file(&marker);
        let cmds = vec!["sleep 5".to_string(), format!("touch {}", marker.display())];

        let start = std::time::Instant::now();
        let completed = run_post_invoke_commands(&cmds, Some(&cancel_token));
        let elapsed = start.elapsed();

        assert!(!completed, "post-invoke should report cancellation");
        assert!(
            elapsed < Duration::from_secs(3),
            "the running hook should be terminated on cancel, took {elapsed:?}"
        );
        assert!(
            !marker.exists(),
            "commands after a canceled hook should not run"
        );
    }

    #[test]
    fn dropped_cancel_handle_does_not_cancel() {
        let rt = tokio::runtime::Builder::new_multi_thread()
            .enable_all()
            .build()
            .unwrap();
        let (tx, rx) = flume::unbounded::<Event>();
        let (cancel_handle, cancel_token) = cancel_channel();
        // 句柄直接断开：不算取消，事件照常泵完、结果照常返回。
        drop(cancel_handle);

        let seen = Arc::new(AtomicUsize::new(0));
        let seen_in_callback = seen.clone();
        let mut callback = move |_event: Event| {
            seen_in_callback.fetch_add(1, Ordering::Relaxed);
        };
        let result: Result<u32> = run_task_with_pump(
            rt.handle(),
            &rx,
            &mut callback,
            Some(&cancel_token),
            None,
            async move {
                tx.send(Event::Done).unwrap();
                Ok(42)
            },
        );

        assert!(matches!(result, Ok(42)));
        assert_eq!(seen.load(Ordering::Relaxed), 1);
    }

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
