use std::path::Path;

use apt_auth_config::AuthConfig;
use dialoguer::console::style;
use inquire::{
    formatter::MultiOptionFormatter,
    ui::{Color, RenderConfig, StyleSheet, Styled},
    MultiSelect,
};
use oma_console::writer::Writeln;
use oma_history::SummaryType;
use oma_pm::{
    apt::{AptConfig, FilterMode, OmaApt, OmaAptArgs, Upgrade},
    query::OmaDatabase,
};
use oma_utils::dpkg::dpkg_arch;
use reqwest::Client;
use tokio::task::spawn_blocking;
use tracing::warn;

use crate::{
    error::OutputError,
    pb::OmaProgressBar,
    utils::{dbus_check, root},
    OmaArgs, RT,
};

use super::utils::{
    lock_oma, no_check_dbus_warn, select_tui_display_msg, tui_select_list_size, CommitRequest,
    RefreshRequest,
};
use crate::fl;
use anyhow::anyhow;
use oma_topics::{scan_closed_topic, Topic, TopicManager};

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
    pub no_check_dbus: bool,
    pub sysroot: String,
}

pub fn execute(args: TopicArgs, client: Client, oma_args: OmaArgs) -> Result<i32, OutputError> {
    root()?;
    lock_oma()?;

    let TopicArgs {
        opt_in,
        opt_out,
        dry_run,
        network_thread,
        no_progress,
        sysroot,
        no_check_dbus,
    } = args;

    let fds = if !no_check_dbus {
        Some(dbus_check(false)?)
    } else {
        no_check_dbus_warn();
        None
    };

    let sysroot_ref = &sysroot;
    let client_ref = &client;

    let topics_changed = RT.block_on(async move {
        topics_inner(
            opt_in,
            opt_out,
            dry_run,
            no_progress,
            sysroot_ref,
            &fl!("do-not-edit-topic-sources-list"),
            client_ref,
        )
        .await
    })?;

    let enabled_pkgs = topics_changed.enabled_pkgs;
    let downgrade_pkgs = topics_changed.downgrade_pkgs;

    let apt_config = AptConfig::new();
    let auth_config = AuthConfig::system(&sysroot)?;

    RefreshRequest {
        client: &client,
        dry_run,
        no_progress,
        limit: network_thread,
        sysroot: &sysroot,
        _refresh_topics: true,
        config: &apt_config,
        auth_config: &auth_config,
    }
    .run()?;

    let oma_apt_args = OmaAptArgs::builder()
        .sysroot(sysroot.clone())
        .another_apt_options(oma_args.another_apt_options)
        .build();

    let mut apt = OmaApt::new(vec![], oma_apt_args, false, apt_config)?;

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
    apt.upgrade(Upgrade::FullUpgrade)?;

    let request = CommitRequest {
        apt,
        dry_run,
        request_type: SummaryType::TopicsChanged {
            add: topics_changed.opt_in,
            remove: topics_changed.opt_out,
        },
        no_fixbroken: false,
        network_thread,
        no_progress,
        sysroot,
        fix_dpkg_status: true,
        protect_essential: oma_args.protect_essentials,
        client: &client,
        yes: false,
        remove_config: false,
        auth_config: &auth_config,
    };

    let code = request.run()?;

    drop(fds);

    Ok(code)
}

async fn topics_inner(
    mut opt_in: Vec<String>,
    mut opt_out: Vec<String>,
    dry_run: bool,
    no_progress: bool,
    sysroot: impl AsRef<Path>,
    topic_msg: &str,
    client: &Client,
) -> Result<TopicChanged, OutputError> {
    let dpkg_arch = dpkg_arch(&sysroot)?;
    let mut tm = TopicManager::new(client, sysroot, &dpkg_arch, dry_run).await?;

    refresh_topics(no_progress, &mut tm).await?;

    let all_topics = Box::from(tm.all_topics());
    let enabled_topics = Box::from(tm.enabled_topics());

    if opt_in.is_empty() && opt_out.is_empty() {
        (opt_in, opt_out) = spawn_blocking(move || select_prompt(&all_topics, &enabled_topics))
            .await
            .unwrap()?;
    }

    for i in &opt_in {
        tm.add(i)?;
    }

    let mut downgrade_pkgs = vec![];
    for i in &opt_out {
        let removed_topic = tm.remove(i)?;
        downgrade_pkgs.extend(removed_topic.packages);
    }

    if !opt_in.is_empty() || !opt_out.is_empty() {
        tm.write_enabled(topic_msg, |topic, mirror| {
            warn!(
                "{}",
                fl!("topic-not-in-mirror", topic = topic, mirror = mirror)
            );
            warn!("{}", fl!("skip-write-mirror"));
        })
        .await?;
    }

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

fn select_prompt(
    all_topics: &[Topic],
    enabled_topics: &[Topic],
) -> anyhow::Result<(Vec<String>, Vec<String>)> {
    let mut opt_in = vec![];
    let mut opt_out = vec![];

    let mut swap_count = 0;

    let mut all_topics = all_topics.to_vec();

    let enabled_names = &*enabled_topics.iter().map(|x| &x.name).collect::<Vec<_>>();

    // 把所有已启用的源排到最前面
    for i in enabled_names {
        let pos = all_topics.iter().position(|x| x.name == **i);

        if let Some(pos) = pos {
            let entry = all_topics.remove(pos);
            all_topics.insert(0, entry);
            swap_count += 1;
        }
    }

    let all_names = all_topics.iter().map(|x| &x.name).collect::<Vec<_>>();

    let default = (0..swap_count).collect::<Vec<_>>();

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

            select_tui_display_msg(&s, true).to_string()
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
    let page_size = tui_select_list_size();

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

    Ok((opt_in, opt_out))
}

async fn refresh_topics(no_progress: bool, tm: &mut TopicManager<'_>) -> Result<(), OutputError> {
    let pb = if !no_progress {
        let pb = OmaProgressBar::new_spinner(Some(fl!("refreshing-topic-metadata")));

        Some(pb)
    } else {
        None
    };

    tm.refresh().await?;

    scan_closed_topic(
        tm,
        &fl!("do-not-edit-topic-sources-list"),
        |topic, mirror| {
            if let Some(pb) = &pb {
                pb.writeln(
                    &style("WARNING").yellow().bold().to_string(),
                    &fl!("topic-not-in-mirror", topic = topic, mirror = mirror),
                )
                .ok();
                pb.writeln(
                    &style("WARNING").yellow().bold().to_string(),
                    &fl!("skip-write-mirror"),
                )
                .ok();
            } else {
                warn!(
                    "{}",
                    fl!("topic-not-in-mirror", topic = topic, mirror = mirror)
                );
                warn!("{}", fl!("skip-write-mirror"));
            }
        },
    )
    .await?;

    if let Some(pb) = pb {
        pb.inner.finish_and_clear();
    }

    Ok(())
}
