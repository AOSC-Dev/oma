use std::{
    fmt::Display,
    path::{Path, PathBuf},
};

use apt_auth_config::AuthConfig;
use clap::{ArgAction, Args};
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
    matches::{GetArchMethod, PackagesMatcher},
};
use oma_utils::dpkg::dpkg_arch;
use reqwest::Client;
use tokio::task::spawn_blocking;
use tracing::warn;

use crate::{
    config::Config,
    error::OutputError,
    pb::OmaProgressBar,
    utils::{dbus_check, root},
    HTTP_CLIENT, RT,
};

use super::utils::{
    lock_oma, no_check_dbus_warn, select_tui_display_msg, tui_select_list_size, CommitChanges,
    RefreshRequest,
};

use crate::args::CliExecuter;

use crate::fl;
use anyhow::anyhow;
use oma_topics::{scan_closed_topic, Topic, TopicManager};

#[derive(Debug, Args)]
pub struct Topics {
    /// Enroll in one or more topic(s), delimited by space
    #[arg(long, action = ArgAction::Append)]
    opt_in: Vec<String>,
    /// Withdraw from one or more topic(s) and rollback to stable versions, delimited by space
    #[arg(long, action = ArgAction::Append)]
    opt_out: Vec<String>,
    /// Fix apt broken status
    #[arg(short, long)]
    no_fixbroken: bool,
    /// Install package(s) without fsync(2)
    #[arg(long)]
    force_unsafe_io: bool,
    /// Ignore repository and package dependency issues
    #[arg(long)]
    force_yes: bool,
    /// Replace configuration file(s) in the system those shipped in the package(s) to be installed (invokes `dpkg --force-confnew`)
    #[arg(long)]
    force_confnew: bool,
    /// Auto remove unnecessary package(s)
    #[arg(long)]
    autoremove: bool,
    /// Remove package(s) also remove configuration file(s), like apt purge
    #[arg(long, visible_alias = "purge")]
    remove_config: bool,
    /// Run oma in “dry-run” mode. Useful for testing changes and operations without making changes to the system
    #[arg(from_global)]
    dry_run: bool,
    /// Run oma do not check dbus
    #[arg(from_global)]
    no_check_dbus: bool,
    /// Set sysroot target directory
    #[arg(from_global)]
    sysroot: PathBuf,
    /// Set apt options
    #[arg(from_global)]
    apt_options: Vec<String>,
}

struct TopicChanged {
    opt_in: Vec<String>,
    opt_out: Vec<String>,
    enabled_pkgs: Vec<String>,
    downgrade_pkgs: Vec<String>,
}

struct TopicDisplay<'a> {
    topic: &'a Topic,
}

impl Display for TopicDisplay<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut s = String::new();

        if let Some(desc) = &self.topic.description {
            s += &style(desc).bold().to_string();
            s += &format!(" ({})", self.topic.name);
        } else {
            s += &style(&self.topic.name).bold().to_string();
        }

        let s = select_tui_display_msg(&s, true);

        write!(f, "{}", s)?;

        Ok(())
    }
}

impl CliExecuter for Topics {
    fn execute(self, config: &Config, no_progress: bool) -> Result<i32, OutputError> {
        root()?;
        lock_oma()?;

        let Topics {
            opt_in,
            opt_out,
            no_fixbroken,
            force_unsafe_io,
            force_yes,
            force_confnew,
            autoremove,
            remove_config,
            dry_run,
            no_check_dbus,
            sysroot,
            apt_options,
        } = self;

        let _fds = if !no_check_dbus && !config.no_check_dbus() {
            Some(dbus_check(false)?)
        } else {
            no_check_dbus_warn();
            None
        };

        let sysroot_ref = &sysroot;
        let topics_changed = RT.block_on(async move {
            topics_inner(
                opt_in,
                opt_out,
                dry_run,
                no_progress,
                sysroot_ref,
                &fl!("do-not-edit-topic-sources-list"),
                &HTTP_CLIENT,
            )
            .await
        })?;

        let enabled_pkgs = topics_changed.enabled_pkgs;
        let downgrade_pkgs = topics_changed.downgrade_pkgs;

        let apt_config = AptConfig::new();
        let auth_config = AuthConfig::system(&sysroot)?;

        RefreshRequest {
            client: &HTTP_CLIENT,
            dry_run,
            no_progress,
            limit: config.network_thread(),
            sysroot: &sysroot.to_string_lossy(),
            _refresh_topics: true,
            config: &apt_config,
            auth_config: &auth_config,
        }
        .run()?;

        let oma_apt_args = OmaAptArgs::builder()
            .sysroot(sysroot.to_string_lossy().to_string())
            .another_apt_options(apt_options)
            .dpkg_force_unsafe_io(force_unsafe_io)
            .dpkg_force_confnew(force_confnew)
            .force_yes(force_yes)
            .build();

        let mut apt = OmaApt::new(vec![], oma_apt_args, false, apt_config)?;

        let mut pkgs = vec![];

        let matcher = PackagesMatcher::builder()
            .cache(&apt.cache)
            .native_arch(GetArchMethod::SpecifySysroot(&sysroot))
            .build();

        for pkg in downgrade_pkgs {
            let mut f = apt
                .filter_pkgs(&[FilterMode::Default])?
                .filter(|x| x.name() == pkg);

            if let Some(pkg) = f.next() {
                if enabled_pkgs.contains(&pkg.name().to_string()) {
                    continue;
                }

                if pkg.is_installed() {
                    let pkginfo = matcher.find_candidate_by_pkgname(pkg.name())?;

                    pkgs.push(pkginfo);
                }
            }
        }

        apt.install(&pkgs, false)?;
        apt.upgrade(Upgrade::FullUpgrade)?;

        CommitChanges::builder()
            .apt(apt)
            .dry_run(dry_run)
            .request_type(SummaryType::TopicsChanged {
                add: topics_changed.opt_in,
                remove: topics_changed.opt_out,
            })
            .no_fixbroken(!no_fixbroken)
            .network_thread(config.network_thread())
            .no_progress(no_progress)
            .sysroot(sysroot.to_string_lossy().to_string())
            .fix_dpkg_status(true)
            .protect_essential(config.protect_essentials())
            .client(&HTTP_CLIENT)
            .yes(false)
            .remove_config(remove_config)
            .auth_config(&auth_config)
            .autoremove(autoremove)
            .build()
            .run()
    }
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

    // 把所有已启用的源排到最前面
    for i in enabled_topics {
        let pos = all_topics.iter().position(|x| x.name == i.name);

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
        .filter(|x| {
            (x.description.is_some() && !x.draft.is_some_and(|x| x)) || enabled_topics.contains(x)
        })
        .map(|x| TopicDisplay { topic: x })
        .collect::<Vec<_>>();

    let formatter: MultiOptionFormatter<TopicDisplay> =
        &|a| format!("Activating {} topics", a.len());
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

    let ans = MultiSelect::new(&fl!("select-topics-dialog"), display)
        .with_help_message(&fl!("tips"))
        .with_formatter(formatter)
        .with_default(&default)
        .with_page_size(page_size as usize)
        .with_render_config(render_config)
        .prompt()
        .map_err(|_| anyhow!(""))?;

    for i in &ans {
        if !enabled_topics.contains(i.topic) {
            opt_in.push(i.topic.name.to_string());
        }
    }

    for i in all_names {
        if enabled_topics.iter().any(|x| x.name == *i) && !ans.iter().any(|x| x.topic.name == *i) {
            opt_out.push(i.to_string());
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
