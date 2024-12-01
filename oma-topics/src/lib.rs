use std::{
    borrow::Cow,
    io,
    path::{Path, PathBuf},
};

use oma_mirror::MirrorManager;
use reqwest::Client;
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use tokio::fs;
use tracing::{debug, warn};
use url::Url;

pub type Result<T> = std::result::Result<T, OmaTopicsError>;

#[derive(Debug, thiserror::Error)]
pub enum OmaTopicsError {
    #[error("Failed to operate dir or flle {0}: {1}")]
    FailedToOperateDirOrFile(String, std::io::Error),
    #[error("Can not find topic: {0}")]
    CanNotFindTopic(String),
    #[error("Failed to enable topic: {0}")]
    FailedToDisableTopic(String),
    #[error(transparent)]
    ReqwestError(#[from] reqwest::Error),
    #[error("Failed to serialize data")]
    FailedSer,
    #[error("Failed to get path parent: {0:?}")]
    FailedGetParentPath(PathBuf),
    #[error("file is broken")]
    BrokenFile(String),
    #[error("Failed to Parse Url: {0}")]
    ParseUrl(url::ParseError),
    #[error("Unsupported url protocol from url: {0}")]
    UnsupportedProtocol(String),
    #[error("Failed to open file {0}: {1}")]
    OpenFile(String, io::Error),
    #[error("Failed to read file {0}: {1}")]
    ReadFile(String, serde_json::Error),
    #[error(transparent)]
    MirrorError(#[from] oma_mirror::MirrorError),
}

async fn enabled_mirror(rootfs: PathBuf) -> Result<Vec<Box<str>>> {
    let mm = MirrorManager::new(rootfs)?;
    let urls = mm
        .enabled_mirrors()
        .values()
        .map(|x| x.to_owned())
        .collect::<Vec<Box<str>>>();

    Ok(urls)
}

const TOPICS_JSON: &str = "manifest/topics.json";

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct Topic {
    pub name: String,
    pub description: Option<String>,
    date: u64,
    update_date: Option<u64>,
    #[serde(skip_serializing)]
    arch: Option<Vec<String>>,
    pub packages: Vec<String>,
    #[serde(skip_serializing)]
    pub draft: Option<bool>,
}

impl PartialEq for Topic {
    fn eq(&self, other: &Self) -> bool {
        self.name == other.name
    }
}

#[derive(Debug)]
pub struct TopicManager<'a> {
    enabled: Vec<Topic>,
    all: Vec<Topic>,
    client: &'a Client,
    arch: &'a str,
    atm_state_path: PathBuf,
    atm_source_list_path: PathBuf,
    dry_run: bool,
    enabled_mirrors: Vec<Box<str>>,
}

impl<'a> TopicManager<'a> {
    const ATM_STATE_PATH_SUFFIX: &'a str = "var/lib/atm/state";
    const ATM_SOURCE_LIST_PATH_SUFFIX: &'a str = "etc/apt/sources.list.d/atm.list";

    pub fn new_blocking(
        client: &'a Client,
        sysroot: impl AsRef<Path>,
        arch: &'a str,
        dry_run: bool,
    ) -> Result<Self> {
        tokio::runtime::Builder::new_multi_thread()
            .enable_all()
            .build()
            .unwrap()
            .block_on(Self::new(client, sysroot, arch, dry_run))
    }

    pub async fn new(
        client: &'a Client,
        sysroot: impl AsRef<Path>,
        arch: &'a str,
        dry_run: bool,
    ) -> Result<Self> {
        let atm_state_path = sysroot.as_ref().join(Self::ATM_STATE_PATH_SUFFIX);

        let parent = atm_state_path
            .parent()
            .ok_or_else(|| OmaTopicsError::FailedGetParentPath(atm_state_path.to_path_buf()))?;

        if !parent.is_dir() {
            tokio::fs::create_dir_all(parent).await.map_err(|e| {
                OmaTopicsError::FailedToOperateDirOrFile(parent.display().to_string(), e)
            })?;
        }

        let enabled: Vec<Topic> = if !atm_state_path.is_file() {
            warn!("oma topics status file does not exist, a new status file will be created.");
            create_empty_state(&atm_state_path).await?
        } else {
            let atm_state_string =
                tokio::fs::read_to_string(&atm_state_path)
                    .await
                    .map_err(|e| {
                        OmaTopicsError::FailedToOperateDirOrFile(
                            atm_state_path.display().to_string(),
                            e,
                        )
                    })?;

            match serde_json::from_str(&atm_state_string) {
                Ok(v) => v,
                Err(e) => {
                    debug!("Deserialize oma topics state JSON failed: {e}");
                    warn!(
                        "oma topics status file is corrupted, a new status file will be created."
                    );
                    create_empty_state(&atm_state_path).await?
                }
            }
        };

        Ok(Self {
            enabled,
            all: vec![],
            client,
            arch,
            atm_state_path,
            dry_run,
            enabled_mirrors: enabled_mirror(sysroot.as_ref().to_path_buf()).await?,
            atm_source_list_path: sysroot.as_ref().join(Self::ATM_SOURCE_LIST_PATH_SUFFIX),
        })
    }

    pub fn all_topics(&self) -> &[Topic] {
        &self.all
    }

    pub fn enabled_topics(&self) -> &[Topic] {
        &self.enabled
    }

    /// Get all new topics
    pub async fn refresh(&mut self) -> Result<()> {
        let urls = self
            .enabled_mirrors
            .iter()
            .map(|x| {
                if x.ends_with('/') {
                    format!("{}debs/{TOPICS_JSON}", x)
                } else {
                    format!("{}/debs/{TOPICS_JSON}", x)
                }
            })
            .collect::<Vec<_>>();

        self.all = refresh_innter(self.client, urls, self.arch).await?;

        Ok(())
    }

    /// Enable select topic
    pub fn add(&mut self, topic: &str) -> Result<()> {
        debug!("oma will opt_in: {}", topic);

        let all = &self.all;

        debug!("all topic: {all:?}");

        let index = all
            .iter()
            .find(|x| x.name.to_ascii_lowercase() == topic.to_ascii_lowercase());

        let enabled_names = self.enabled.iter().map(|x| &x.name).collect::<Vec<_>>();

        debug!("Enabled: {enabled_names:?}");

        if let Some(index) = index {
            if !enabled_names.contains(&&index.name) {
                self.enabled.push(index.clone());
            }

            return Ok(());
        }

        debug!("index: {index:?} does not exist");

        Err(OmaTopicsError::CanNotFindTopic(topic.to_owned()))
    }

    /// Disable select topic
    pub fn remove(&mut self, topic: &str) -> Result<Topic> {
        let index = self
            .enabled
            .iter()
            .position(|x| x.name.to_ascii_lowercase() == topic.to_ascii_lowercase());

        debug!("oma will opt_out: {}", topic);
        debug!("index is: {index:?}");
        debug!("topic is: {topic}");
        debug!("enabled topics: {:?}", self.enabled);

        if let Some(index) = index {
            let d = self.enabled.remove(index);
            return Ok(d);
        }

        Err(OmaTopicsError::FailedToDisableTopic(topic.to_string()))
    }

    /// Write topic changes to mirror list
    pub async fn write_enabled(
        &self,
        source_list_comment: &str,
        message_cb: impl Fn(&str, &str),
    ) -> Result<()> {
        if self.dry_run {
            debug!("enabled: {:?}", self.enabled);
        }

        let mirrors = &self.enabled_mirrors;

        let mut new_source_list = String::new();

        new_source_list.push_str(&format!("{}\n", source_list_comment));

        for i in &self.enabled {
            new_source_list.push_str(&format!("# Topic `{}`\n", i.name));

            let mut tasks = vec![];
            for mirror in mirrors {
                tasks.push(self.mirror_topic_is_exist(format!(
                    "{}debs/dists/{}",
                    url_with_suffix(mirror),
                    i.name
                )))
            }

            let is_exists = futures::future::join_all(tasks).await;

            for (index, c) in is_exists.into_iter().enumerate() {
                if !c.unwrap_or(false) {
                    message_cb(&i.name, &mirrors[index]);
                    continue;
                }

                new_source_list.push_str(&format!(
                    "deb {}debs {} main\n",
                    url_with_suffix(&mirrors[index]),
                    i.name
                ));
            }
        }

        let s = serde_json::to_vec(&self.enabled).map_err(|_| OmaTopicsError::FailedSer)?;

        if self.dry_run {
            debug!("ATM State:\n{}", String::from_utf8_lossy(&s));
            debug!("atm.list:\n{}", new_source_list);
            return Ok(());
        }

        tokio::fs::write(&self.atm_state_path, s)
            .await
            .map_err(|e| {
                OmaTopicsError::FailedToOperateDirOrFile(
                    self.atm_state_path.display().to_string(),
                    e,
                )
            })?;

        tokio::fs::write(&self.atm_source_list_path, new_source_list)
            .await
            .map_err(|e| {
                OmaTopicsError::FailedToOperateDirOrFile(
                    self.atm_source_list_path.display().to_string(),
                    e,
                )
            })?;

        Ok(())
    }

    async fn mirror_topic_is_exist(&self, url: String) -> Result<bool> {
        check(self.client, &format!("{}/InRelease", url)).await
    }
}

async fn create_empty_state(atm_state_path: &Path) -> Result<Vec<Topic>> {
    let v = vec![];

    fs::write(atm_state_path, serde_json::to_vec(&v).unwrap())
        .await
        .map_err(|e| {
            OmaTopicsError::FailedToOperateDirOrFile(atm_state_path.display().to_string(), e)
        })?;

    Ok(v)
}

async fn get<T: DeserializeOwned>(client: &Client, url: String) -> Result<T> {
    let url = Url::parse(&url).map_err(OmaTopicsError::ParseUrl)?;

    let schema = url.scheme();

    match schema {
        "file" => {
            let path = url.path();
            let bytes = fs::read(path)
                .await
                .map_err(|e| OmaTopicsError::OpenFile(path.to_string(), e))?;
            serde_json::from_slice(&bytes)
                .map_err(|e| OmaTopicsError::ReadFile(path.to_string(), e))
        }
        x if x.starts_with("http") => {
            let res = client
                .get(url)
                .send()
                .await?
                .error_for_status()?
                .json::<T>()
                .await?;
            Ok(res)
        }
        _ => Err(OmaTopicsError::UnsupportedProtocol(url.to_string())),
    }
}

async fn check(client: &Client, url: &str) -> Result<bool> {
    let url = Url::parse(url).map_err(OmaTopicsError::ParseUrl)?;

    let schema = url.scheme();

    match schema {
        "file" => Ok(Path::new(url.path()).exists()),
        x if x.starts_with("http") => Ok(client.head(url).send().await?.error_for_status().is_ok()),
        _ => Err(OmaTopicsError::UnsupportedProtocol(url.to_string())),
    }
}

fn url_with_suffix(url: &str) -> Cow<str> {
    if url.ends_with('/') {
        Cow::Borrowed(url)
    } else {
        Cow::Owned(format!("{url}/"))
    }
}

async fn refresh_innter(client: &Client, urls: Vec<String>, arch: &str) -> Result<Vec<Topic>> {
    let mut json: Vec<Topic> = vec![];
    let mut tasks = vec![];

    for url in urls {
        let v = get::<Vec<Topic>>(client, url);
        tasks.push(v);
    }

    let res = futures::future::try_join_all(tasks).await?;

    for i in res {
        for j in i {
            match json.iter().position(|x| x.name == j.name) {
                Some(index) => {
                    if j.update_date > json[index].update_date {
                        json[index] = j.clone();
                    }
                }
                None => {
                    json.push(j);
                }
            }
        }
    }

    json.sort_unstable_by(|a, b| a.name.cmp(&b.name));

    let json = json
        .into_iter()
        .filter(|x| {
            x.arch
                .as_ref()
                .map(|x| x.contains(&arch.to_string()) || x.contains(&"all".to_string()))
                .unwrap_or(false)
        })
        .collect::<Vec<_>>();

    Ok(json)
}

/// Scan all close topics from upstream and disable it
pub async fn scan_closed_topic(
    tm: &mut TopicManager<'_>,
    comment: &str,
    message_cb: impl Fn(&str, &str),
) -> Result<Vec<String>> {
    tm.refresh().await?;
    let all: Box<[Topic]> = Box::from(tm.all_topics());
    let enabled: Box<[Topic]> = Box::from(tm.enabled_topics());

    let mut res = vec![];

    for i in enabled {
        if all.iter().all(|x| x.name != i.name) {
            let d = tm.remove(&i.name)?;
            res.push(d.name);
        }
    }

    if !res.is_empty() {
        tm.write_enabled(comment, message_cb).await?;
    }

    Ok(res)
}
