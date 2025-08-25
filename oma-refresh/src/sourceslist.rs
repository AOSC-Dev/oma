use std::{
    fmt::Debug,
    fs::Permissions,
    os::unix::fs::PermissionsExt,
    path::{Path, PathBuf},
};

use ahash::HashMap;
use apt_auth_config::{AuthConfig, Authenticator};
use fancy_regex::Regex;
use futures::StreamExt;
use oma_apt_sources_lists::{
    Signature, SourceEntry, SourceLine, SourceListType, SourcesList, SourcesListError,
};
use oma_fetch::{
    SingleDownloadError, build_request_with_basic_auth,
    reqwest::{Client, Method, Response, StatusCode},
    send_request,
};
use once_cell::sync::OnceCell;
use tokio::{
    fs::{self, File},
    io::AsyncWriteExt,
};
use tracing::debug;
use url::Url;

use crate::{
    db::{Event, RefreshError, content_length},
    util::DatabaseFilenameReplacer,
};

#[derive(Clone)]
pub struct OmaSourceEntry<'a> {
    source: SourceEntry,
    arch: &'a str,
    url: OnceCell<String>,
    suite: OnceCell<String>,
    dist_path: OnceCell<String>,
    from: OnceCell<OmaSourceEntryFrom>,
}

impl Debug for OmaSourceEntry<'_> {
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

pub(crate) async fn scan_sources_lists_paths_from_sysroot(
    sysroot: impl AsRef<Path>,
) -> Result<Vec<PathBuf>, SourcesListError> {
    let mut paths = vec![];
    let default = sysroot.as_ref().join("etc/apt/sources.list");

    if default.exists() {
        paths.push(default);
    }

    if sysroot.as_ref().join("etc/apt/sources.list.d/").exists() {
        let mut dir = tokio::fs::read_dir(sysroot.as_ref().join("etc/apt/sources.list.d/")).await?;
        while let Some(entry) = dir.next_entry().await? {
            let path = entry.path();
            if !path.is_file() {
                continue;
            }

            paths.push(path);
        }
    }

    Ok(paths)
}

#[cfg(feature = "apt")]
pub fn ignores(config: &oma_apt::config::Config) -> Vec<Regex> {
    use tracing::warn;

    config.find_vector("Dir::Ignore-Files-Silently")
        .iter()
        .filter_map(|re| Regex::new(re)
            .inspect_err(|e|
                warn!("Failed to parse regex {} in ignore rule list (Dir::Ignore-Files-Silently): {}", re, e)).ok())
        .collect::<Vec<_>>()
}

pub async fn scan_sources_list_from_paths<'a>(
    paths: &[impl AsRef<Path>],
    arch: &'a str,
    ignores: &[Regex],
    cb: &'a impl AsyncFn(Event),
) -> Result<Vec<OmaSourceEntry<'a>>, SourcesListError> {
    let mut res = vec![];

    for p in paths {
        match SourcesList::new(p) {
            Ok(s) => match s.entries {
                SourceListType::SourceLine(source_list_line_style) => {
                    for i in source_list_line_style.0 {
                        if let SourceLine::Entry(entry) = i {
                            res.push(OmaSourceEntry::new(entry, arch));
                        }
                    }
                }
                SourceListType::Deb822(source_list_deb822) => {
                    for i in source_list_deb822.entries {
                        res.push(OmaSourceEntry::new(i, arch));
                    }
                }
            },
            Err(e) => match e {
                SourcesListError::UnknownFile { path } => {
                    let Some(file_name) = path.file_name() else {
                        cb(Event::SourceListFileNotSupport { path }).await;
                        continue;
                    };

                    if ignores
                        .iter()
                        .any(|re| re.is_match(&file_name.to_string_lossy()).unwrap_or(false))
                    {
                        debug!("File {:?} matches ignore list.", file_name);
                        continue;
                    }

                    cb(Event::SourceListFileNotSupport { path }).await;
                }
                e => return Err(e),
            },
        }
    }

    Ok(res)
}

#[derive(PartialEq, Eq, Debug, Copy, Clone)]
pub enum OmaSourceEntryFrom {
    Http,
    Local,
}

impl<'a> OmaSourceEntry<'a> {
    pub fn new(source: SourceEntry, arch: &'a str) -> Self {
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
            .get_or_init(|| self.source.url.replace("$(ARCH)", self.arch))
    }

    pub fn is_flat(&self) -> bool {
        self.components().is_empty()
    }

    pub fn suite(&self) -> &str {
        self.suite
            .get_or_init(|| self.source.suite.replace("$(ARCH)", self.arch))
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
                } else if url.ends_with('/') {
                    format!("{url}{suite}")
                } else {
                    format!("{url}/{suite}")
                }
            } else {
                self.source.dist_path()
            }
        })
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

#[derive(Debug)]
pub struct MirrorSources<'a>(pub Vec<MirrorSource<'a>>);

#[derive(Debug)]
pub struct MirrorSource<'a> {
    pub sources: Vec<&'a OmaSourceEntry<'a>>,
    release_file_name: OnceCell<String>,
    auth: Option<&'a Authenticator>,
}

impl MirrorSource<'_> {
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

    pub fn from(&self) -> Result<&OmaSourceEntryFrom, RefreshError> {
        self.sources.first().unwrap().from()
    }

    pub fn get_human_download_message(
        &self,
        file_name: Option<&str>,
    ) -> Result<String, RefreshError> {
        self.sources
            .first()
            .unwrap()
            .get_human_download_url(file_name)
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

    pub fn auth(&self) -> Option<&Authenticator> {
        self.auth
    }

    pub async fn fetch(
        &self,
        client: &Client,
        replacer: &DatabaseFilenameReplacer,
        index: usize,
        total: usize,
        download_dir: &Path,
        callback: &impl AsyncFn(Event),
    ) -> Result<(), RefreshError> {
        match self.from()? {
            OmaSourceEntryFrom::Http => {
                self.fetch_http_release(client, replacer, index, total, download_dir, callback)
                    .await
            }
            OmaSourceEntryFrom::Local => {
                self.fetch_local_release(replacer, index, total, download_dir, callback)
                    .await
            }
        }
    }

    async fn fetch_http_release(
        &self,
        client: &Client,
        replacer: &DatabaseFilenameReplacer,
        index: usize,
        total: usize,
        download_dir: &Path,
        callback: impl AsyncFn(Event),
    ) -> Result<(), RefreshError> {
        let dist_path = self.dist_path();

        let msg = self.get_human_download_message(None)?;

        callback(Event::DownloadEvent(oma_fetch::Event::NewProgressSpinner {
            index,
            msg: format!("({index}/{total}) {msg}"),
        }))
        .await;

        let mut url = format!("{dist_path}/InRelease");
        let mut is_release = false;

        let resp = match self.send_request(client, &url, Method::GET).await {
            Ok(resp) => resp,
            Err(e) if e.status().is_some_and(|e| e == StatusCode::NOT_FOUND) => {
                url = format!("{dist_path}/Release");
                let resp = self.send_request(client, &url, Method::GET).await;

                if resp.is_err() && self.is_flat() {
                    // Flat repo no release
                    callback(Event::DownloadEvent(oma_fetch::Event::ProgressDone(index))).await;
                    return Ok(());
                }

                is_release = true;

                callback(Event::DownloadEvent(oma_fetch::Event::ProgressDone(index))).await;

                resp.map_err(|e| SingleDownloadError::ReqwestError { source: e })
                    .map_err(|e| RefreshError::DownloadFailed(Some(e)))?
            }
            Err(e) => {
                return Err(RefreshError::DownloadFailed(Some(
                    SingleDownloadError::ReqwestError { source: e },
                )));
            }
        };

        callback(Event::DownloadEvent(oma_fetch::Event::ProgressDone(index))).await;

        let file_name = replacer.replace(&url)?;

        self.download_file(&file_name, resp, index, total, download_dir, &callback)
            .await
            .map_err(|e| RefreshError::DownloadFailed(Some(e)))?;

        self.set_release_file_name(file_name);

        if is_release && !self.trusted() {
            let url = format!("{}/{}", dist_path, "Release.gpg");

            let request = build_request_with_basic_auth(
                client,
                Method::GET,
                &self
                    .auth()
                    .map(|x| (x.login.to_string(), x.password.to_string())),
                &url,
            );

            let resp = request
                .send()
                .await
                .and_then(|resp| resp.error_for_status())
                .map_err(|e| SingleDownloadError::ReqwestError { source: e })
                .map_err(|e| RefreshError::DownloadFailed(Some(e)))?;

            let file_name = replacer.replace(&url)?;

            self.download_file(&file_name, resp, index, total, download_dir, &callback)
                .await
                .map_err(|e| RefreshError::DownloadFailed(Some(e)))?;
        }

        Ok(())
    }

    async fn send_request(
        &self,
        client: &Client,
        url: &str,
        method: Method,
    ) -> Result<Response, oma_fetch::reqwest::Error> {
        let request = build_request_with_basic_auth(
            client,
            method,
            &self
                .auth()
                .map(|x| (x.login.to_string(), x.password.to_string())),
            url,
        );

        let resp = send_request(url, request).await?;

        Ok(resp)
    }

    async fn download_file(
        &self,
        file_name: &str,
        mut resp: Response,
        index: usize,
        total: usize,
        download_dir: &Path,
        callback: &impl AsyncFn(Event),
    ) -> std::result::Result<(), SingleDownloadError> {
        let total_size = content_length(&resp);

        callback(Event::DownloadEvent(oma_fetch::Event::NewProgressBar {
            index,
            msg: format!(
                "({}/{}) {}",
                index,
                total,
                self.get_human_download_message(Some(file_name)).unwrap(),
            ),
            size: total_size,
        }))
        .await;

        let mut f = File::create(download_dir.join(file_name))
            .await
            .map_err(|e| SingleDownloadError::Create { source: e })?;

        f.set_permissions(Permissions::from_mode(0o644))
            .await
            .map_err(|e| SingleDownloadError::SetPermission { source: e })?;

        while let Some(chunk) = resp
            .chunk()
            .await
            .map_err(|e| SingleDownloadError::ReqwestError { source: e })?
        {
            callback(Event::DownloadEvent(oma_fetch::Event::ProgressInc {
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

        callback(Event::DownloadEvent(oma_fetch::Event::ProgressDone(index))).await;

        Ok(())
    }

    async fn fetch_local_release(
        &self,
        replacer: &DatabaseFilenameReplacer,
        index: usize,
        total: usize,
        download_dir: &Path,
        callback: &impl AsyncFn(Event),
    ) -> Result<(), RefreshError> {
        let dist_path_with_protocol = self.dist_path();
        let dist_path = dist_path_with_protocol
            .strip_prefix("file:")
            .unwrap_or(dist_path_with_protocol);
        let dist_path = Path::new(dist_path);

        let mut name = None;

        let msg = self.get_human_download_message(None)?;

        callback(Event::DownloadEvent(oma_fetch::Event::NewProgressSpinner {
            index,
            msg: format!("({index}/{total}) {msg}"),
        }))
        .await;

        let mut is_release = false;

        for (index, entry) in ["InRelease", "Release"].iter().enumerate() {
            let p = dist_path.join(entry);

            let dst = if dist_path_with_protocol.ends_with('/') {
                format!("{dist_path_with_protocol}{entry}")
            } else {
                format!("{dist_path_with_protocol}/{entry}")
            };

            let file_name = replacer.replace(&dst)?;

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
            let entry = "Release.gpg";

            let dst = if dist_path_with_protocol.ends_with('/') {
                format!("{dist_path_with_protocol}{entry}")
            } else {
                format!("{dist_path_with_protocol}/{entry}")
            };

            let file_name = replacer.replace(&dst)?;

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

        callback(Event::DownloadEvent(oma_fetch::Event::ProgressDone(index))).await;

        let name = name.ok_or_else(|| RefreshError::NoInReleaseFile(self.url().to_string()))?;
        self.set_release_file_name(name);

        Ok(())
    }
}

impl<'a> MirrorSources<'a> {
    pub fn from_sourcelist(
        sourcelist: &'a [OmaSourceEntry<'a>],
        replacer: &DatabaseFilenameReplacer,
        auth_config: Option<&'a AuthConfig>,
    ) -> Result<Self, RefreshError> {
        let mut map: HashMap<String, Vec<&OmaSourceEntry>> =
            HashMap::with_hasher(ahash::RandomState::new());

        if sourcelist.is_empty() {
            return Err(RefreshError::SourceListsEmpty);
        }

        for source in sourcelist {
            let dist_path = source.dist_path();
            let name = replacer.replace(dist_path)?;

            map.entry(name).or_default().push(source);
        }

        let mut res = vec![];

        for (_, v) in map {
            let url = v[0].url();
            let auth = auth_config.and_then(|auth| auth.find(url));

            res.push(MirrorSource {
                sources: v,
                release_file_name: OnceCell::new(),
                auth,
            });
        }

        Ok(Self(res))
    }

    pub async fn fetch_all_release(
        &self,
        client: &Client,
        replacer: &DatabaseFilenameReplacer,
        download_dir: &Path,
        threads: usize,
        callback: &impl AsyncFn(Event),
    ) -> Vec<Result<(), RefreshError>> {
        let tasks = self.0.iter().enumerate().map(|(index, m)| {
            m.fetch(
                client,
                replacer,
                index,
                self.0.len(),
                download_dir,
                &callback,
            )
        });

        futures::stream::iter(tasks)
            .buffer_unordered(threads)
            .collect::<Vec<_>>()
            .await
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
    let ose = OmaSourceEntry::new(entry, &arch);
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
    let ose = OmaSourceEntry::new(entry, &arch);
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
    let ose = OmaSourceEntry::new(entry, &arch);
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
    let ose = OmaSourceEntry::new(entry, &arch);
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
    let ose = OmaSourceEntry::new(entry, &arch);
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
    let ose = OmaSourceEntry::new(entry, &arch);
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
    let ose = OmaSourceEntry::new(entry, &arch);
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
    let ose = OmaSourceEntry::new(entry, &arch);
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
    let ose = OmaSourceEntry::new(entry, &arch);
    assert_eq!(ose.url(), "file:/usr/../debs/");
    assert_eq!(ose.dist_path(), "file:/usr/../debs/./");
}

#[test]
fn test_database_filename() {
    use crate::util::DatabaseFilenameReplacer;
    let replacer = DatabaseFilenameReplacer::new().unwrap();

    // Encode + as %252b.
    let s = "https://repo.aosc.io/debs/dists/x264-0+git20240305/InRelease";
    let res = replacer.replace(s).unwrap();
    assert_eq!(
        res,
        "repo.aosc.io_debs_dists_x264-0%252bgit20240305_InRelease"
    );

    // Encode : as %3A.
    let s = "https://ci.deepin.com/repo/obs/deepin%3A/CI%3A/TestingIntegration%3A/test-integration-pr-1537/testing/./Packages";
    let res = replacer.replace(s).unwrap();
    assert_eq!(
        res,
        "ci.deepin.com_repo_obs_deepin:_CI:_TestingIntegration:_test-integration-pr-1537_testing_._Packages"
    );

    // Encode _ as %5f
    let s = "https://repo.aosc.io/debs/dists/xorg-server-21.1.13-hyperv_drm-fix";
    let res = replacer.replace(s).unwrap();
    assert_eq!(
        res,
        "repo.aosc.io_debs_dists_xorg-server-21.1.13-hyperv%5fdrm-fix"
    );

    // file:/// should be transliterated as file:/.
    let s1 = "file:/debs";
    let s2 = "file:///debs";
    let res1 = replacer.replace(s1).unwrap();
    let res2 = replacer.replace(s2).unwrap();
    assert_eq!(res1, "_debs");
    assert_eq!(res1, res2);

    // Dots (.) in flat repo URLs should be preserved in resolved database name.
    let s = "file:///././debs/./Packages";
    let res = replacer.replace(s).unwrap();
    assert_eq!(res, "_._._debs_._Packages");

    // Slash (/) in flat repo "suite" names should be transliterated as _.
    let s = "file:///debs/Packages";
    let res = replacer.replace(s).unwrap();
    assert_eq!(res, "_debs_Packages");

    // Dots (.) in flat repo "suite" names should be preserved in resolved database name.
    let s = "file:///debs/./Packages";
    let res = replacer.replace(s).unwrap();
    assert_eq!(res, "_debs_._Packages");

    // Slashes in URL and in flat repo "suite" names should be preserved in original number (1).
    let s = "file:///debs///./Packages";
    let res = replacer.replace(s).unwrap();
    assert_eq!(res, "_debs___._Packages");

    // Slashes in URL and in flat repo "suite" names should be preserved in original number (2).
    let s = "file:///debs///.///Packages";
    let res = replacer.replace(s).unwrap();
    assert_eq!(res, "_debs___.___Packages");
}
