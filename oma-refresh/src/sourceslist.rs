use std::{borrow::Cow, fmt::Debug, path::Path, sync::Arc};

use ahash::{AHashMap, HashMap};
use fancy_regex::Regex;
use flume::Sender;
use oma_apt_pkg::AptConfig;
use oma_apt_sources_lists::{Signature, SourceEntry};
use oma_fetch::{
    SingleDownloadError,
    mirror::{MirrorCandidate, MirrorList, MirrorTarget, MirrorTransport, TargetKind},
    reqwest::{Method, Response, StatusCode},
    send_request_with_url_and_method,
};
use oma_logger::{debug, warn};
use oma_utils::concat_url;
use once_cell::sync::OnceCell;
use reqwest_middleware::ClientWithMiddleware;
use tokio::{
    fs::{self, File},
    io::AsyncWriteExt,
    sync::Semaphore,
    task::JoinSet,
};
use url::Url;

use crate::{
    db::{Event, RefreshError, content_length},
    util::{concat_url_only_check_once_slash, url_to_list_filename},
};

#[derive(Clone)]
pub struct OmaSourceEntry {
    source: SourceEntry,
    arch: Arc<str>,
    url: OnceCell<String>,
    suite: OnceCell<String>,
    dist_path: OnceCell<String>,
    from: OnceCell<OmaSourceEntryFrom>,
}

impl Debug for OmaSourceEntry {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("OmaSourceEntry")
            .field("url", &self.url)
            .field("options", &self.source.options)
            .field("suite", &self.suite)
            .field("dist_path", &self.dist_path)
            .field("from", &self.from)
            .field("is-src", &self.source.source)
            .finish()
    }
}

pub fn ignores(cfg: &AptConfig) -> Vec<Regex> {
    let ignores_lines = cfg
        .keys_under("Dir::Ignore-Files-Silently")
        .map(|k| cfg.get(&format!("Dir::Ignore-Files-Silently::{k}"), ""))
        .filter(|s| !s.is_empty());

    ignores_lines
        .filter_map(|re| {
            Regex::new(&re).inspect_err(|e| {
                warn!("Failed to parse regex {re} in ignore rule list (Dir::Ignore-Files-Silently): {e}")
            }).ok()
        })
        .collect::<Vec<_>>()
}

#[derive(PartialEq, Eq, Debug, Copy, Clone)]
pub enum OmaSourceEntryFrom {
    Http,
    Local,
}

impl OmaSourceEntry {
    pub fn new(source: SourceEntry, arch: Arc<str>) -> Self {
        Self {
            source,
            arch,
            url: OnceCell::new(),
            suite: OnceCell::new(),
            dist_path: OnceCell::new(),
            from: OnceCell::new(),
        }
    }

    pub fn from(&self) -> Result<&OmaSourceEntryFrom, RefreshError> {
        self.from.get_or_try_init(|| {
            let url = Url::parse(self.url())
                .map_err(|_| RefreshError::InvalidUrl(self.url().to_string()))?;

            match url.scheme() {
                "file" => Ok(OmaSourceEntryFrom::Local),
                "http" | "https" => Ok(OmaSourceEntryFrom::Http),
                x => Err(RefreshError::UnsupportedProtocol(x.to_string())),
            }
        })
    }

    /// Whether this entry uses apt's mirror protocol. Such a source does not
    /// name a transport itself; its mirror list must be resolved first.
    pub fn is_mirror(&self) -> bool {
        let url = self.url();
        url.starts_with("mirror:") || url.starts_with("mirror+")
    }

    pub fn components(&self) -> &[String] {
        &self.source.components
    }

    pub fn archs(&self) -> &Option<Vec<String>> {
        &self.source.archs
    }

    pub fn trusted(&self) -> bool {
        self.source.trusted
    }

    pub fn signed_by(&self) -> &Option<Signature> {
        &self.source.signed_by
    }

    pub fn url(&self) -> &str {
        self.url
            .get_or_init(|| self.source.url.replace("$(ARCH)", &self.arch))
    }

    pub fn is_flat(&self) -> bool {
        self.components().is_empty()
    }

    pub fn suite(&self) -> &str {
        self.suite
            .get_or_init(|| self.source.suite.replace("$(ARCH)", &self.arch))
    }

    pub fn is_source(&self) -> bool {
        self.source.source
    }

    pub fn dist_path(&self) -> &str {
        self.dist_path.get_or_init(|| {
            let suite = self.suite();
            let url = self.url();

            if self.is_flat() {
                if suite == "/" {
                    if !url.ends_with('/') {
                        format!("{url}{suite}")
                    } else {
                        url.to_string()
                    }
                } else {
                    concat_url_only_check_once_slash(url, suite)
                }
            } else {
                self.source.dist_path()
            }
        })
    }

    pub fn get_download_file_name(&self, file_name: Option<&str>) -> Result<String, RefreshError> {
        let url = if let Some(file_name) = file_name {
            Cow::Owned(concat_url_only_check_once_slash(
                self.dist_path(),
                file_name,
            ))
        } else {
            self.dist_path().into()
        };

        url_to_list_filename(&url)
    }

    #[inline]
    pub fn get_download_url(&self, file_name: &str) -> String {
        concat_url(self.dist_path(), file_name)
    }

    pub fn get_human_download_url(&self, file_name: Option<&str>) -> Result<String, RefreshError> {
        let url = self.url();
        let url = Url::parse(url).map_err(|_| RefreshError::InvalidUrl(url.to_string()))?;

        let host = url.host_str();

        let url = if let Some(host) = host {
            host
        } else {
            url.path()
        };

        let mut s = format!("{}:{}", url, self.suite());

        if let Some(file_name) = file_name {
            s.push(' ');
            s.push_str(file_name);
        }

        Ok(s)
    }
}

#[derive(Debug, Clone)]
pub struct MirrorSources(pub Vec<MirrorSource>);

#[derive(Debug, Clone)]
pub struct MirrorSource {
    pub sources: Vec<OmaSourceEntry>,
    release_file_name: OnceCell<String>,
    /// Mirror list for a `mirror://` source, resolved before any download.
    mirror_list: OnceCell<Arc<MirrorList>>,
}

impl MirrorSource {
    pub fn set_release_file_name(&self, file_name: String) {
        self.release_file_name
            .set(file_name)
            .expect("Release file name was init");
    }

    /// Whether this source group uses apt's mirror protocol.
    pub fn is_mirror(&self) -> bool {
        self.sources.first().is_some_and(OmaSourceEntry::is_mirror)
    }

    /// Attach the already-loaded mirror list. Called once per refresh before
    /// any release or index download.
    pub fn set_mirror_list(&self, list: Arc<MirrorList>) {
        let _ = self.mirror_list.set(list);
    }

    pub fn mirror_list(&self) -> Option<&MirrorList> {
        self.mirror_list.get().map(Arc::as_ref)
    }

    pub fn dist_path(&self) -> &str {
        self.sources.first().unwrap().dist_path()
    }

    #[cfg(feature = "aosc")]
    pub fn suite(&self) -> &str {
        self.sources.first().unwrap().suite()
    }

    #[inline]
    pub fn from(&self) -> Result<&OmaSourceEntryFrom, RefreshError> {
        self.sources.first().unwrap().from()
    }

    #[inline]
    pub fn get_human_download_message(
        &self,
        file_name: Option<&str>,
    ) -> Result<String, RefreshError> {
        self.sources
            .first()
            .unwrap()
            .get_human_download_url(file_name)
    }

    #[inline]
    pub fn get_download_file_name(&self, file_name: Option<&str>) -> Result<String, RefreshError> {
        self.sources
            .first()
            .unwrap()
            .get_download_file_name(file_name)
    }

    #[inline]
    pub fn get_download_url(&self, file_name: &str) -> String {
        self.sources.first().unwrap().get_download_url(file_name)
    }

    pub fn signed_by(&self) -> Option<&Signature> {
        self.sources.iter().find_map(|x| x.signed_by().as_ref())
    }

    pub fn url(&self) -> &str {
        self.sources.first().unwrap().url()
    }

    pub fn is_flat(&self) -> bool {
        self.sources.first().unwrap().is_flat()
    }

    pub fn trusted(&self) -> bool {
        self.sources.iter().any(|x| x.trusted())
    }

    pub fn file_name(&self) -> Option<&str> {
        self.release_file_name.get().map(|x| x.as_str())
    }

    #[allow(clippy::too_many_arguments)]
    pub async fn fetch(
        &self,
        client: &ClientWithMiddleware,
        index: usize,
        total: usize,
        tmp_dir: &Path,
        download_dir: &Path,
        tx: Sender<Event>,
    ) -> Result<(), RefreshError> {
        if self.is_mirror() {
            return self
                .fetch_mirror_release(client, index, total, tmp_dir, download_dir, tx)
                .await;
        }

        match self.from()? {
            OmaSourceEntryFrom::Http => {
                self.fetch_http_release(client, index, total, tmp_dir, download_dir, tx)
                    .await
            }
            OmaSourceEntryFrom::Local => {
                self.fetch_local_release(index, total, download_dir, tx)
                    .await
            }
        }
    }

    #[allow(clippy::too_many_arguments)]
    async fn fetch_http_release(
        &self,
        client: &ClientWithMiddleware,
        index: usize,
        total: usize,
        tmp_dir: &Path,
        download_dir: &Path,
        tx: Sender<Event>,
    ) -> Result<(), RefreshError> {
        let msg = self.get_human_download_message(None)?;

        let _ = tx
            .send_async(Event::DownloadEvent(oma_fetch::Event::NewProgressSpinner {
                index,
                total,
                msg,
            }))
            .await;

        let mut url = self.get_download_url("InRelease");
        let mut is_release = false;

        let resp = send_request_with_url_and_method(&url, client, Method::GET).await;
        let _ = tx
            .send_async(Event::DownloadEvent(oma_fetch::Event::ProgressDone(index)))
            .await;

        let resp = match resp {
            Ok(resp) => resp,
            Err(e) if e.status().is_some_and(|e| e == StatusCode::NOT_FOUND) => {
                url = self.get_download_url("Release");
                let resp = send_request_with_url_and_method(&url, client, Method::GET).await;

                if resp.is_err() && self.is_flat() {
                    // Flat repo no release
                    return Ok(());
                }

                is_release = true;

                resp.map_err(|e| SingleDownloadError::ReqwestMiddlewareError { source: e })
                    .map_err(|e| RefreshError::DownloadFailed(Some(e)))?
            }
            Err(e) => {
                return Err(RefreshError::DownloadFailed(Some(
                    SingleDownloadError::ReqwestMiddlewareError { source: e },
                )));
            }
        };

        let file_name = if is_release {
            self.get_download_file_name(Some("Release"))?
        } else {
            self.get_download_file_name(Some("InRelease"))?
        };

        self.download_file(
            &file_name,
            resp,
            index,
            total,
            tmp_dir,
            download_dir,
            tx.clone(),
        )
        .await
        .map_err(|e| RefreshError::DownloadFailed(Some(e)))?;

        self.set_release_file_name(file_name);

        if is_release && !self.trusted() {
            let url = self.get_download_url("Release.gpg");

            let resp = send_request_with_url_and_method(&url, client, Method::GET)
                .await
                .and_then(|resp| resp.error_for_status().map_err(|e| e.into()))
                .map_err(|e| SingleDownloadError::ReqwestMiddlewareError { source: e })
                .map_err(|e| RefreshError::DownloadFailed(Some(e)))?;

            let file_name = self.get_download_file_name(Some("Release.gpg"))?;

            self.download_file(
                &file_name,
                resp,
                index,
                total,
                tmp_dir,
                download_dir,
                tx.clone(),
            )
            .await
            .map_err(|e| RefreshError::DownloadFailed(Some(e)))?;
        }

        Ok(())
    }

    /// Fetch `InRelease`/`Release` for a mirror source, trying each resolved
    /// mirror in priority order. `InRelease` is tried on every mirror first;
    /// then `Release` and `Release.gpg` are fetched from the same mirror.
    #[allow(clippy::too_many_arguments)]
    async fn fetch_mirror_release(
        &self,
        client: &ClientWithMiddleware,
        index: usize,
        total: usize,
        tmp_dir: &Path,
        download_dir: &Path,
        tx: Sender<Event>,
    ) -> Result<(), RefreshError> {
        let list = self
            .mirror_list()
            .ok_or_else(|| RefreshError::MirrorListNotLoaded(self.url().to_string()))?;

        let source_uri = self.url();
        let suite = self.sources.first().unwrap().suite();
        let target = MirrorTarget {
            release: (!self.is_flat()).then_some(suite),
            kind: TargetKind::Index,
            ..Default::default()
        };

        let inrelease_name = self.get_download_file_name(Some("InRelease"))?;
        let inrelease_url = self.get_download_url("InRelease");
        let inrelease_candidates = list.resolve(source_uri, &inrelease_url, &target)?;

        let release_name = self.get_download_file_name(Some("Release"))?;
        let gpg_name = self.get_download_file_name(Some("Release.gpg"))?;
        let release_url = self.get_download_url("Release");
        let gpg_url = self.get_download_url("Release.gpg");

        let release_candidates = list.resolve(source_uri, &release_url, &target)?;
        let gpg_candidates = list.resolve(source_uri, &gpg_url, &target)?;
        let trusted = self.trusted();

        let msg = self.get_human_download_message(None)?;

        let _ = tx
            .send_async(Event::DownloadEvent(oma_fetch::Event::NewProgressSpinner {
                index,
                total,
                msg,
            }))
            .await;

        let mut spinner_done = false;

        for candidate in inrelease_candidates {
            if self
                .try_mirror_file(
                    client,
                    &candidate,
                    &inrelease_name,
                    index,
                    total,
                    tmp_dir,
                    download_dir,
                    &tx,
                    &mut spinner_done,
                )
                .await?
            {
                self.set_release_file_name(inrelease_name);
                return Ok(());
            }
        }

        if !spinner_done {
            let _ = tx
                .send_async(Event::DownloadEvent(oma_fetch::Event::ProgressDone(index)))
                .await;
            spinner_done = true;
        }

        let mut release_found = false;

        for (release_candidate, gpg_candidate) in release_candidates.into_iter().zip(gpg_candidates)
        {
            if !self
                .try_mirror_file(
                    client,
                    &release_candidate,
                    &release_name,
                    index,
                    total,
                    tmp_dir,
                    download_dir,
                    &tx,
                    &mut spinner_done,
                )
                .await?
            {
                continue;
            }

            release_found = true;

            if trusted {
                self.set_release_file_name(release_name);
                return Ok(());
            }

            if self
                .try_mirror_file(
                    client,
                    &gpg_candidate,
                    &gpg_name,
                    index,
                    total,
                    tmp_dir,
                    download_dir,
                    &tx,
                    &mut spinner_done,
                )
                .await?
            {
                self.set_release_file_name(release_name);
                return Ok(());
            }

            // The pair is incomplete; do not leave a Release behind from a
            // mirror whose signature could not be fetched.
            let _ = fs::remove_file(download_dir.join(&release_name)).await;
        }

        if release_found {
            // A Release existed but no mirror could provide its signature.
            // Do not fall back to an unsigned flat repository.
            return Err(RefreshError::ReleaseSignatureMissing(
                self.url().to_string(),
            ));
        }

        if self.is_flat() {
            // Flat repo without a release file.
            return Ok(());
        }

        Err(RefreshError::NoInReleaseFile(self.url().to_string()))
    }

    /// Try one concrete mirror URL for a release metadata file. Returns
    /// `false` when the mirror cannot serve it, so the caller can try the
    /// next mirror. Progress bars opened by a failed body transfer are closed
    /// before returning.
    #[allow(clippy::too_many_arguments)]
    async fn try_mirror_file(
        &self,
        client: &ClientWithMiddleware,
        candidate: &MirrorCandidate,
        file_name: &str,
        index: usize,
        total: usize,
        tmp_dir: &Path,
        download_dir: &Path,
        tx: &Sender<Event>,
        spinner_done: &mut bool,
    ) -> Result<bool, RefreshError> {
        match candidate.transport {
            MirrorTransport::Http => {
                let resp =
                    match send_request_with_url_and_method(&candidate.url, client, Method::GET)
                        .await
                    {
                        Ok(resp) => resp,
                        Err(e) => {
                            debug!("Mirror {} cannot serve {file_name}: {e}", candidate.url);
                            return Ok(false);
                        }
                    };

                if !*spinner_done {
                    let _ = tx
                        .send_async(Event::DownloadEvent(oma_fetch::Event::ProgressDone(index)))
                        .await;
                    *spinner_done = true;
                }

                match self
                    .download_file(
                        file_name,
                        resp,
                        index,
                        total,
                        tmp_dir,
                        download_dir,
                        tx.clone(),
                    )
                    .await
                {
                    Ok(()) => Ok(true),
                    Err(SingleDownloadError::ReqwestMiddlewareError { source }) => {
                        debug!("Mirror {} dropped {file_name}: {source}", candidate.url);
                        let _ = tx
                            .send_async(Event::DownloadEvent(oma_fetch::Event::ProgressDone(index)))
                            .await;
                        let _ = fs::remove_file(tmp_dir.join(file_name)).await;
                        Ok(false)
                    }
                    Err(e) => Err(RefreshError::DownloadFailed(Some(e))),
                }
            }
            MirrorTransport::File => {
                let path = Path::new(
                    candidate
                        .url
                        .strip_prefix("file:")
                        .unwrap_or(candidate.url.as_str()),
                );

                if !path.exists() {
                    debug!("Mirror {} does not have {file_name}", candidate.url);
                    return Ok(false);
                }

                if !*spinner_done {
                    let _ = tx
                        .send_async(Event::DownloadEvent(oma_fetch::Event::ProgressDone(index)))
                        .await;
                    *spinner_done = true;
                }

                let dst = download_dir.join(file_name);

                if dst.exists() || dst.is_symlink() {
                    fs::remove_file(&dst)
                        .await
                        .map_err(|e| RefreshError::OperateFile(dst.clone(), e))?;
                }

                fs::symlink(path, &dst)
                    .await
                    .map_err(|e| RefreshError::OperateFile(dst.clone(), e))?;

                Ok(true)
            }
        }
    }

    #[allow(clippy::too_many_arguments)]
    async fn download_file(
        &self,
        file_name: &str,
        mut resp: Response,
        index: usize,
        total: usize,
        tmp_dir: &Path,
        download_dir: &Path,
        tx: Sender<Event>,
    ) -> std::result::Result<(), SingleDownloadError> {
        let total_size = content_length(&resp);

        let _ = tx
            .send_async(Event::DownloadEvent(oma_fetch::Event::NewProgressBar {
                index,
                total,
                msg: self.get_human_download_message(Some(file_name)).unwrap(),
                size: total_size,
            }))
            .await;

        if !tmp_dir.is_dir() {
            tokio::fs::create_dir_all(tmp_dir)
                .await
                .map_err(|e| SingleDownloadError::Create { source: e })?;
        }

        let tmp = tmp_dir.join(file_name);

        let mut f = File::create(&tmp)
            .await
            .map_err(|e| SingleDownloadError::Create { source: e })?;

        while let Some(chunk) = resp
            .chunk()
            .await
            .map_err(|e| SingleDownloadError::ReqwestMiddlewareError { source: e.into() })?
        {
            let _ = tx
                .send_async(Event::DownloadEvent(oma_fetch::Event::ProgressInc {
                    index,
                    size: chunk.len() as u64,
                }))
                .await;

            f.write_all(&chunk)
                .await
                .map_err(|e| SingleDownloadError::Write { source: e })?;
        }

        f.shutdown()
            .await
            .map_err(|e| SingleDownloadError::Flush { source: e })?;

        debug!(
            "Rename release metadata from {} to {}",
            tmp.display(),
            download_dir.display()
        );
        tokio::fs::rename(&tmp, &download_dir.join(file_name))
            .await
            .map_err(|e| SingleDownloadError::Write { source: e })?;

        let _ = tx
            .send_async(Event::DownloadEvent(oma_fetch::Event::ProgressDone(index)))
            .await;

        Ok(())
    }

    async fn fetch_local_release(
        &self,
        index: usize,
        total: usize,
        download_dir: &Path,
        tx: Sender<Event>,
    ) -> Result<(), RefreshError> {
        let dist_path_with_protocol = self.dist_path();
        let dist_path = dist_path_with_protocol
            .strip_prefix("file:")
            .unwrap_or(dist_path_with_protocol);
        let dist_path = Path::new(dist_path);

        let mut name = None;

        let msg = self.get_human_download_message(None)?;

        let _ = tx
            .send_async(Event::DownloadEvent(oma_fetch::Event::NewProgressSpinner {
                index,
                total,
                msg,
            }))
            .await;

        let mut is_release = false;

        for (index, entry) in ["InRelease", "Release"].iter().enumerate() {
            let p = dist_path.join(entry);
            let file_name = self.get_download_file_name(Some(entry))?;
            let dst = download_dir.join(&file_name);

            if p.exists() {
                if dst.exists() || dst.is_symlink() {
                    debug!("get_release_file: Removing {} ...", dst.display());
                    fs::remove_file(&dst)
                        .await
                        .map_err(|e| RefreshError::OperateFile(dst.clone(), e))?;
                }

                debug!("get_release_file: Symlinking {} ...", dst.display());
                fs::symlink(p, &dst)
                    .await
                    .map_err(|e| RefreshError::OperateFile(dst.clone(), e))?;

                if index == 1 {
                    is_release = true;
                }

                name = Some(file_name);
                break;
            }
        }

        if name.is_none() && self.is_flat() {
            // Flat repo no release
            return Ok(());
        }

        if is_release {
            let p = dist_path.join("Release.gpg");
            let file_name = self.get_download_file_name(Some("Release.gpg"))?;
            let dst = download_dir.join(&file_name);

            if p.exists() {
                if dst.exists() || dst.is_symlink() {
                    fs::remove_file(&dst)
                        .await
                        .map_err(|e| RefreshError::OperateFile(dst.clone(), e))?;
                }

                fs::symlink(p, download_dir.join(file_name))
                    .await
                    .map_err(|e| RefreshError::OperateFile(dst.clone(), e))?;
            }
        }

        let _ = tx
            .send_async(Event::DownloadEvent(oma_fetch::Event::ProgressDone(index)))
            .await;

        let name = name.ok_or_else(|| RefreshError::NoInReleaseFile(self.url().to_string()))?;
        self.set_release_file_name(name);

        Ok(())
    }
}

impl MirrorSources {
    pub fn from_sourcelist(sourcelist: &[OmaSourceEntry]) -> Result<Self, RefreshError> {
        let mut map: HashMap<String, Vec<OmaSourceEntry>> =
            HashMap::with_hasher(ahash::RandomState::new());

        if sourcelist.is_empty() {
            return Err(RefreshError::SourceListsEmpty);
        }

        for source in sourcelist {
            map.entry(source.get_download_file_name(None)?)
                .or_default()
                .push(source.clone());
        }

        let mut res = vec![];

        for (_, v) in map {
            res.push(MirrorSource {
                sources: v,
                release_file_name: OnceCell::new(),
                mirror_list: OnceCell::new(),
            });
        }

        Ok(Self(res))
    }

    pub async fn fetch_all_release(
        &mut self,
        client: ClientWithMiddleware,

        download_dir: Arc<Path>,
        threads: usize,
        sender: Sender<Event>,
    ) -> Vec<Result<(), RefreshError>> {
        let total_len = self.0.len();
        let sources = std::mem::take(&mut self.0);

        let mut set = JoinSet::new();
        let mut source_locks = AHashMap::new();
        let tmp_dir = Arc::new(download_dir.join("partial"));

        for (index, m) in sources.into_iter().enumerate() {
            let client = client.clone();
            let tmp_dir = tmp_dir.clone();
            let sender = sender.clone();

            let source_key = if let Ok(url) = Url::parse(m.dist_path()) {
                format!("{}://{}", url.scheme(), url.host_str().unwrap_or("unknown"))
            } else {
                m.dist_path().to_string()
            };

            let source_sem = source_locks
                .entry(source_key)
                .or_insert_with(|| Arc::new(Semaphore::new(threads)))
                .clone();

            let download_dir = download_dir.clone();
            set.spawn(async move {
                let _permit = match source_sem.acquire_owned().await {
                    Ok(p) => Some(p),
                    Err(_) => return (m, Err(RefreshError::DownloadFailed(None))),
                };

                let res = m
                    .fetch(&client, index, total_len, &tmp_dir, &download_dir, sender)
                    .await;

                (m, res)
            });
        }

        let mut results = Vec::with_capacity(total_len);

        while let Some(task_res) = set.join_next().await {
            match task_res {
                Ok((m, res)) => {
                    self.0.push(m);
                    results.push(res);
                }
                Err(_) => {
                    results.push(Err(RefreshError::DownloadFailed(None)));
                }
            }
        }

        results
    }
}

#[test]
fn test_ose() {
    use oma_utils::dpkg::dpkg_arch;
    // Flat repository tests.

    // deb file:///debs/ /
    let entry = SourceEntry {
        enabled: true,
        source: false,
        options: vec![],
        url: "file:///debs/".to_string(),
        suite: "/".to_string(),
        components: vec![],
        is_deb822: false,
        archs: None,
        signed_by: None,
        trusted: false,
    };

    let arch = dpkg_arch("/").unwrap();
    let ose = OmaSourceEntry::new(entry, arch.into());
    assert_eq!(ose.url(), "file:///debs/");
    assert_eq!(ose.dist_path(), "file:///debs/");

    // deb file:///debs/ ./
    let entry = SourceEntry {
        enabled: true,
        source: false,
        options: vec![],
        url: "file:///debs/".to_string(),
        suite: "./".to_string(),
        components: vec![],
        is_deb822: false,
        archs: None,
        signed_by: None,
        trusted: false,
    };

    let arch = dpkg_arch("/").unwrap();
    let ose = OmaSourceEntry::new(entry, arch.into());
    assert_eq!(ose.url(), "file:///debs/");
    assert_eq!(ose.dist_path(), "file:///debs/./");

    // deb file:/debs/ /
    let entry = SourceEntry {
        enabled: true,
        source: false,
        options: vec![],
        url: "file:/debs/".to_string(),
        suite: "/".to_string(),
        components: vec![],
        is_deb822: false,
        archs: None,
        signed_by: None,
        trusted: false,
    };

    let arch = dpkg_arch("/").unwrap();
    let ose = OmaSourceEntry::new(entry, arch.into());
    assert_eq!(ose.url(), "file:/debs/");
    assert_eq!(ose.dist_path(), "file:/debs/");

    // deb file:/debs /
    //
    // APT will append implicitly a / at the end of the URL.
    let entry = SourceEntry {
        enabled: true,
        source: false,
        options: vec![],
        url: "file:/debs".to_string(),
        suite: "/".to_string(),
        components: vec![],
        is_deb822: false,
        archs: None,
        signed_by: None,
        trusted: false,
    };

    let arch = dpkg_arch("/").unwrap();
    let ose = OmaSourceEntry::new(entry, arch.into());
    assert_eq!(ose.url(), "file:/debs");
    assert_eq!(ose.dist_path(), "file:/debs/");

    // deb file:/debs/ ./././
    let entry = SourceEntry {
        enabled: true,
        source: false,
        options: vec![],
        url: "file:/debs/".to_string(),
        suite: "./././".to_string(),
        components: vec![],
        is_deb822: false,
        archs: None,
        signed_by: None,
        trusted: false,
    };

    let arch = dpkg_arch("/").unwrap();
    let ose = OmaSourceEntry::new(entry, arch.into());
    assert_eq!(ose.url(), "file:/debs/");
    assert_eq!(ose.dist_path(), "file:/debs/./././");

    // deb file:/debs/ .//
    //
    // APT will throw a warning but carry on with the suite name:
    //
    // W: Conflicting distribution: file:/debs .// Release (expected .// but got )
    let entry = SourceEntry {
        enabled: true,
        source: false,
        options: vec![],
        url: "file:/debs/".to_string(),
        suite: ".//".to_string(),
        components: vec![],
        is_deb822: false,
        archs: None,
        signed_by: None,
        trusted: false,
    };

    let arch = dpkg_arch("/").unwrap();
    let ose = OmaSourceEntry::new(entry, arch.into());
    assert_eq!(ose.url(), "file:/debs/");
    assert_eq!(ose.dist_path(), "file:/debs/.//");

    // deb file:/debs/ //
    //
    // APT will throw a warning but carry on with the suite name:
    //
    // W: Conflicting distribution: file:/debs // Release (expected // but got )
    let entry = SourceEntry {
        enabled: true,
        source: false,
        options: vec![],
        url: "file:/debs/".to_string(),
        suite: "//".to_string(),
        components: vec![],
        is_deb822: false,
        archs: None,
        signed_by: None,
        trusted: false,
    };

    let arch = dpkg_arch("/").unwrap();
    let ose = OmaSourceEntry::new(entry, arch.into());
    assert_eq!(ose.url(), "file:/debs/");
    assert_eq!(ose.dist_path(), "file:/debs///");

    // deb file:/./debs/ ./
    let entry = SourceEntry {
        enabled: true,
        source: false,
        options: vec![],
        url: "file:/./debs/".to_string(),
        suite: "./".to_string(),
        components: vec![],
        is_deb822: false,
        archs: None,
        signed_by: None,
        trusted: false,
    };

    let arch = dpkg_arch("/").unwrap();
    let ose = OmaSourceEntry::new(entry, arch.into());
    assert_eq!(ose.url(), "file:/./debs/");
    assert_eq!(ose.dist_path(), "file:/./debs/./");

    // deb file:/usr/../debs/ ./
    let entry = SourceEntry {
        enabled: true,
        source: false,
        options: vec![],
        url: "file:/usr/../debs/".to_string(),
        suite: "./".to_string(),
        components: vec![],
        is_deb822: false,
        archs: None,
        signed_by: None,
        trusted: false,
    };

    let arch = dpkg_arch("/").unwrap();
    let ose = OmaSourceEntry::new(entry, arch.into());
    assert_eq!(ose.url(), "file:/usr/../debs/");
    assert_eq!(ose.dist_path(), "file:/usr/../debs/./");
}

// Encode + as %252b: a real AOSC mirror path contains a '+' (e.g.
// `x264-0+git20240305`). apt percent-encodes the suite with
// `pkgAcquire::URIEncode` (an S3-bug workaround, LP#1003633/LP#1086997),
// then `URItoFileName` re-encodes the `%` — so apt's list filename has
// `%252b` (see commit d9287e33, "apt will encode '+' twice"). oma must
// produce the same filename, or the downloaded list files won't be found.
#[test]
fn test_url_encode_plus() {
    let entry = SourceEntry {
        enabled: true,
        source: false,
        options: vec![],
        url: "https://repo.aosc.io/debs/".to_string(),
        suite: "x264-0+git20240305".to_string(),
        components: vec!["main".to_string()],
        is_deb822: false,
        archs: None,
        signed_by: None,
        trusted: false,
    };

    let arch = oma_utils::dpkg::dpkg_arch("/").unwrap();
    let ose = OmaSourceEntry::new(entry, arch.into());
    let file_name = ose.get_download_file_name(Some("InRelease")).unwrap();

    assert_eq!(
        file_name,
        "repo.aosc.io_debs_dists_x264-0%252bgit20240305_InRelease"
    );
}

#[test]
fn test_dot() {
    let entry = SourceEntry {
        enabled: true,
        source: false,
        options: vec![],
        url: "https://ci.deepin.com/repo/obs/deepin:/CI:/TestingIntegration:/test-integration-pr-1537/testing".to_string(),
        suite: "./".to_string(),
        components: vec![],
        is_deb822: false,
        archs: None,
        signed_by: None,
        trusted: false,
    };

    let arch = oma_utils::dpkg::dpkg_arch("/").unwrap();
    let ose = OmaSourceEntry::new(entry, arch.into());
    let file_name = ose.get_download_file_name(Some("Packages")).unwrap();

    assert_eq!(
        file_name,
        "ci.deepin.com_repo_obs_deepin:_CI:_TestingIntegration:_test-integration-pr-1537_testing_._Packages"
    );
}

// Encode _ as %5f
#[test]
fn test_encode_underline() {
    let entry = SourceEntry {
        enabled: true,
        source: false,
        options: vec![],
        url: "https://repo.aosc.io/debs/".to_string(),
        suite: "xorg-server-21.1.13-hyperv_drm-fix".to_string(),
        components: vec!["main".to_string()],
        is_deb822: false,
        archs: None,
        signed_by: None,
        trusted: false,
    };

    let arch = oma_utils::dpkg::dpkg_arch("/").unwrap();
    let ose = OmaSourceEntry::new(entry, arch.into());

    let file_name = ose.get_download_file_name(Some("InRelease")).unwrap();

    assert_eq!(
        file_name,
        "repo.aosc.io_debs_dists_xorg-server-21.1.13-hyperv%5fdrm-fix_InRelease"
    );
}

// file:/// should be transliterated as file:/
#[test]
fn test_file_protocol_translate() {
    let s1 = "file:/debs";
    let s2 = "file:///debs";
    let res1 = url_to_list_filename(s1).unwrap();
    let res2 = url_to_list_filename(s2).unwrap();
    assert_eq!(res1, "_debs");
    assert_eq!(res1, res2);
}

// Dots (.) in flat repo URLs should be preserved in resolved database name.
#[test]
fn test_flat_repo_file_name_1() {
    let entry = SourceEntry {
        enabled: true,
        source: false,
        options: vec![],
        url: "file:///././debs/".to_string(),
        suite: "./".to_string(),
        components: vec![],
        is_deb822: false,
        archs: None,
        signed_by: None,
        trusted: false,
    };

    let arch = oma_utils::dpkg::dpkg_arch("/").unwrap();
    let ose = OmaSourceEntry::new(entry, arch.into());

    let file_name = ose.get_download_file_name(Some("Packages")).unwrap();

    assert_eq!(file_name, "_._._debs_._Packages");
}

// Slash (/) in flat repo "suite" names should be transliterated as _.
#[test]
fn test_flat_repo_file_name_2() {
    let entry = SourceEntry {
        enabled: true,
        source: false,
        options: vec![],
        url: "file:///debs".to_string(),
        suite: "/".to_string(),
        components: vec![],
        is_deb822: false,
        archs: None,
        signed_by: None,
        trusted: false,
    };

    let arch = oma_utils::dpkg::dpkg_arch("/").unwrap();
    let ose = OmaSourceEntry::new(entry, arch.into());

    let file_name = ose.get_download_file_name(Some("Packages")).unwrap();

    assert_eq!(file_name, "_debs_Packages");
}

// Dots (.) in flat repo "suite" names should be preserved in resolved database name
#[test]
fn test_flat_repo_file_name_3() {
    let entry = SourceEntry {
        enabled: true,
        source: false,
        options: vec![],
        url: "file:///debs/".to_string(),
        suite: "./".to_string(),
        components: vec![],
        is_deb822: false,
        archs: None,
        signed_by: None,
        trusted: false,
    };

    let arch = oma_utils::dpkg::dpkg_arch("/").unwrap();
    let ose = OmaSourceEntry::new(entry, arch.into());

    let file_name = ose.get_download_file_name(Some("Packages")).unwrap();

    assert_eq!(file_name, "_debs_._Packages");
}

// Slashes in URL and in flat repo "suite" names should be preserved in original number
#[test]
fn test_flat_repo_file_name_4() {
    let entry = SourceEntry {
        enabled: true,
        source: false,
        options: vec![],
        url: "file:///debs///".to_string(),
        suite: "./".to_string(),
        components: vec![],
        is_deb822: false,
        archs: None,
        signed_by: None,
        trusted: false,
    };

    let arch = oma_utils::dpkg::dpkg_arch("/").unwrap();
    let arch: Arc<str> = Arc::from(arch.as_str());
    let ac = arch.clone();
    let ose = OmaSourceEntry::new(entry, ac);
    let res = ose.get_download_file_name(Some("Packages")).unwrap();
    assert_eq!(res, "_debs___._Packages");

    let entry = SourceEntry {
        enabled: true,
        source: false,
        options: vec![],
        url: "file:///debs///".to_string(),
        suite: ".///".to_string(),
        components: vec![],
        is_deb822: false,
        archs: None,
        signed_by: None,
        trusted: false,
    };

    let ose = OmaSourceEntry::new(entry, arch);
    let res = ose.get_download_file_name(Some("Packages")).unwrap();
    assert_eq!(res, "_debs___.___Packages");
}

#[test]
fn test_mirror_source_file_names() {
    let entry: SourceEntry = "deb [trusted=yes] mirror+file:/etc/apt/mirrors.list bookworm main"
        .parse()
        .unwrap();
    let ose = OmaSourceEntry::new(entry, Arc::from("amd64"));

    assert!(ose.is_mirror());
    assert_eq!(
        ose.get_download_file_name(None).unwrap(),
        "_etc_apt_mirrors.list_dists_bookworm"
    );
    assert_eq!(
        ose.get_download_file_name(Some("InRelease")).unwrap(),
        "_etc_apt_mirrors.list_dists_bookworm_InRelease"
    );

    let sources = MirrorSources::from_sourcelist(&[ose]).unwrap();
    assert_eq!(sources.0.len(), 1);
    assert!(sources.0[0].is_mirror());
}

#[tokio::test]
async fn test_mirror_release_from_local_mirror() {
    rustls::crypto::ring::default_provider()
        .install_default()
        .ok();

    let root = std::env::temp_dir().join(format!(
        "oma-refresh-mirror-release-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    let repo = root.join("repo");
    let dist = repo.join("dists/bookworm");
    std::fs::create_dir_all(&dist).unwrap();
    std::fs::write(dist.join("InRelease"), "Origin: test\n").unwrap();

    let list_path = root.join("mirrors.list");
    std::fs::write(&list_path, format!("file:{}\n", repo.display())).unwrap();

    let entry: SourceEntry = format!(
        "deb [trusted=yes] mirror+file:{} bookworm main",
        list_path.display()
    )
    .parse()
    .unwrap();
    let ose = OmaSourceEntry::new(entry, Arc::from("amd64"));
    let mut sources = MirrorSources::from_sourcelist(&[ose]).unwrap();
    let source = sources.0.pop().unwrap();

    let list = MirrorList::parse(
        source.url(),
        &std::fs::read_to_string(&list_path).unwrap(),
        false,
    )
    .unwrap();
    source.set_mirror_list(Arc::new(list));

    let download_dir = root.join("lists");
    std::fs::create_dir_all(&download_dir).unwrap();
    let tmp_dir = download_dir.join("partial");

    // A previous refresh may have left a symlink to a mirror that no longer
    // has the file; the new symlink must still be installable.
    let expected = source.get_download_file_name(Some("InRelease")).unwrap();
    let dst = download_dir.join(&expected);
    std::os::unix::fs::symlink("/nonexistent-oma-mirror", &dst).unwrap();

    let client: reqwest_middleware::ClientWithMiddleware = oma_fetch::reqwest::ClientBuilder::new()
        .build()
        .unwrap()
        .into();
    let (tx, _rx) = flume::unbounded();

    source
        .fetch(&client, 0, 1, &tmp_dir, &download_dir, tx)
        .await
        .unwrap();

    assert_eq!(source.file_name(), Some(expected.as_str()));
    assert!(dst.is_symlink());
    assert_eq!(std::fs::read_link(&dst).unwrap(), dist.join("InRelease"));

    std::fs::remove_dir_all(&root).ok();
}

#[tokio::test]
async fn test_flat_mirror_release_requires_signature() {
    rustls::crypto::ring::default_provider()
        .install_default()
        .ok();

    let root = std::env::temp_dir().join(format!(
        "oma-refresh-flat-mirror-sig-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    let repo = root.join("repo");
    std::fs::create_dir_all(&repo).unwrap();
    std::fs::write(repo.join("Release"), "Origin: test\n").unwrap();

    let list_path = root.join("mirrors.list");
    std::fs::write(&list_path, format!("file:{}\n", repo.display())).unwrap();

    let entry: SourceEntry = format!("deb mirror+file:{} /", list_path.display())
        .parse()
        .unwrap();
    let ose = OmaSourceEntry::new(entry, Arc::from("amd64"));
    let mut sources = MirrorSources::from_sourcelist(&[ose]).unwrap();
    let source = sources.0.pop().unwrap();

    let list = MirrorList::parse(
        source.url(),
        &std::fs::read_to_string(&list_path).unwrap(),
        false,
    )
    .unwrap();
    source.set_mirror_list(Arc::new(list));

    let download_dir = root.join("lists");
    std::fs::create_dir_all(&download_dir).unwrap();
    let tmp_dir = download_dir.join("partial");

    let client: reqwest_middleware::ClientWithMiddleware = oma_fetch::reqwest::ClientBuilder::new()
        .build()
        .unwrap()
        .into();
    let (tx, _rx) = flume::unbounded();

    let err = source
        .fetch(&client, 0, 1, &tmp_dir, &download_dir, tx)
        .await
        .unwrap_err();

    assert!(
        matches!(&err, RefreshError::ReleaseSignatureMissing(_)),
        "{err:?}"
    );

    std::fs::remove_dir_all(&root).ok();
}
