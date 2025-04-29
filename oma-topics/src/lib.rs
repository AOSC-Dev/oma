use std::{
    borrow::Cow,
    hash::Hash,
    io,
    path::{Path, PathBuf},
};

use ahash::{HashMap, HashSet};
use futures::future::try_join_all;
use itertools::Itertools;
use oma_mirror::MirrorManager;
use reqwest::Client;
use serde::{Deserialize, Serialize, de::DeserializeOwned};
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
    ParseUrl(url::ParseError, String),
    #[error("Unsupported url protocol from url: {0}")]
    UnsupportedProtocol(String),
    #[error("Failed to open file {0}: {1}")]
    OpenFile(String, io::Error),
    #[error("Failed to read file {0}: {1}")]
    ReadFile(String, serde_json::Error),
    #[error(transparent)]
    MirrorError(#[from] oma_mirror::MirrorError),
}

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

impl Eq for Topic {}

impl Hash for Topic {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.name.hash(state);
    }
}

pub struct TopicManager<'a> {
    enabled: Vec<Topic>,
    all: HashMap<Box<str>, Vec<Topic>>,
    client: &'a Client,
    arch: &'a str,
    atm_state_path: PathBuf,
    atm_source_list_path: PathBuf,
    atm_source_list_path_new: PathBuf,
    dry_run: bool,
    old_enabled: Vec<Topic>,
    mm: MirrorManager,
}

impl<'a> TopicManager<'a> {
    const ATM_STATE_PATH_SUFFIX: &'a str = "var/lib/atm/state";
    const ATM_SOURCE_LIST_PATH_SUFFIX: &'a str = "etc/apt/sources.list.d/atm.list";
    const ATM_SOURCE_LIST_PATH_NEW_SUFFIX: &'a str = "etc/apt/sources.list.d/atm.sources";

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

        let sysroot_box: Box<Path> = Box::from(sysroot.as_ref());

        Ok(Self {
            enabled: enabled.clone(),
            all: HashMap::with_hasher(ahash::RandomState::new()),
            client,
            arch,
            atm_state_path,
            dry_run,
            atm_source_list_path: sysroot.as_ref().join(Self::ATM_SOURCE_LIST_PATH_SUFFIX),
            atm_source_list_path_new: sysroot.as_ref().join(Self::ATM_SOURCE_LIST_PATH_NEW_SUFFIX),
            old_enabled: enabled,
            mm: tokio::task::spawn_blocking(move || MirrorManager::new(sysroot_box))
                .await
                .unwrap()?,
        })
    }

    pub fn available_topics(&self) -> impl Iterator<Item = &Topic> {
        self.all.values().flatten().unique()
    }

    pub fn enabled_topics(&self) -> &[Topic] {
        &self.enabled
    }

    /// Get all topics
    pub async fn refresh(&mut self) -> Result<()> {
        let enabled_mirrors = self.mm.enabled_mirrors();
        let tasks = enabled_mirrors
            .iter()
            .map(|x| refresh_innter(self.client, x.1, self.arch));

        let res = try_join_all(tasks).await?;
        let res = res
            .into_iter()
            .map(|(x, y)| (Box::from(x), y))
            .collect::<HashMap<Box<str>, _>>();

        self.all = res;

        Ok(())
    }

    /// Enable select topic
    pub fn add(&mut self, topic: &str) -> Result<()> {
        debug!("oma will opt_in: {}", topic);

        let index = self
            .available_topics()
            .find(|x| x.name.eq_ignore_ascii_case(topic));

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
            .position(|x| x.name.eq_ignore_ascii_case(topic));

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
    pub async fn write_enabled(&self, revert: bool) -> Result<()> {
        let enabled = if revert {
            &self.old_enabled
        } else {
            &self.enabled
        };

        let s = serde_json::to_vec(enabled).map_err(|_| OmaTopicsError::FailedSer)?;

        if self.dry_run {
            debug!("ATM State:\n{}", String::from_utf8_lossy(&s));
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

        Ok(())
    }

    pub async fn write_sources_list(
        &self,
        source_list_comment: &str,
        revert: bool,
        message_cb: impl AsyncFn(String, String),
    ) -> Result<()> {
        if self.dry_run {
            debug!("enabled: {:?}", self.enabled);
            return Ok(());
        }

        let is_deb822 = if self.atm_source_list_path_new.exists() {
            if self.atm_source_list_path.exists() {
                tokio::fs::remove_file(&self.atm_source_list_path)
                    .await
                    .map_err(|e| {
                        OmaTopicsError::FailedToOperateDirOrFile(
                            self.atm_source_list_path.display().to_string(),
                            e,
                        )
                    })?;
            }

            true
        } else {
            false
        };

        let mut new_source_list = String::new();

        new_source_list.push_str(&format!("{}\n", source_list_comment));

        let write_source = if revert {
            &self.old_enabled
        } else {
            &self.enabled
        };

        for i in write_source {
            new_source_list.push_str(&format!("# Topic `{}`\n", i.name));
            for (_, mirror) in self.mm.enabled_mirrors() {
                if !self.all.get(mirror).is_some_and(|x| x.contains(i)) {
                    message_cb(i.name.to_string(), mirror.to_string()).await;
                    continue;
                }

                if !is_deb822 {
                    new_source_list.push_str(&format!(
                        "deb {}debs {} main\n",
                        url_with_suffix(mirror),
                        i.name
                    ));
                } else {
                    new_source_list.push_str(&format!(
                        "Types: deb\nURIs: {}debs\nSuites: {}\nComponents: main\n\n",
                        url_with_suffix(mirror),
                        i.name
                    ));
                }
            }
        }

        let path = if is_deb822 {
            &self.atm_source_list_path_new
        } else {
            &self.atm_source_list_path
        };

        tokio::fs::write(path, new_source_list)
            .await
            .map_err(|e| OmaTopicsError::FailedToOperateDirOrFile(path.display().to_string(), e))?;

        Ok(())
    }

    pub fn remove_closed_topics(&mut self) -> Result<Vec<String>> {
        let all = self
            .available_topics()
            .map(|x| x.to_owned())
            .collect::<HashSet<_>>();

        let enabled: Box<[Topic]> = Box::from(self.enabled_topics());

        let mut res = vec![];

        for i in enabled {
            if !all.contains(&i) {
                debug!("removing closed topic {} ...", i.name);
                let d = self.remove(&i.name)?;
                res.push(d.name);
            }
        }

        Ok(res)
    }

    pub fn url_is_enabled_topic(&self, url: &str) -> bool {
        let enabled = self.enabled_topics();

        let Some(suite) = url.split('/').nth_back(1) else {
            return false;
        };

        enabled.iter().any(|x| x.name == suite)
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

async fn get<T: DeserializeOwned>(client: &Client, url: Url) -> Result<T> {
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

fn url_with_suffix(url: &str) -> Cow<str> {
    if url.ends_with('/') {
        Cow::Borrowed(url)
    } else {
        Cow::Owned(format!("{url}/"))
    }
}

async fn refresh_innter<'a>(
    client: &'a Client,
    url: &'a str,
    arch: &'a str,
) -> Result<(&'a str, Vec<Topic>)> {
    let topics_metadata_url = Url::parse(url)
        .and_then(|url| url.join("debs/manifest/topics.json"))
        .map_err(|e| OmaTopicsError::ParseUrl(e, url.to_string()))?;

    let json: Vec<Topic> = get(client, topics_metadata_url).await?;
    let json = json
        .into_iter()
        .filter(|x| {
            x.arch
                .as_ref()
                .is_some_and(|x| x.contains(&arch.to_string()) || x.contains(&"all".to_string()))
        })
        .collect::<Vec<_>>();

    Ok((url, json))
}
