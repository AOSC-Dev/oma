use std::cmp::Ordering;
use std::fmt::Display;
use std::io::stdout;
use std::path::PathBuf;
use std::str::FromStr;
use std::sync::Mutex;
use std::time::Duration;
use std::time::Instant;

use ahash::HashMap;
use anyhow::Context;
use anyhow::anyhow;
use anyhow::bail;
use chrono::DateTime;
use clap::Args;
use clap::Subcommand;
use dialoguer::Sort;
use dialoguer::theme::ColorfulTheme;
use faster_hex::hex_string;
use humantime::format_duration;
use inquire::formatter::MultiOptionFormatter;
use inquire::ui::Color;
use inquire::ui::RenderConfig;
use inquire::ui::StyleSheet;
use inquire::ui::Styled;
use oma_console::indicatif::HumanBytes;
use oma_console::indicatif::ProgressBar;
use oma_console::indicatif::ProgressStyle;
use oma_mirror::MirrorManager;
use oma_mirror::parser::MirrorConfig;
use oma_pm::apt::AptConfig;
use oma_refresh::inrelease::Release;
use oma_topics::TopicManager;
use oma_utils::concat_url;
use oma_utils::dpkg::dpkg_arch;
use rayon::iter::IntoParallelRefIterator;
use rayon::iter::ParallelIterator;
use reqwest::Url;
use reqwest::blocking;
use sha2::Digest;
use sha2::Sha256;
use std::io::Write;
use tabled::Tabled;
use tracing::warn;
use tracing::{error, info};

use crate::APP_USER_AGENT;
use crate::HTTP_CLIENT;
use crate::RT;
use crate::args::HELP_TEMPLATE;
use crate::config::Config;
use crate::error::OutputError;
use crate::fl;
use crate::lang::SYSTEM_LANG;
use crate::pb::OmaProgressBar;
use crate::pb::Print;
use crate::subcommand::utils::multiselect;
use crate::success;
use crate::table::PagerPrinter;
use crate::utils::root;

use super::utils::Refresh;
use super::utils::auth_config;
use super::utils::create_progress_spinner;
use super::utils::select_tui_display_msg;
use super::utils::tui_select_list_size;
use crate::args::CliExecuter;

const REPO_TEST_SHA256: &str = "1e2a82e7babb443b2b26b61ce5dd2bd25b06b30422b42ee709fddd2cc3ffe231";
const TEST_FILE_PREFIX: &str = ".repotest";

struct MirrorDisplay((Box<str>, MirrorConfig));

impl Display for MirrorDisplay {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let desc = self
            .0
            .1
            .description
            .get(&*SYSTEM_LANG)
            .unwrap_or_else(|| self.0.1.description.get("default").unwrap());

        write!(
            f,
            "{}",
            select_tui_display_msg(&format!("{} ({})", desc, self.0.0), true)
        )?;

        Ok(())
    }
}

#[derive(Debug, Args)]
pub struct CliMirror {
    #[command(subcommand)]
    mirror_subcmd: Option<MirrorSubCmd>,
    /// Do not refresh topics manifest.json file
    #[arg(long, help = fl!("clap-no-refresh-topics-help"))]
    no_refresh_topics: bool,
    /// Do not refresh repository metadata
    #[arg(long, help = fl!("clap-no-refresh-help"))]
    no_refresh: bool,
    /// Run oma in "dry-run" mode. Useful for testing changes and operations without making changes to the system
    #[arg(from_global, help = fl!("clap-dry-run-help", long_help = fl!("clap-dry-run-long-help")))]
    dry_run: bool,
    /// Setup download threads (default as 4)
    #[arg(from_global, help = fl!("clap-download-threads-help"))]
    download_threads: Option<usize>,
    /// Set apt options
    #[arg(from_global, help = fl!("clap-apt-options-help"))]
    apt_options: Vec<String>,
}

#[derive(Debug, Subcommand)]
#[command(subcommand_help_heading = &**crate::args::HELP_HEADING)]
pub enum MirrorSubCmd {
    /// Set mirror(s) to sources.list
    #[command(about = fl!("clap-mirror-set-help"))]
    #[command(help_template = &*HELP_TEMPLATE)]
    #[command(next_help_heading = &**crate::args::ARG_HELP_HEADING)]
    Set {
        /// Enable mirror name(s)
        #[arg(required = true, help = fl!("clap-mirror-set-names-help"))]
        names: Vec<String>,
        /// Set sysroot target directory
        #[arg(from_global, help = fl!("clap-sysroot-help"))]
        sysroot: PathBuf,
        /// Do not refresh topics manifest.json file
        #[arg(long, help = fl!("clap-no-refresh-topics-help"))]
        no_refresh_topics: bool,
        /// Do not refresh repository metadata
        #[arg(long, help = fl!("clap-no-refresh-help"))]
        no_refresh: bool,
    },
    /// Add mirror(s) to sources.list
    #[command(about = fl!("clap-mirror-add-help"))]
    #[command(help_template = &*HELP_TEMPLATE)]
    #[command(next_help_heading = &**crate::args::ARG_HELP_HEADING)]
    Add {
        /// Add mirror name(s)
        #[arg(required = true, help = fl!("clap-mirror-add-names-help"))]
        names: Vec<String>,
        /// Set sysroot target directory
        #[arg(from_global, help = fl!("clap-sysroot-help"))]
        sysroot: PathBuf,
        /// Do not refresh topics manifest.json file
        #[arg(long, help = fl!("clap-no-refresh-topics-help"))]
        no_refresh_topics: bool,
        /// Do not refresh repository metadata
        #[arg(long, help = fl!("clap-no-refresh-help"))]
        no_refresh: bool,
    },
    /// Remove mirror(s) from sources.list
    #[command(about = fl!("clap-mirror-remove-help"))]
    #[command(help_template = &*HELP_TEMPLATE)]
    #[command(next_help_heading = &**crate::args::ARG_HELP_HEADING)]
    Remove {
        /// Remove mirror name(s)
        #[arg(required = true, help = fl!("clap-mirror-remove-names-help"))]
        names: Vec<String>,
        /// Set sysroot target directory
        #[arg(from_global, help = fl!("clap-sysroot-help"))]
        sysroot: PathBuf,
        /// Do not refresh topics manifest.json file
        #[arg(long, help = fl!("clap-no-refresh-topics-help"))]
        no_refresh_topics: bool,
        /// Do not refresh repository metadata
        #[arg(long, help = fl!("clap-no-refresh-help"))]
        no_refresh: bool,
    },
    /// Sort mirror(s) order
    #[command(about = fl!("clap-mirror-sort-mirrors-help"))]
    #[command(help_template = &*HELP_TEMPLATE)]
    #[command(next_help_heading = &**crate::args::ARG_HELP_HEADING)]
    SortMirrors {
        /// Do not refresh topics manifest.json file
        #[arg(long, help = fl!("clap-no-refresh-topics-help"))]
        no_refresh_topics: bool,
        /// Do not refresh repository metadata
        #[arg(long, help = fl!("clap-no-refresh-help"))]
        no_refresh: bool,
    },
    /// Speedtest mirror(s)
    #[command(about = fl!("clap-mirror-speedtest-help"))]
    #[command(help_template = &*HELP_TEMPLATE)]
    #[command(next_help_heading = &**crate::args::ARG_HELP_HEADING)]
    Speedtest {
        /// Also set fastest as mirror
        #[arg(long, help = fl!("clap-mirror-speedtest-set-fastest-help"))]
        set_fastest: bool,
        /// Network timeout in seconds (default: 120)
        #[arg(long, default_value = "120", help = fl!("clap-mirror-speedtest-timeout-help"))]
        timeout: f64,
        /// Do not refresh topics manifest.json file
        #[arg(long, help = fl!("clap-no-refresh-topics-help"))]
        no_refresh_topics: bool,
        /// Do not refresh repository metadata
        #[arg(long, help = fl!("clap-no-refresh-help"))]
        no_refresh: bool,
    },
    /// Get mirrors latency
    #[command(help_template = &*HELP_TEMPLATE)]
    #[command(next_help_heading = &**crate::args::ARG_HELP_HEADING)]
    Latency {
        /// Network timeout in seconds (default: 120)
        #[arg(long, default_value = "120", help = fl!("clap-mirror-speedtest-timeout-help"))]
        timeout: f64,
        /// Set output format as JSON
        #[arg(long, help = fl!("clap-json-help"))]
        json: bool,
    },
}

impl CliExecuter for CliMirror {
    fn execute(self, config: &Config, no_progress: bool) -> Result<i32, OutputError> {
        let CliMirror {
            mirror_subcmd,
            no_refresh_topics,
            no_refresh,
            dry_run,
            download_threads,
            apt_options,
        } = self;

        if dry_run {
            info!("Running in dry-run mode, Exit.");
            return Ok(0);
        }

        if let Some(subcmd) = mirror_subcmd {
            match subcmd {
                MirrorSubCmd::Set {
                    names,
                    sysroot,
                    no_refresh_topics,
                    no_refresh,
                } => operate(
                    no_progress,
                    !no_refresh_topics && !config.no_refresh_topics(),
                    download_threads.unwrap_or_else(|| config.network_thread()),
                    no_refresh,
                    names.iter().map(|x| x.as_str()).collect::<Vec<_>>(),
                    sysroot,
                    Operate::Set,
                    apt_options,
                ),
                MirrorSubCmd::Speedtest {
                    set_fastest,
                    no_refresh_topics,
                    no_refresh,
                    timeout,
                } => speedtest(
                    no_progress,
                    set_fastest,
                    !no_refresh_topics && !config.no_refresh_topics(),
                    download_threads.unwrap_or_else(|| config.network_thread()),
                    no_refresh,
                    apt_options,
                    timeout,
                ),
                MirrorSubCmd::Add {
                    names,
                    sysroot,
                    no_refresh_topics,
                    no_refresh,
                } => operate(
                    no_progress,
                    !no_refresh_topics && !config.no_refresh_topics(),
                    download_threads.unwrap_or_else(|| config.network_thread()),
                    no_refresh,
                    names.iter().map(|x| x.as_str()).collect::<Vec<_>>(),
                    sysroot,
                    Operate::Add,
                    apt_options,
                ),
                MirrorSubCmd::Remove {
                    names,
                    sysroot,
                    no_refresh_topics,
                    no_refresh,
                } => operate(
                    no_progress,
                    !no_refresh_topics && !config.no_refresh_topics(),
                    download_threads.unwrap_or_else(|| config.network_thread()),
                    no_refresh,
                    names.iter().map(|x| x.as_str()).collect::<Vec<_>>(),
                    sysroot,
                    Operate::Remove,
                    apt_options,
                ),
                MirrorSubCmd::SortMirrors {
                    no_refresh_topics,
                    no_refresh,
                } => set_order(
                    no_progress,
                    !no_refresh_topics && !config.no_refresh_topics(),
                    download_threads.unwrap_or_else(|| config.network_thread()),
                    no_refresh,
                    apt_options,
                ),
                MirrorSubCmd::Latency { timeout, json } => get_latency(timeout, no_progress, json),
            }
        } else {
            tui(
                no_progress,
                !no_refresh_topics && !config.no_refresh_topics(),
                download_threads.unwrap_or_else(|| config.network_thread()),
                no_refresh,
                apt_options,
            )
        }
    }
}

pub fn tui(
    no_progress: bool,
    refresh_topic: bool,
    network_threads: usize,
    no_refresh: bool,
    apt_options: Vec<String>,
) -> Result<i32, OutputError> {
    root()?;

    let mut mm = MirrorManager::new("/")?;
    let mut mirrors = mm
        .mirrors_iter()?
        .map(|x| (x.0, x.1.to_owned()))
        .collect::<Vec<_>>();

    let enabled = mm.enabled_mirrors();

    sort_mirrors(&mut mirrors, enabled);

    // 把已启用但自定义配置文件中已经删除的源靠前
    for (name, url) in enabled {
        if mirrors.iter().all(|(n, _)| name.as_ref() != *n) {
            mirrors.insert(
                0,
                (
                    url,
                    MirrorConfig {
                        description: [("default".into(), url.to_string())].into_iter().collect(),
                        url: url.to_owned(),
                    },
                ),
            );
        }
    }

    let mirrors = mirrors
        .iter()
        .map(|x| MirrorDisplay((x.0.into(), x.1.to_owned())))
        .collect::<Vec<_>>();

    let formatter: MultiOptionFormatter<MirrorDisplay> =
        &|a| format!("Activating {} mirrors", a.len());
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

    // 空行（最多两行）+ tips (最多两行) + prompt（最多两行）
    let page_size = tui_select_list_size();

    let default = (0..enabled.len()).collect::<Vec<_>>();

    let ans = multiselect(
        &fl!("select-mirror-prompt"),
        mirrors,
        formatter,
        render_config,
        page_size,
        default,
    )?;

    let set = ans.iter().map(|x| x.0.0.as_ref()).collect::<Vec<_>>();

    mm.set(&set)?;
    mm.write_status(Some(&fl!("do-not-edit-topic-sources-list")))?;

    if !no_refresh {
        refresh_enabled_topics_sources_list(no_progress)?;
        refresh(no_progress, network_threads, refresh_topic, apt_options)?;
    }

    Ok(0)
}

enum Operate {
    Set,
    Add,
    Remove,
}

#[allow(clippy::too_many_arguments)]
fn operate(
    no_progress: bool,
    refresh_topic: bool,
    network_threads: usize,
    no_refresh: bool,
    args: Vec<&str>,
    sysroot: PathBuf,
    subcmd: Operate,
    apt_options: Vec<String>,
) -> Result<i32, OutputError> {
    root()?;

    let mut mm = MirrorManager::new(sysroot)?;

    match subcmd {
        Operate::Set => {
            mm.set(&args)?;
        }
        Operate::Add => {
            for i in args {
                mm.add(i)?;
            }
        }
        Operate::Remove => {
            for i in args {
                mm.remove(i)?;
            }
        }
    }

    mm.write_status(Some(&fl!("do-not-edit-topic-sources-list")))?;

    if !no_refresh {
        refresh_enabled_topics_sources_list(no_progress)?;
        refresh(no_progress, network_threads, refresh_topic, apt_options)?;
    }

    Ok(0)
}

fn set_order(
    no_progress: bool,
    refresh_topic: bool,
    network_threads: usize,
    no_refresh: bool,
    apt_options: Vec<String>,
) -> Result<i32, OutputError> {
    root()?;

    let mut mm = MirrorManager::new("/")?;

    let mirrors = mm
        .enabled_mirrors()
        .keys()
        .map(|x| x.as_ref())
        .collect::<Vec<_>>();

    let page_size = tui_select_list_size();

    let sorted = Sort::with_theme(&ColorfulTheme::default())
        .with_prompt(fl!("set-mirror-order-prompt"))
        .items(&mirrors)
        .max_length(page_size.into())
        .interact()
        .map_err(|_| anyhow!(""))?;

    mm.set_order(&sorted);
    mm.write_status(Some(&fl!("do-not-edit-topic-sources-list")))?;

    if !no_refresh {
        refresh_enabled_topics_sources_list(no_progress)?;
        refresh(no_progress, network_threads, refresh_topic, apt_options)?;
    }

    Ok(0)
}

fn get_latency(timeout: f64, no_progress: bool, json: bool) -> Result<i32, OutputError> {
    let mm = MirrorManager::new("/")?;

    let client = blocking::ClientBuilder::new()
        .user_agent(APP_USER_AGENT)
        .timeout(Duration::from_secs_f64(timeout))
        .build()?;

    let mirrors = mm.mirrors_iter()?.collect::<Vec<_>>();

    let pb = if !no_progress && !json {
        Some(progress_bar(mirrors.len() as u64 - 5))
    } else {
        None
    };

    let origin_date = get_mirror_date(
        "https://repo-hk.aosc.io/debs/dists/stable/InRelease",
        &client,
        "origin",
        &pb,
    )?;

    let origin_date =
        DateTime::parse_from_rfc2822(&origin_date).context("Failed parse origin date")?;

    let result = Mutex::new(vec![]);

    mirrors
        .par_iter()
        .filter(|m| !["origin", "origin4", "origin6", "repo-hk", "fastly"].contains(&m.0))
        .map(|m| (m.0, &m.1.url))
        .map(|(m, url)| (m, concat_url(url, "debs/dists/stable/InRelease")))
        .map(|(m, url)| (m, get_mirror_date(&url, &client, m, &pb)))
        .filter_map(|(m, res)| {
            res.map_err(|e| {
                if let Some(pb) = &pb {
                    pb.error(&format!("{}: {}", m, e));
                } else {
                    error!("{}: {}", m, e);
                }
                result.lock().unwrap().push((m, Err(anyhow!("{e}"))));
                e
            })
            .ok()
            .map(|res| (m, res))
        })
        .map(|res| {
            (
                res.0,
                chrono::DateTime::parse_from_rfc2822(&res.1).expect("{} is not a rfc2822 fmt"),
            )
        })
        .for_each(|res| {
            let delta = origin_date - res.1;

            result.lock().unwrap().push((res.0, Ok(delta)));

            let delta_duration = delta.to_std().expect("Should not < 0");

            if let Some(pb) = &pb {
                if delta.is_zero() {
                    pb.info(&format!("{}: is newest", res.0));
                } else {
                    pb.info(&format!(
                        "{}: expired {}",
                        res.0,
                        format_duration(delta_duration)
                    ));
                }
            } else if delta.is_zero() {
                info!("{}: is newest", res.0);
            } else {
                info!("{}: expired {}", res.0, format_duration(delta_duration));
            }
        });

    if let Some(pb) = &pb {
        pb.inner.finish_and_clear();
    }

    if json {
        let result = result.into_inner().unwrap();
        let mut result_json = vec![];
        for (m, time) in result {
            result_json.push((
                m,
                match time {
                    Ok(t) => serde_json::json!({
                        "status": "ok",
                        "seconds": t.num_seconds(),
                    }),
                    Err(e) => serde_json::json!({
                        "status": "fail",
                        "reason": e.to_string(),
                    }),
                },
            ));
        }
        writeln!(
            stdout(),
            "{}",
            serde_json::to_string(&result_json).context("Failed to ser to JSON format")?
        )
        .ok();
    }

    Ok(0)
}

fn get_mirror_date(
    url: &str,
    client: &blocking::Client,
    m: &str,
    p: &Option<OmaProgressBar>,
) -> anyhow::Result<String> {
    let url = Url::parse(url)?;

    let inrelease = match url.scheme() {
        "http" | "https" => client
            .get(url)
            .send()
            .and_then(|x| x.error_for_status())
            .and_then(|x| x.text())?,
        "file" => std::fs::read_to_string(url.path())?,
        x => bail!("Unsupported protocol {x}"),
    };

    let release = oma_repo_verify::verify_inrelease_by_sysroot(&inrelease, None, "/", false)?;

    let release = Release::from_str(&release)?;
    let date = release
        .source
        .date
        .with_context(|| format!("mirror {} no date field found", m))?;

    if let Some(p) = p {
        p.inner.inc(1);
    }

    Ok(date)
}

#[derive(Debug, Tabled)]
struct MirrorScoreDisplay<'a> {
    name: &'a str,
    score: String,
}

fn speedtest(
    no_progress: bool,
    set_fastest: bool,
    refresh_topic: bool,
    network_threads: usize,
    no_refresh: bool,
    apt_options: Vec<String>,
    timeout: f64,
) -> Result<i32, OutputError> {
    if set_fastest {
        root()?;
    }

    let mut mm = MirrorManager::new("/")?;

    let mirrors = mm.mirrors_iter()?.collect::<Vec<_>>();

    let pb = if !no_progress {
        Some(progress_bar(mirrors.len() as u64))
    } else {
        None
    };

    let client = blocking::ClientBuilder::new()
        .user_agent(APP_USER_AGENT)
        .timeout(Duration::from_secs_f64(timeout))
        .build()?;

    let mut score_map = HashMap::with_hasher(ahash::RandomState::new());

    if let Some(ref pb) = pb {
        pb.info(&fl!("mirror-speedtest-start"));
    }

    for (name, mirror) in mirrors {
        let mut sha256 = Sha256::new();
        let timer = Instant::now();
        let res = client
            .get(format!("{}{}", mirror.url, TEST_FILE_PREFIX))
            .send()
            .and_then(|x| x.error_for_status())
            .and_then(|mut x| x.copy_to(&mut sha256));

        let dur = timer.elapsed();

        match res {
            Ok(_) => {
                if REPO_TEST_SHA256 == hex_string(&sha256.finalize()) {
                    score_map.insert(name, dur);
                    let msg = format!(
                        "{}: {}/s",
                        name,
                        HumanBytes((10.0 * 1024.0 * 1024.0 / dur.as_secs_f64()) as u64)
                    );
                    if let Some(ref pb) = pb {
                        pb.info(&msg);
                    } else {
                        info!("{msg}");
                    }
                } else {
                    let msg = format!("{name}: Checksum verification failed.");
                    if let Some(ref pb) = pb {
                        pb.error(&msg);
                    } else {
                        error!("{msg}");
                    }
                }
            }
            Err(e) => {
                let msg = format!("{}: {}", name, e.without_url());
                if let Some(ref pb) = pb {
                    pb.error(&msg);
                } else {
                    error!("{}", msg);
                }
            }
        }

        if let Some(ref pb) = pb {
            pb.inner.inc(1);
        }
    }

    if let Some(ref pb) = pb {
        pb.inner.finish_and_clear();
    }

    let mut printer = PagerPrinter::new(stdout());

    let mut score = score_map.iter().collect::<Vec<_>>();

    score.sort_unstable_by(|a, b| a.1.cmp(b.1));

    let score_table = score.iter().map(|x| MirrorScoreDisplay {
        name: x.0,
        score: format!(
            "{}/s",
            HumanBytes((10.0 * 1024.0 * 1024.0 / x.1.as_secs_f64()) as u64)
        ),
    });

    success!("{}\n", fl!("speedtest-complete"));

    printer
        .print_table(
            score_table,
            vec![&fl!("mirror-name"), &fl!("mirror-score")],
            None,
            None,
        )
        .ok();

    if set_fastest {
        let (name, _) = score.first().ok_or_else(|| OutputError {
            description: fl!("all-speedtest-failed"),
            source: None,
        })?;

        let name: Box<str> = Box::from(**name);
        mm.set(&[&name])?;
        mm.write_status(Some(&fl!("do-not-edit-topic-sources-list")))?;

        if !no_refresh {
            refresh_enabled_topics_sources_list(no_progress)?;
            refresh(no_progress, network_threads, refresh_topic, apt_options)?;
        }
    }

    Ok(0)
}

#[inline]
fn progress_bar(mirrors_len: u64) -> OmaProgressBar {
    OmaProgressBar::new(
        ProgressBar::new(mirrors_len).with_style(
            ProgressStyle::with_template("{spinner:.green} ({pos}/{len}) [{wide_bar:.cyan/blue}]")
                .unwrap()
                .progress_chars("=>-"),
        ),
    )
}

fn refresh(
    no_progress: bool,
    network_threads: usize,
    refresh_topic: bool,
    apt_options: Vec<String>,
) -> Result<(), OutputError> {
    let auth_config = auth_config("/");
    let auth_config = auth_config.as_ref();

    Refresh::builder()
        .client(&HTTP_CLIENT)
        .dry_run(false)
        .no_progress(no_progress)
        .network_thread(network_threads)
        .refresh_topics(refresh_topic)
        .config(&AptConfig::new())
        .maybe_auth_config(auth_config)
        .apt_options(apt_options.clone())
        .build()
        .run()?;

    success!("{}", fl!("successfully-refresh-without-status"));

    Ok(())
}

fn sort_mirrors(
    mirrors: &mut Vec<(&str, MirrorConfig)>,
    enabled: &indexmap::IndexMap<Box<str>, Box<str>>,
) {
    mirrors.sort_unstable_by(|a, b| {
        if enabled.contains_key(a.0) && !enabled.contains_key(b.0) {
            Ordering::Less
        } else if !enabled.contains_key(a.0) && enabled.contains_key(b.0) {
            Ordering::Greater
        } else {
            a.0.cmp(b.0)
        }
    });
}

fn refresh_enabled_topics_sources_list(no_progress: bool) -> Result<(), OutputError> {
    let pb = create_progress_spinner(no_progress, fl!("refreshing-topic-metadata"));

    let try_refresh = Ok(()).and_then(|_| -> Result<(), OutputError> {
        let arch = dpkg_arch("/")?;
        let mut tm = TopicManager::new_blocking(&HTTP_CLIENT, "/", &arch, false)?;
        RT.block_on(tm.refresh())?;
        tm.remove_closed_topics()?;
        RT.block_on(tm.write_sources_list(
            &fl!("do-not-edit-topic-sources-list"),
            false,
            async |topic, mirror| {
                warn!(
                    "{}",
                    fl!("topic-not-in-mirror", topic = topic, mirror = mirror)
                );
                warn!("{}", fl!("skip-write-mirror"));
            },
        ))?;
        Ok(())
    });

    if let Some(pb) = pb {
        pb.inner.finish_and_clear();
    }

    try_refresh?;

    Ok(())
}

#[test]
fn test_sort() {
    use indexmap::IndexMap;
    use indexmap::indexmap;

    let enabled: IndexMap<Box<str>, Box<str>> = indexmap! {};
    let m1 = MirrorConfig {
        description: [("default".into(), "baka".into())].into_iter().collect(),
        url: "bala".into(),
    };

    let m2 = MirrorConfig {
        description: [("default".into(), "baka".into())].into_iter().collect(),
        url: "bala".into(),
    };

    let m3 = MirrorConfig {
        description: [("default".into(), "baka".into())].into_iter().collect(),
        url: "bala".into(),
    };

    let mut mirrors = vec![("b", m1.clone()), ("a", m2.clone())];

    sort_mirrors(&mut mirrors, &enabled);

    assert_eq!(
        mirrors.iter().map(|x| x.0).collect::<Vec<_>>(),
        vec!["a", "b"]
    );

    let enabled: IndexMap<Box<str>, Box<str>> = indexmap! {"c".into() => "baka".into()};
    let mut mirrors = vec![("b", m1), ("a", m2), ("c", m3)];

    sort_mirrors(&mut mirrors, &enabled);

    assert_eq!(
        mirrors.iter().map(|x| x.0).collect::<Vec<_>>(),
        vec!["c", "a", "b"]
    );
}
