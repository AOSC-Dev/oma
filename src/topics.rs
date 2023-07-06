use std::path::PathBuf;

use anyhow::{anyhow, bail, Context, Result};
use apt_sources_lists::{SourceLine, SourcesLists};
use indexmap::IndexMap;
use indicatif::ProgressBar;
use inquire::{
    formatter::MultiOptionFormatter,
    ui::{Color, RenderConfig, StyleSheet, Styled},
    MultiSelect,
};
use once_cell::sync::Lazy;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::sync::atomic::Ordering;
use tokio::{io::AsyncWriteExt, runtime::Runtime, task::spawn_blocking};

use crate::{download::oma_spinner, fl, info, ARCH, DRYRUN};

static ATM_STATE: Lazy<PathBuf> = Lazy::new(|| {
    let p = PathBuf::from("/var/lib/atm/state");

    let top = p.parent().unwrap();

    if !top.exists() {
        let _ = std::fs::create_dir_all(top);
    }

    if !p.exists() {
        let _ = std::fs::File::create(&p);
    }

    p
});

static APT_GEN_LIST: Lazy<PathBuf> = Lazy::new(|| {
    let p = PathBuf::from("/var/lib/apt/gen/status.json");

    let top = p.parent().unwrap();

    if !top.exists() {
        let _ = std::fs::create_dir_all(top);
    }

    if !p.exists() {
        let _ = std::fs::File::create(&p);
    }

    p
});

#[derive(Deserialize)]
struct GenList {
    mirror: IndexMap<String, String>,
}

fn enabled_mirror() -> Result<Vec<String>> {
    let s = std::fs::read_to_string(&*APT_GEN_LIST)?;
    let gen_list: GenList = serde_json::from_str(&s)?;

    let urls = gen_list
        .mirror
        .values()
        .map(|x| x.to_owned())
        .collect::<Vec<_>>();

    Ok(urls)
}

const TOPICS_JSON: &str = "manifest/topics.json";

#[derive(Debug, Deserialize, Serialize, Clone, PartialEq)]
pub struct Topic {
    pub name: String,
    description: Option<String>,
    date: u64,
    #[serde(skip_serializing)]
    arch: Option<Vec<String>>,
    pub packages: Vec<String>,
}

#[derive(Debug)]
pub struct TopicManager {
    pub enabled: Vec<Topic>,
    all: Vec<Topic>,
}

impl TryFrom<&str> for TopicManager {
    type Error = anyhow::Error;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        Ok(Self {
            enabled: serde_json::from_str(value)?,
            all: vec![],
        })
    }
}

impl TopicManager {
    pub fn new() -> Result<Self> {
        let f = std::fs::read_to_string(&*ATM_STATE)
            .map_err(|e| anyhow!("Failed to read atm state file: {e}."))?;

        Ok(Self {
            enabled: serde_json::from_str(&f).unwrap_or(vec![]),
            all: vec![],
        })
    }

    async fn refresh_all_topics(&mut self, client: &Client) -> Result<Vec<Topic>> {
        let urls = enabled_mirror()
            .unwrap_or_else(|_| {
                info!("apt-gen-list status file is empty, fallbacking to repo.aosc.io ...");
                vec!["https://repo.aosc.io/".to_string()]
            })
            .iter()
            .map(|x| {
                if x.ends_with('/') {
                    format!("{}debs/{TOPICS_JSON}", x)
                } else {
                    format!("{}/debs/{TOPICS_JSON}", x)
                }
            })
            .collect::<Vec<_>>();

        let all = refresh_all_topics_innter(client, urls).await?;

        self.all = all.clone();

        Ok(all)
    }

    pub fn opt_in(&mut self, client: &Client, rt: &Runtime, topic: &str) -> Result<()> {
        tracing::info!("oma will opt_in: {}", topic);

        if DRYRUN.load(Ordering::Relaxed) {
            return Ok(());
        }
        let all = if self.all.is_empty() {
            rt.block_on(self.refresh_all_topics(client))?
        } else {
            self.all.clone()
        };

        tracing::debug!("all topic: {all:?}");

        let index = all.iter().find(|x| {
            x.name.to_ascii_lowercase() == topic.to_ascii_lowercase()
                && x.arch
                    .as_ref()
                    .map(|x| x.contains(ARCH.get().unwrap()) || x.contains(&"all".to_string()))
                    == Some(true)
        });

        let enabled_names = self.enabled.iter().map(|x| &x.name).collect::<Vec<_>>();

        tracing::debug!("Enabled: {enabled_names:?}");

        if let Some(index) = index {
            if !enabled_names.contains(&&index.name) {
                self.enabled.push(index.clone());
            }

            return Ok(());
        }

        tracing::debug!("index: {index:?} does not exist");

        bail!(fl!("can-not-find-specified-topic", topic = topic))
    }

    pub fn opt_out(&mut self, topic: &str) -> Result<Vec<String>> {
        let index = self
            .enabled
            .iter()
            .position(|x| x.name.to_ascii_lowercase() == topic.to_ascii_lowercase());

        if DRYRUN.load(Ordering::Relaxed) {
            info!("oma will opt_out: {}", topic);
            return Ok(self.enabled[index.unwrap()].packages.clone());
        }

        if let Some(index) = index {
            let d = self.enabled.remove(index);
            let pkgs = d.packages;
            return Ok(pkgs);
        }

        bail!(fl!("failed-to-enable-following-topics", topic = topic))
    }

    pub async fn write_enabled(&self, client: &Client) -> Result<()> {
        info!("{}", fl!("saving-topic-settings"));
        if DRYRUN.load(Ordering::Relaxed) {
            return Ok(());
        }

        let mut f = tokio::fs::File::create("/etc/apt/sources.list.d/atm.list").await?;
        let mirrors = enabled_mirror().unwrap_or_else(|_| {
            info!("apt-gen-list status file is empty, fallbacking to repo.aosc.io ...");
            vec!["https://repo.aosc.io/".to_string()]
        });

        f.write_all(format!("{}\n", fl!("do-not-edit-topic-sources-list")).as_bytes())
            .await?;

        let pb = ProgressBar::new_spinner();
        oma_spinner(&pb);
        for i in &self.enabled {
            f.write_all(format!("# Topic `{}`\n", i.name).as_bytes())
                .await?;
            for j in &mirrors {
                if mirror_is_ok(client, j).await {
                    f.write_all(
                        format!(
                            "deb {}debs {} main\n",
                            if j.ends_with('/') {
                                j.to_owned()
                            } else {
                                format!("{j}/")
                            },
                            i.name
                        )
                        .as_bytes(),
                    )
                    .await?;
                }
            }
        }
        pb.finish_and_clear();

        let s = serde_json::to_vec(&self.enabled)?;

        tokio::fs::write(&*ATM_STATE, s).await?;

        Ok(())
    }
}

async fn refresh_all_topics_innter(client: &Client, urls: Vec<String>) -> Result<Vec<Topic>> {
    let mut json = vec![];

    let mut tasks = vec![];

    for url in urls {
        let v = client.get(url).send();
        tasks.push(v);
    }

    let res = futures::future::try_join_all(tasks).await?;

    let mut tasks = vec![];

    for i in res {
        tasks.push(i.error_for_status()?.json::<Vec<Topic>>());
    }

    let res = futures::future::try_join_all(tasks).await?;

    for i in res {
        let f =
            i.into_iter()
                .filter(|x| {
                    !json.contains(x)
                        && x.arch.as_ref().map(|x| {
                            x.contains(ARCH.get().unwrap()) || x.contains(&"all".to_string())
                        }) == Some(true)
                })
                .collect::<Vec<_>>();

        json.extend(f);
    }

    Ok(json)
}

async fn mirror_is_ok(client: &Client, url: &str) -> bool {
    match client.get(url).send().await {
        Ok(r) => {
            tracing::debug!(
                "Mirror: {url} is {}.",
                match r.error_for_status_ref().is_ok() {
                    true => "ok",
                    false => "not ok",
                }
            );
            r.error_for_status().is_ok()
        }
        Err(_) => {
            tracing::debug!("Mirror: {url} is not ok. still sync progress?");
            false
        }
    }
}

fn list(tm: &mut TopicManager, client: &Client, rt: &Runtime) -> Result<Vec<String>> {
    let all = rt.block_on(tm.refresh_all_topics(client))?;

    let ft = all
        .iter()
        .map(|x| {
            let mut s = x.name.clone();
            if let Some(d) = &x.description {
                s += &format!(" ({d})");
            }
            s
        })
        .collect::<Vec<_>>();

    Ok(ft)
}

pub fn dialoguer(
    tm: &mut TopicManager,
    rt: &Runtime,
    client: &Client,
) -> Result<(Vec<String>, Vec<String>)> {
    let mut opt_in = vec![];
    let mut opt_out = vec![];
    let pb = ProgressBar::new_spinner();
    oma_spinner(&pb);
    pb.set_message(fl!("refreshing-topic-metadata"));
    let display = list(tm, client, rt)?;
    pb.finish_and_clear();
    let all = tm.all.clone();

    let enabled_names = tm.enabled.iter().map(|x| &x.name).collect::<Vec<_>>();
    let all_names = all.iter().map(|x| &x.name).collect::<Vec<_>>();

    let mut default = vec![];

    for (i, c) in all_names.iter().enumerate() {
        if enabled_names.contains(c) {
            default.push(i);
        }
    }

    if DRYRUN.load(Ordering::Relaxed) {
        info!(
            "Your running in dry_run mode, oma will select first: {} to opt_in",
            all_names[0]
        );
        opt_in.push(all_names[0].clone());
        return Ok((opt_in, opt_out));
    }

    let formatter: MultiOptionFormatter<&str> = &|a| format!("Activating {} topics", a.len());

    let render_config = RenderConfig {
        selected_checkbox: Styled::new("✔").with_fg(Color::LightGreen),
        help_message: StyleSheet::empty().with_fg(Color::LightBlue),
        unselected_checkbox: Styled::new(" "),
        highlighted_option_prefix: Styled::new(""),
        selected_option: Some(StyleSheet::new().with_fg(Color::DarkCyan)),
        scroll_down_prefix: Styled::new("▼"),
        scroll_up_prefix: Styled::new("▲"),
        ..Default::default()
    };

    let ans = match MultiSelect::new(
        fl!("select-topics-dialog").as_str(),
        display.iter().map(|x| x.as_str()).collect(),
    )
    .with_help_message(fl!("tips").as_str())
    .with_formatter(formatter)
    .with_default(&default)
    .with_page_size(20)
    .with_render_config(render_config)
    .prompt()
    {
        Ok(ans) => ans,
        Err(_) => bail!(""),
    };

    for i in &ans {
        let index = display.iter().position(|x| x == i).unwrap();
        if !enabled_names.contains(&all_names[index]) {
            opt_in.push(all_names[index].clone());
        }
    }

    for (i, c) in all_names.iter().enumerate() {
        if enabled_names.contains(c) && !ans.contains(&display[i].as_str()) {
            opt_out.push(c.to_string());
        }
    }

    Ok((opt_in, opt_out))
}

pub async fn scan_closed_topic(client: &Client) -> Result<Vec<String>> {
    let mut atm_sources = vec![];
    let s = SourcesLists::new_from_paths(vec!["/etc/apt/sources.list.d/atm.list"].iter())?;

    for file in s.iter() {
        for i in &file.lines {
            if let SourceLine::Entry(entry) = i {
                atm_sources.push(entry.to_owned());
            }
        }
    }

    let mut tm = spawn_blocking(TopicManager::new).await??;

    let all = tm.refresh_all_topics(client).await?;

    let mut res = vec![];

    for i in atm_sources {
        let suite = i.suite;
        let suite_clone = suite.clone();

        if all.iter().all(|x| x.name != suite) {
            info!("{}", fl!("scan-topic-is-removed", name = suite.as_str()));
            rm_topic(&suite, client).await?;
        }

        res.push(suite_clone);
    }

    Ok(res)
}

pub async fn rm_topic(name: &str, client: &Client) -> Result<()> {
    if DRYRUN.load(Ordering::Relaxed) {
        return Ok(());
    }

    let mut tm = spawn_blocking(TopicManager::new).await??;

    let mut enabled = tm.enabled;

    let index = enabled
        .iter()
        .position(|x| x.name == name)
        .context(fl!("can-not-find-specified-topic", topic = name))?;

    info!("{}", fl!("removing-topic", name = name));

    enabled.remove(index);

    tm.enabled = enabled;

    tm.write_enabled(client).await?;

    Ok(())
}
