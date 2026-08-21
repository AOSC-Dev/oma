use std::{borrow::Cow, fmt::Debug, path::Path, sync::Arc};

use ahash::{AHashMap, HashMap};
use fancy_regex::Regex;
use flume::Sender;
use oma_apt_pkg::AptConfig;
use oma_apt_sources_lists::{Signature, SourceEntry};
use oma_fetch::{
    DownloadSource, DownloadSourceType, SingleDownloadError,
    mirror::{MirrorSourceType, ResolvedMirror},
    reqwest::{Method, Response, StatusCode},
    send_request_with_url_and_method,
};
use oma_utils::concat_url;
use once_cell::sync::OnceCell;
use reqwest_middleware::ClientWithMiddleware;
use spdlog::{debug, warn};
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
    /// Resolved mirrors for `mirror://` sources (in priority order), set
    /// during the pre-resolution step in `OmaRefresh`. `url()`/`dist_path()`
    /// and the resulting list-file names keep the original `mirror://` URI.
    mirrors: OnceCell<Vec<ResolvedMirror>>,
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
            mirrors: OnceCell::new(),
        }
    }

    pub fn from(&self) -> Result<&OmaSourceEntryFrom, RefreshError> {
        self.from.get_or_try_init(|| {
            if self.is_mirror() {
                // The primary mirror's transport decides how this source is
                // fetched. Mirrors are resolved during `OmaRefresh`'s
                // pre-resolution step, before any download begins.
                let primary = self
                    .mirrors()
                    .and_then(|m| m.first())
                    .ok_or_else(|| RefreshError::UnsupportedProtocol(self.url().to_string()))?;
                return Ok(match primary.source_type {
                    MirrorSourceType::Http => OmaSourceEntryFrom::Http,
                    MirrorSourceType::File => OmaSourceEntryFrom::Local,
                });
            }

            let url = Url::parse(self.url())
                .map_err(|_| RefreshError::InvalidUrl(self.url().to_string()))?;

            match url.scheme() {
                "file" => Ok(OmaSourceEntryFrom::Local),
                "http" | "https" => Ok(OmaSourceEntryFrom::Http),
                x => Err(RefreshError::UnsupportedProtocol(x.to_string())),
            }
        })
    }

    /// Whether this source uses the apt `mirror://` protocol.
    pub fn is_mirror(&self) -> bool {
        let url = self.url();
        url.starts_with("mirror:") || url.starts_with("mirror+")
    }

    /// Store the resolved mirrors (priority order). Must be called before any
    /// download; list-file naming keeps the original `mirror://` URI so files
    /// are stored under stable names. Idempotent: re-resolving a source is a
    /// no-op.
    pub fn set_mirrors(&self, mirrors: Vec<ResolvedMirror>) {
        let _ = self.mirrors.set(mirrors);
    }

    pub fn mirrors(&self) -> Option<&[ResolvedMirror]> {
        self.mirrors.get().map(Vec::as_slice)
    }

    /// Expand an original (mirror-based) full URL into concrete URLs — one per
    /// resolved mirror — each with its transport and the mirror's priority
    /// (lower is tried first). Non-mirror sources return the URL unchanged
    /// with a default priority so their transport preference applies.
    ///
    /// `local_as_symlink` is used for `file:` mirrors, mirroring how plain
    /// local sources map to `DownloadSourceType::Local`.
    pub fn expand_mirror_url(
        &self,
        original_url: &str,
        local_as_symlink: bool,
    ) -> Result<Vec<(String, DownloadSourceType, u64)>, RefreshError> {
        if !self.is_mirror() {
            let source_type = match self.from()? {
                OmaSourceEntryFrom::Http => DownloadSourceType::Http,
                OmaSourceEntryFrom::Local => DownloadSourceType::Local(local_as_symlink),
            };
            return Ok(vec![(original_url.to_string(), source_type, u64::MAX)]);
        }

        let mirrors = self
            .mirrors()
            .ok_or_else(|| RefreshError::UnsupportedProtocol(self.url().to_string()))?;

        let base = self.url().trim_end_matches('/');
        let suffix = original_url
            .strip_prefix(base)
            .ok_or_else(|| RefreshError::InvalidUrl(original_url.to_string()))?;

        Ok(mirrors
            .iter()
            .map(|m| {
                let source_type = match m.source_type {
                    MirrorSourceType::Http => DownloadSourceType::Http,
                    MirrorSourceType::File => DownloadSourceType::Local(local_as_symlink),
                };
                (format!("{}{suffix}", m.url), source_type, m.priority)
            })
            .collect())
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
        // For mirror sources display the primary mirror instead of the
        // `mirror://` URI, which `url::Url` cannot parse.
        let url = if self.is_mirror() {
            self.mirrors()
                .and_then(|m| m.first())
                .map(|m| Cow::Owned(m.url.clone()))
                .unwrap_or_else(|| Cow::Borrowed(self.url()))
        } else {
            Cow::Borrowed(self.url())
        };
        let url = Url::parse(&url).map_err(|_| RefreshError::InvalidUrl(url.to_string()))?;

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
}

impl MirrorSource {
    pub fn set_release_file_name(&self, file_name: String) {
        self.release_file_name
            .set(file_name)
            .expect("Release file name was init");
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

    pub fn is_mirror(&self) -> bool {
        self.sources.first().is_some_and(|x| x.is_mirror())
    }

    /// Candidate URLs for a metadata file — one per resolved mirror, with its
    /// transport. Non-mirror sources yield a single candidate.
    pub fn candidate_urls_for(
        &self,
        file_name: &str,
    ) -> Result<Vec<(String, OmaSourceEntryFrom)>, RefreshError> {
        let original = self.get_download_url(file_name);
        self.sources
            .first()
            .unwrap()
            .expand_mirror_url(&original, self.is_flat())
            .map(|v| {
                v.into_iter()
                    .map(|(url, source_type, _priority)| {
                        let from = match source_type {
                            DownloadSourceType::Http => OmaSourceEntryFrom::Http,
                            DownloadSourceType::Local(_) => OmaSourceEntryFrom::Local,
                        };
                        (url, from)
                    })
                    .collect()
            })
    }

    /// Expand a full original URL into `DownloadSource`s, one per mirror.
    /// `local_as_symlink` is applied to `file:` mirrors (and to plain local
    /// sources), matching `collect_download_task`'s existing logic.
    pub fn download_sources_for(
        &self,
        original_url: &str,
        local_as_symlink: bool,
    ) -> Result<Vec<DownloadSource>, RefreshError> {
        self.sources
            .first()
            .unwrap()
            .expand_mirror_url(original_url, local_as_symlink)
            .map(|v| {
                v.into_iter()
                    .map(|(url, source_type, priority)| DownloadSource {
                        url,
                        source_type,
                        priority,
                    })
                    .collect()
            })
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
                if dst.exists() {
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
                if dst.exists() {
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

    /// Fetch the release files for a `mirror://` source, trying each resolved
    /// mirror in order until one succeeds (apt's mirror method behavior).
    /// Supports both http(s) and `file:` mirrors within one list.
    ///
    /// `InRelease` embeds its own signature, so the first mirror that has it
    /// wins. Otherwise `Release` is fetched and — when the repo is untrusted —
    /// its detached `Release.gpg` is fetched from the *same* mirror: taking
    /// the signature from any mirror that has one could combine it with a
    /// `Release` from another, and an out-of-sync cross-mirror pair fails
    /// verification even though some mirror carries a valid pair.
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
        let msg = self.get_human_download_message(None)?;

        let _ = tx
            .send_async(Event::DownloadEvent(oma_fetch::Event::NewProgressSpinner {
                index,
                total,
                msg,
            }))
            .await;

        // Try `InRelease` first — it carries its own signature, so any
        // mirror that has it yields a complete, verifiable release file.
        let inrelease_name = self.get_download_file_name(Some("InRelease"))?;
        for (url, from) in self.candidate_urls_for("InRelease")? {
            if self
                .try_fetch_release_file(
                    &inrelease_name,
                    &url,
                    &from,
                    client,
                    index,
                    total,
                    tmp_dir,
                    download_dir,
                    tx.clone(),
                )
                .await?
            {
                self.set_release_file_name(inrelease_name);
                let _ = tx
                    .send_async(Event::DownloadEvent(oma_fetch::Event::ProgressDone(index)))
                    .await;
                return Ok(());
            }
        }

        // No mirror has `InRelease`. Fall back to `Release`, and when the
        // repo is untrusted fetch its detached `Release.gpg` from the same
        // mirror — try the pair mirror-by-mirror instead of taking `Release`
        // from one mirror and the signature from another.
        let release_name = self.get_download_file_name(Some("Release"))?;
        let gpg_name = self.get_download_file_name(Some("Release.gpg"))?;
        let release_candidates = self.candidate_urls_for("Release")?;
        // The same mirrors in the same order, so zip pairs each `Release`
        // candidate with the `Release.gpg` candidate of the same mirror.
        let gpg_candidates = self.candidate_urls_for("Release.gpg")?;
        let trusted = self.trusted();

        for ((release_url, release_from), (gpg_url, gpg_from)) in
            release_candidates.iter().zip(gpg_candidates.iter())
        {
            if !self
                .try_fetch_release_file(
                    &release_name,
                    release_url,
                    release_from,
                    client,
                    index,
                    total,
                    tmp_dir,
                    download_dir,
                    tx.clone(),
                )
                .await?
            {
                continue;
            }

            if trusted {
                self.set_release_file_name(release_name);
                let _ = tx
                    .send_async(Event::DownloadEvent(oma_fetch::Event::ProgressDone(index)))
                    .await;
                return Ok(());
            }

            // Untrusted: the detached signature must come from this mirror
            // too; otherwise the pair is incomplete, try the next mirror.
            if self
                .try_fetch_release_file(
                    &gpg_name,
                    gpg_url,
                    gpg_from,
                    client,
                    index,
                    total,
                    tmp_dir,
                    download_dir,
                    tx.clone(),
                )
                .await?
            {
                self.set_release_file_name(release_name);
                let _ = tx
                    .send_async(Event::DownloadEvent(oma_fetch::Event::ProgressDone(index)))
                    .await;
                return Ok(());
            }
        }

        // Nothing usable: no mirror served `InRelease`, or (untrusted) no
        // mirror had a consistent `Release` + `Release.gpg` pair.
        if self.is_flat() {
            // Flat repo without a release file.
            let _ = tx
                .send_async(Event::DownloadEvent(oma_fetch::Event::ProgressDone(index)))
                .await;
            return Ok(());
        }
        Err(RefreshError::NoInReleaseFile(self.url().to_string()))
    }

    /// Try to obtain `file_name` from one candidate mirror, writing it under
    /// `download_dir`. Returns whether the file was obtained; failures to
    /// reach a mirror return `false` so the caller can try the next one.
    #[allow(clippy::too_many_arguments)]
    async fn try_fetch_release_file(
        &self,
        file_name: &str,
        url: &str,
        from: &OmaSourceEntryFrom,
        client: &ClientWithMiddleware,
        index: usize,
        total: usize,
        tmp_dir: &Path,
        download_dir: &Path,
        tx: Sender<Event>,
    ) -> Result<bool, RefreshError> {
        let got = match from {
            OmaSourceEntryFrom::Http => {
                match send_request_with_url_and_method(url, client, Method::GET).await {
                    Ok(resp) => {
                        self.download_file(
                            file_name,
                            resp,
                            index,
                            total,
                            tmp_dir,
                            download_dir,
                            tx.clone(),
                        )
                        .await
                        .map_err(|e| RefreshError::DownloadFailed(Some(e)))?;
                        true
                    }
                    // Try the next mirror on any failure.
                    Err(_) => false,
                }
            }
            OmaSourceEntryFrom::Local => {
                let path = Path::new(url.strip_prefix("file:").unwrap_or(url));
                if path.exists() {
                    let dst = download_dir.join(file_name);
                    if dst.exists() {
                        fs::remove_file(&dst)
                            .await
                            .map_err(|e| RefreshError::OperateFile(dst.clone(), e))?;
                    }
                    fs::symlink(path, &dst)
                        .await
                        .map_err(|e| RefreshError::OperateFile(dst.clone(), e))?;
                    true
                } else {
                    false
                }
            }
        };
        Ok(got)
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
fn test_mirror_expansion() {
    // A mirror source keeps its `mirror://` URI for naming, and expands to
    // one concrete URL per resolved mirror for downloading.
    let entry: SourceEntry = "deb mirror+file:///etc/apt/mirrors.test bookworm main"
        .parse()
        .unwrap();
    let entry = OmaSourceEntry::new(entry, Arc::from("amd64"));
    assert!(entry.is_mirror());

    // Simulate the pre-resolution step.
    entry.set_mirrors(vec![
        ResolvedMirror {
            url: "http://m1.example.com/debian".into(),
            source_type: MirrorSourceType::Http,
            priority: 1,
        },
        ResolvedMirror {
            url: "http://m2.example.com/debian".into(),
            source_type: MirrorSourceType::Http,
            priority: 2,
        },
    ]);

    // `from()` follows the primary mirror's transport.
    assert_eq!(entry.from().unwrap(), &OmaSourceEntryFrom::Http);

    // Naming stays based on the original `mirror://` URI.
    let original = entry.get_download_url("InRelease");
    assert!(original.starts_with("mirror+file:"));

    // Expansion replaces the mirror base with each resolved mirror.
    let expanded = entry.expand_mirror_url(&original, false).unwrap();
    assert_eq!(expanded.len(), 2);
    assert_eq!(
        expanded[0].0,
        "http://m1.example.com/debian/dists/bookworm/InRelease"
    );
    assert_eq!(
        expanded[1].0,
        "http://m2.example.com/debian/dists/bookworm/InRelease"
    );
    assert_eq!(expanded[0].1, DownloadSourceType::Http);

    // A `file:` mirror maps to a local source.
    let entry_file = OmaSourceEntry::new(
        "deb mirror+file:///etc/apt/mirrors.test bookworm main"
            .parse()
            .unwrap(),
        Arc::from("amd64"),
    );
    entry_file.set_mirrors(vec![ResolvedMirror {
        url: "file:///repo".into(),
        source_type: MirrorSourceType::File,
        priority: 1,
    }]);
    let original = entry_file.get_download_url("InRelease");
    let expanded = entry_file.expand_mirror_url(&original, true).unwrap();
    assert_eq!(expanded[0].0, "file:///repo/dists/bookworm/InRelease");
    assert_eq!(expanded[0].1, DownloadSourceType::Local(true));

    // Non-mirror sources are returned unchanged.
    let plain = OmaSourceEntry::new(
        "deb http://example.com/debian bookworm main"
            .parse()
            .unwrap(),
        Arc::from("amd64"),
    );
    let original = plain.get_download_url("InRelease");
    let expanded = plain.expand_mirror_url(&original, false).unwrap();
    assert_eq!(expanded.len(), 1);
    assert_eq!(expanded[0].0, original);
    assert_eq!(expanded[0].1, DownloadSourceType::Http);
}

#[tokio::test]
async fn test_resolve_mirrors_local_file() {
    let dir = tempfile::tempdir().unwrap();
    let list = dir.path().join("mirrors.list");
    std::fs::write(
        &list,
        "http://m1.example.com/\tpriority:1\trelease:bookworm\nfile:///local/repo\tpriority:2\n",
    )
    .unwrap();

    let uri = format!("mirror+file://{}", list.display());
    let _ = rustls::crypto::ring::default_provider().install_default();
    let client = reqwest_middleware::ClientBuilder::new(oma_fetch::reqwest::Client::new()).build();

    let mirrors =
        oma_fetch::mirror::resolve_mirrors(&uri, &client, Some("amd64"), Some("bookworm"), false)
            .await
            .unwrap();

    assert_eq!(mirrors.len(), 2);
    assert_eq!(mirrors[0].url, "http://m1.example.com");
    assert_eq!(mirrors[0].source_type, MirrorSourceType::Http);
    assert_eq!(mirrors[1].url, "file:///local/repo");
    assert_eq!(mirrors[1].source_type, MirrorSourceType::File);
}

#[tokio::test]
async fn test_fetch_mirror_release_atomic_pair() {
    // Mirror A has `Release` but no signature; mirror B has a consistent
    // `Release` + `Release.gpg` pair. The release fetch must fall through to
    // mirror B's *pair* rather than combining A's `Release` with B's
    // signature — a cross-mirror pair that would fail verification.
    let dir = tempfile::tempdir().unwrap();
    let mirror_a = dir.path().join("a");
    let mirror_b = dir.path().join("b");
    let download_dir = tempfile::tempdir().unwrap();
    let tmp_dir = tempfile::tempdir().unwrap();

    let entry: SourceEntry = "deb mirror+file:///etc/apt/mirrors.test bookworm main"
        .parse()
        .unwrap();
    let ose = OmaSourceEntry::new(entry, Arc::from("amd64"));
    ose.set_mirrors(vec![
        ResolvedMirror {
            url: format!("file://{}", mirror_a.display()),
            source_type: MirrorSourceType::File,
            priority: 1,
        },
        ResolvedMirror {
            url: format!("file://{}", mirror_b.display()),
            source_type: MirrorSourceType::File,
            priority: 2,
        },
    ]);

    let release_name = ose.get_download_file_name(Some("Release")).unwrap();
    let gpg_name = ose.get_download_file_name(Some("Release.gpg")).unwrap();

    // Expand the concrete paths so the fixtures land exactly where the
    // fetch will look for them.
    let release_urls = ose
        .expand_mirror_url(&ose.get_download_url("Release"), false)
        .unwrap();
    let gpg_urls = ose
        .expand_mirror_url(&ose.get_download_url("Release.gpg"), false)
        .unwrap();
    assert_eq!(release_urls.len(), 2);
    assert_eq!(gpg_urls.len(), 2);

    // Mirror A: only `Release` — out of sync, no signature.
    let a_release = Path::new(release_urls[0].0.strip_prefix("file:").unwrap());
    std::fs::create_dir_all(a_release.parent().unwrap()).unwrap();
    std::fs::write(a_release, b"release-a").unwrap();

    // Mirror B: a complete `Release` + `Release.gpg` pair.
    let b_release = Path::new(release_urls[1].0.strip_prefix("file:").unwrap());
    std::fs::create_dir_all(b_release.parent().unwrap()).unwrap();
    std::fs::write(b_release, b"release-b").unwrap();
    let b_gpg = Path::new(gpg_urls[1].0.strip_prefix("file:").unwrap());
    std::fs::create_dir_all(b_gpg.parent().unwrap()).unwrap();
    std::fs::write(b_gpg, b"gpg-b").unwrap();

    let ms = MirrorSource {
        sources: vec![ose],
        release_file_name: OnceCell::new(),
    };

    let _ = rustls::crypto::ring::default_provider().install_default();
    let client = reqwest_middleware::ClientBuilder::new(oma_fetch::reqwest::Client::new()).build();
    let (tx, _rx) = flume::unbounded();

    ms.fetch_mirror_release(
        &client,
        0,
        1,
        tmp_dir.path(),
        download_dir.path(),
        tx,
    )
    .await
    .unwrap();

    // The pair must come from the same (second) mirror: the downloaded
    // `Release` is B's, not A's, alongside B's signature.
    assert_eq!(
        ms.release_file_name.get().map(String::as_str),
        Some(release_name.as_str())
    );
    assert_eq!(
        std::fs::read_to_string(download_dir.path().join(&release_name)).unwrap(),
        "release-b"
    );
    assert_eq!(
        std::fs::read_to_string(download_dir.path().join(&gpg_name)).unwrap(),
        "gpg-b"
    );
}
