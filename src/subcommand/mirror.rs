use std::cmp::Ordering;
use std::collections::HashMap;
use std::fmt::Display;
use std::io::stdout;
use std::time::Duration;
use std::time::Instant;

use anyhow::anyhow;
use apt_auth_config::AuthConfig;
use dialoguer::console::style;
use dialoguer::theme::ColorfulTheme;
use dialoguer::Sort;
use faster_hex::hex_string;
use inquire::formatter::MultiOptionFormatter;
use inquire::ui::Color;
use inquire::ui::RenderConfig;
use inquire::ui::StyleSheet;
use inquire::ui::Styled;
use inquire::MultiSelect;
use oma_console::indicatif::HumanBytes;
use oma_console::indicatif::ProgressBar;
use oma_console::indicatif::ProgressStyle;
use oma_console::success;
use oma_mirror::Mirror;
use oma_mirror::MirrorManager;
use oma_pm::apt::AptConfig;
use reqwest::blocking;
use sha2::Digest;
use sha2::Sha256;
use tabled::Tabled;
use tracing::{error, info};

use crate::error::OutputError;
use crate::fl;
use crate::pb::OmaProgressBar;
use crate::table::PagerPrinter;
use crate::utils::root;
use crate::APP_USER_AGENT;
use crate::HTTP_CLIENT;

use super::utils::tui_select_list_size;
use super::utils::RefreshRequest;

const REPO_TEST_SHA256: &str = "1e2a82e7babb443b2b26b61ce5dd2bd25b06b30422b42ee709fddd2cc3ffe231";
const TEST_FILE_PREFIX: &str = ".repotest";

struct MirrorDisplay((Box<str>, Mirror));

impl Display for MirrorDisplay {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} ({})", self.0 .1.desc, self.0 .0)?;

        Ok(())
    }
}

pub fn tui(
    no_progress: bool,
    refresh_topic: bool,
    network_threads: usize,
    no_refresh: bool,
) -> Result<i32, OutputError> {
    root()?;

    let mut mm = MirrorManager::new("/".into())?;
    let mut mirrors = mm.mirrors_iter()?.collect::<Vec<_>>();
    let enabled = mm.enabled_mirrors();

    sort_mirrors(&mut mirrors, enabled);

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

    let ans = MultiSelect::new(&fl!("select-mirror-prompt"), mirrors)
        .with_help_message(&fl!("tips"))
        .with_formatter(formatter)
        .with_default(&default)
        .with_page_size(page_size as usize)
        .with_render_config(render_config)
        .prompt()
        .map_err(|_| anyhow::anyhow!(""))?;

    let set = ans.iter().map(|x| x.0 .0.as_ref()).collect::<Vec<_>>();

    mm.set(&set)?;
    mm.write_status(Some(&fl!("do-not-edit-topic-sources-list")))?;

    if !no_refresh {
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
    subcmd: Operate,
) -> Result<i32, OutputError> {
    root()?;

    let mut mm = MirrorManager::new("/".into())?;

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
                mm.remove(i);
            }
        }
    }

    mm.write_status(Some(&fl!("do-not-edit-topic-sources-list")))?;

    if !no_refresh {
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

    let mut mm = MirrorManager::new("/".into())?;

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

    let mut mm = MirrorManager::new("/".into())?;

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

    let mut score_map = HashMap::new();

    info!("{}", fl!("mirror-speedtest-start"));

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
                        pb.writeln(&style("INFO").blue().bold().to_string(), &msg)
                            .ok();
                    } else {
                        info!("{}", msg);
                    }
                } else {
                    let msg = format!("{}: Checksum verification failed.", name);
                    if let Some(ref pb) = pb {
                        pb.writeln(&style("ERROR").red().bold().to_string(), &msg)
                            .ok();
                    } else {
                        error!("{}", msg);
                    }
                }
            }
            Err(e) => {
                let msg = format!("{}: {}", name, e.without_url());
                if let Some(ref pb) = pb {
                    pb.writeln(&style("ERROR").red().bold().to_string(), &msg)
                        .ok();
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
        .print_table(score_table, vec![&fl!("mirror-name"), &fl!("mirror-score")])
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
    let auth_config = AuthConfig::system("/")?;

    RefreshRequest {
        client: &HTTP_CLIENT,
        dry_run: false,
        no_progress,
        limit: network_threads,
        sysroot: "/",
        _refresh_topics: refresh_topic,
        config: &AptConfig::new(),
        auth_config: &auth_config,
    }
    .run()?;

    Ok(())
}

fn sort_mirrors(mirrors: &mut [(&str, &Mirror)], enabled: &indexmap::IndexMap<Box<str>, Box<str>>) {
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

#[test]
fn test_sort() {
    use indexmap::indexmap;
    use indexmap::IndexMap;

    let enabled: IndexMap<Box<str>, Box<str>> = indexmap! {};
    let m1 = Mirror {
        desc: "baka".into(),
        url: "bala".into(),
    };

    let m2 = Mirror {
        desc: "baka".into(),
        url: "bala".into(),
    };

    let m3 = Mirror {
        desc: "baka".into(),
        url: "bala".into(),
    };

    let mut mirrors = vec![("b", &m1), ("a", &m2)];

    sort_mirrors(&mut mirrors, &enabled);

    assert_eq!(
        mirrors.iter().map(|x| x.0).collect::<Vec<_>>(),
        vec!["a", "b"]
    );

    let enabled: IndexMap<Box<str>, Box<str>> = indexmap! {"c".into() => "baka".into()};
    let mut mirrors = vec![("b", &m1), ("a", &m2), ("c", &m3)];

    sort_mirrors(&mut mirrors, &enabled);

    assert_eq!(
        mirrors.iter().map(|x| x.0).collect::<Vec<_>>(),
        vec!["c", "a", "b"]
    );
}
