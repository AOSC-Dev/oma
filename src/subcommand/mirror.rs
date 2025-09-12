use std::cmp::Ordering;
use std::fmt::Display;
use std::io::stdout;
use std::path::PathBuf;
use std::time::Duration;
use std::time::Instant;

use ahash::HashMap;
use anyhow::anyhow;
use clap::Args;
use clap::Subcommand;
use dialoguer::Sort;
use dialoguer::theme::ColorfulTheme;
use faster_hex::hex_string;
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
use oma_topics::TopicManager;
use oma_utils::dpkg::dpkg_arch;
use reqwest::blocking;
use sha2::Digest;
use sha2::Sha256;
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
        /// Do not refresh topics manifest.json file
        #[arg(long, help = fl!("clap-no-refresh-topics-help"))]
        no_refresh_topics: bool,
        /// Do not refresh repository metadata
        #[arg(long, help = fl!("clap-no-refresh-help"))]
        no_refresh: bool,
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
                ),
                MirrorSubCmd::Speedtest {
                    set_fastest,
                    #[cfg(feature = "aosc")]
                    no_refresh_topics,
                    no_refresh,
                } => speedtest(
                    no_progress,
                    set_fastest,
                    !no_refresh_topics && !config.no_refresh_topics(),
                    download_threads.unwrap_or_else(|| config.network_thread()),
                    no_refresh,
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
                ),
                MirrorSubCmd::SortMirrors {
                    no_refresh_topics,
                    no_refresh,
                } => set_order(
                    no_progress,
                    !no_refresh_topics && !config.no_refresh_topics(),
                    download_threads.unwrap_or_else(|| config.network_thread()),
                    no_refresh,
                ),
            }
        } else {
            tui(
                no_progress,
                !no_refresh_topics && !config.no_refresh_topics(),
                download_threads.unwrap_or_else(|| config.network_thread()),
                no_refresh,
            )
        }
    }
}

pub fn tui(
    no_progress: bool,
    refresh_topic: bool,
    network_threads: usize,
    no_refresh: bool,
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
        refresh(no_progress, network_threads, refresh_topic)?;
    }

    Ok(0)
}

pub enum Operate {
    Set,
    Add,
    Remove,
}

pub fn operate(
    no_progress: bool,
    refresh_topic: bool,
    network_threads: usize,
    no_refresh: bool,
    args: Vec<&str>,
    sysroot: PathBuf,
    subcmd: Operate,
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
        refresh(no_progress, network_threads, refresh_topic)?;
    }

    Ok(0)
}

pub fn set_order(
    no_progress: bool,
    refresh_topic: bool,
    network_threads: usize,
    no_refresh: bool,
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
        refresh(no_progress, network_threads, refresh_topic)?;
    }

    Ok(0)
}

#[derive(Debug, Tabled)]
struct MirrorScoreDisplay<'a> {
    name: &'a str,
    score: String,
}

pub fn speedtest(
    no_progress: bool,
    set_fastest: bool,
    refresh_topic: bool,
    network_threads: usize,
    no_refresh: bool,
) -> Result<i32, OutputError> {
    if set_fastest {
        root()?;
    }

    let mut mm = MirrorManager::new("/")?;

    let mirrors = mm.mirrors_iter()?.collect::<Vec<_>>();

    let pb = if !no_progress {
        Some(OmaProgressBar::new(
            ProgressBar::new(mirrors.len() as u64).with_style(
                ProgressStyle::with_template(
                    "{spinner:.green} ({pos}/{len}) [{wide_bar:.cyan/blue}]",
                )
                .unwrap()
                .progress_chars("=>-"),
            ),
        ))
    } else {
        None
    };

    let client = blocking::ClientBuilder::new()
        .user_agent(APP_USER_AGENT)
        .timeout(Duration::from_secs(120))
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
            refresh(no_progress, network_threads, refresh_topic)?;
        }
    }

    Ok(0)
}

fn refresh(
    no_progress: bool,
    network_threads: usize,
    refresh_topic: bool,
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
