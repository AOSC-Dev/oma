use std::{path::Path, sync::atomic::Ordering};

use dialoguer::console::{self, style};
use inquire::{
    formatter::MultiOptionFormatter,
    ui::{Color, RenderConfig, StyleSheet, Styled},
    MultiSelect,
};
use oma_console::{indicatif::ProgressBar, pb::oma_spinner, writer::bar_writeln, WRITER};
use oma_history::SummaryType;
use oma_pm::{
    apt::{AptArgsBuilder, FilterMode, OmaApt, OmaAptArgsBuilder},
    query::OmaDatabase,
};
use oma_utils::dpkg::dpkg_arch;
use reqwest::Client;
use tracing::warn;

use crate::{
    error::OutputError,
    utils::{create_async_runtime, dbus_check, root},
    AILURUS,
};

use super::utils::{lock_oma, no_check_dbus_warn, normal_commit, refresh, NormalCommitArgs};
use crate::fl;
use anyhow::anyhow;
use oma_topics::{scan_closed_topic, TopicManager};

struct TopicChanged {
    opt_in: Vec<String>,
    opt_out: Vec<String>,
    enabled_pkgs: Vec<String>,
    downgrade_pkgs: Vec<String>,
}

pub struct TopicArgs {
    pub opt_in: Vec<String>,
    pub opt_out: Vec<String>,
    pub dry_run: bool,
    pub network_thread: usize,
    pub no_progress: bool,
    pub download_pure_db: bool,
    pub no_check_dbus: bool,
    pub sysroot: String,
}

pub fn execute(args: TopicArgs, client: Client) -> Result<i32, OutputError> {
    root()?;
    lock_oma()?;

    let TopicArgs {
        opt_in,
        opt_out,
        dry_run,
        network_thread,
        no_progress,
        download_pure_db,
        sysroot,
        no_check_dbus,
    } = args;

    let rt = create_async_runtime()?;

    let fds = if !no_check_dbus {
        Some(dbus_check(&rt, false)?)
    } else {
        no_check_dbus_warn();
        None
    };

    let sysroot_ref = &sysroot;

    let topics_changed = rt.block_on(async move {
        topics_inner(
            opt_in,
            opt_out,
            dry_run,
            no_progress,
            || format!("{}\n", fl!("do-not-edit-topic-sources-list")),
            sysroot_ref,
        )
        .await
    })?;

    let enabled_pkgs = topics_changed.enabled_pkgs;
    let downgrade_pkgs = topics_changed.downgrade_pkgs;

    refresh(
        &client,
        dry_run,
        no_progress,
        download_pure_db,
        network_thread,
        &sysroot,
        true,
    )?;

    let oma_apt_args = OmaAptArgsBuilder::default()
        .sysroot(sysroot.clone())
        .build()?;

    let mut apt = OmaApt::new(vec![], oma_apt_args, false)?;

    let mut pkgs = vec![];

    let db = OmaDatabase::new(&apt.cache)?;

    for pkg in downgrade_pkgs {
        let mut f = apt
            .filter_pkgs(&[FilterMode::Default])?
            .filter(|x| x.name() == pkg);

        if let Some(pkg) = f.next() {
            if enabled_pkgs.contains(&pkg.name().to_string()) {
                continue;
            }

            if pkg.is_installed() {
                let pkginfo = db.find_candidate_by_pkgname(pkg.name())?;

                pkgs.push(pkginfo);
            }
        }
    }

    apt.install(&pkgs, false)?;
    apt.upgrade()?;

    let args = NormalCommitArgs {
        apt,
        dry_run,
        typ: SummaryType::TopicsChanged {
            add: topics_changed.opt_in,
            remove: topics_changed.opt_out,
        },
        apt_args: AptArgsBuilder::default().no_progress(no_progress).build()?,
        no_fixbroken: false,
        network_thread,
        no_progress,
        sysroot,
        fix_dpkg_status: true,
    };

    normal_commit(args, &client)?;

    drop(fds);

    Ok(0)
}

async fn topics_inner<F, P>(
    mut opt_in: Vec<String>,
    mut opt_out: Vec<String>,
    dry_run: bool,
    no_progress: bool,
    callback: F,
    sysroot: P,
) -> Result<TopicChanged, OutputError>
where
    F: Fn() -> String,
    P: AsRef<Path>,
{
    let dpkg_arch = dpkg_arch(&sysroot)?;
    let mut tm = TopicManager::new(&sysroot, &dpkg_arch).await?;

    refresh_topics(no_progress, &mut tm, sysroot).await?;

    if opt_in.is_empty() && opt_out.is_empty() {
        inquire(&mut tm, &mut opt_in, &mut opt_out).await?;
    }

    for i in &opt_in {
        tm.add(i, dry_run)?;
    }

    let mut downgrade_pkgs = vec![];
    for i in &opt_out {
        let removed_topic = tm.remove(i, false)?;
        downgrade_pkgs.extend(removed_topic.packages);
    }

    tm.write_enabled(dry_run, callback, |topic, mirror| {
        warn!(
            "{}",
            fl!("topic-not-in-mirror", topic = topic, mirror = mirror)
        );
        warn!("{}", fl!("skip-write-mirror"));
    })
    .await?;

    let enabled_pkgs = tm
        .enabled_topics()
        .iter()
        .flat_map(|x| x.packages.clone())
        .collect::<Vec<_>>();

    Ok(TopicChanged {
        opt_in,
        opt_out,
        enabled_pkgs,
        downgrade_pkgs,
    })
}

async fn inquire(
    tm: &mut TopicManager,
    opt_in: &mut Vec<String>,
    opt_out: &mut Vec<String>,
) -> Result<(), OutputError> {
    let all_topics = tm.all_topics();

    let enabled_names = tm
        .enabled_topics()
        .iter()
        .map(|x| &x.name)
        .collect::<Vec<_>>();

    let mut swap_count = 0;

    let mut all_topics = all_topics.to_vec();

    // 把所有已启用的源排到最前面
    for i in &enabled_names {
        let pos = all_topics.iter().position(|x| x.name == **i);

        if let Some(pos) = pos {
            let entry = all_topics.remove(pos);
            all_topics.insert(0, entry);
            swap_count += 1;
        }
    }

    let all_names = all_topics.iter().map(|x| &x.name).collect::<Vec<_>>();

    let default = (0..swap_count).collect::<Vec<_>>();

    let term_width = WRITER.get_length() as usize;
    let display = all_topics
        .iter()
        .map(|x| {
            let mut s = String::new();

            if let Some(desc) = &x.description {
                s += &style(desc).bold().to_string();
                s += &format!(" ({})", x.name);
            } else {
                s += &style(&x.name).bold().to_string();
            }

            // 4 是 inquire 前面有四个空格缩进
            // 3 是 ... 的长度
            if console::measure_text_width(&s) + 4 > term_width {
                console::truncate_str(&s, term_width - 4 - 3, "...").to_string()
            } else {
                s
            }
        })
        .collect::<Vec<_>>();

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

    // 空行（最多两行）+ tips (最多两行) + prompt（最多两行）
    let page_size = match WRITER.get_height() {
        0 => panic!("Terminal height must be greater than 0"),
        1..=6 => 1,
        x @ 7..=25 => x - 6,
        26.. => 20,
    };

    let ans = MultiSelect::new(
        &fl!("select-topics-dialog"),
        display.iter().map(|x| x.as_ref()).collect(),
    )
    .with_help_message(&fl!("tips"))
    .with_formatter(formatter)
    .with_default(&default)
    .with_page_size(page_size as usize)
    .with_render_config(render_config)
    .prompt()
    .map_err(|_| anyhow!(""))?;

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

    Ok(())
}

async fn refresh_topics<P: AsRef<Path>>(
    no_progress: bool,
    tm: &mut TopicManager,
    sysroot: P,
) -> Result<(), OutputError> {
    let pb = if !no_progress {
        let pb = ProgressBar::new_spinner();
        let (style, inv) = oma_spinner(AILURUS.load(Ordering::Relaxed));
        pb.set_style(style);
        pb.enable_steady_tick(inv);
        pb.set_message(fl!("refreshing-topic-metadata"));

        Some(pb)
    } else {
        None
    };

    tm.refresh().await?;
    scan_closed_topic(
        || format!("{}\n", fl!("do-not-edit-topic-sources-list")),
        |topic, mirror| {
            if let Some(pb) = &pb {
                bar_writeln(
                    |s| {
                        pb.println(s);
                    },
                    &style("WARNING").yellow().bold().to_string(),
                    &fl!("topic-not-in-mirror", topic = topic, mirror = mirror),
                );
                bar_writeln(
                    |s| {
                        pb.println(s);
                    },
                    &style("WARNING").yellow().bold().to_string(),
                    &fl!("skip-write-mirror"),
                );
            } else {
                warn!(
                    "{}",
                    fl!("topic-not-in-mirror", topic = topic, mirror = mirror)
                );
                warn!("{}", fl!("skip-write-mirror"));
            }
        },
        sysroot,
        &tm.arch,
    )
    .await?;

    if let Some(pb) = pb {
        pb.finish_and_clear();
    }

    Ok(())
}
