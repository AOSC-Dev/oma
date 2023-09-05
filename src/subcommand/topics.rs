use inquire::{
    formatter::MultiOptionFormatter,
    ui::{Color, RenderConfig, StyleSheet, Styled},
    MultiSelect,
};
use oma_console::{indicatif::ProgressBar, pb::oma_spinner};
use oma_pm::{
    apt::{AptArgsBuilder, FilterMode, OmaApt, OmaAptArgsBuilder},
    query::OmaDatabase,
};

use crate::{
    error::OutputError,
    history::SummaryType,
    utils::{create_async_runtime, dbus_check, root},
};

use super::utils::{normal_commit, refresh};
use crate::fl;
use anyhow::anyhow;
use oma_topics::TopicManager;

struct TopicChanged {
    opt_in: Vec<String>,
    opt_out: Vec<String>,
    enabled_pkgs: Vec<String>,
    downgrade_pkgs: Vec<String>,
}

pub fn execute(
    opt_in: Vec<String>,
    opt_out: Vec<String>,
    dry_run: bool,
    network_thread: usize,
    no_progress: bool,
) -> Result<i32, OutputError> {
    root()?;

    let rt = create_async_runtime()?;
    dbus_check(&rt)?;

    let topics_changed =
        rt.block_on(async move { topics_inner(opt_in, opt_out, dry_run, no_progress).await })?;

    let enabled_pkgs = topics_changed.enabled_pkgs;
    let downgrade_pkgs = topics_changed.downgrade_pkgs;

    refresh(dry_run, no_progress)?;

    let oma_apt_args = OmaAptArgsBuilder::default().build()?;
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

    normal_commit(
        apt,
        dry_run,
        SummaryType::TopicsChanged {
            add: topics_changed.opt_in,
            remove: topics_changed.opt_out,
        },
        AptArgsBuilder::default().no_progress(no_progress).build()?,
        false,
        network_thread,
        no_progress
    )?;

    Ok(0)
}

async fn topics_inner(
    mut opt_in: Vec<String>,
    mut opt_out: Vec<String>,
    dry_run: bool,
    no_progress: bool
) -> Result<TopicChanged, OutputError> {
    let mut tm = TopicManager::new().await?;

    if opt_in.is_empty() && opt_out.is_empty() {
        inquire(&mut tm, &mut opt_in, &mut opt_out, no_progress).await?;
    }

    for i in &opt_in {
        tm.add(i, dry_run, "amd64").await?;
    }

    let mut downgrade_pkgs = vec![];
    for i in &opt_out {
        downgrade_pkgs.extend(tm.remove(i, false)?);
    }

    tm.write_enabled(dry_run).await?;

    let enabled_pkgs = tm
        .enabled
        .into_iter()
        .flat_map(|x| x.packages)
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
    no_progress: bool
) -> Result<(), OutputError> {
    let pb = if !no_progress {
        let pb = ProgressBar::new_spinner();
        let (style, inv) = oma_spinner(false).unwrap();
        pb.set_style(style);
        pb.enable_steady_tick(inv);
        pb.set_message(fl!("refreshing-topic-metadata"));

        Some(pb)
    } else {
        None
    };

    let display = oma_topics::list(tm).await?;

    if let Some(pb) = pb {
        pb.finish_and_clear();
    }

    let all = tm.all.clone();
    let enabled_names = tm.enabled.iter().map(|x| &x.name).collect::<Vec<_>>();
    let all_names = all.iter().map(|x| &x.name).collect::<Vec<_>>();
    let mut default = vec![];

    for (i, c) in all_names.iter().enumerate() {
        if enabled_names.contains(c) {
            default.push(i);
        }
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

    let ans = MultiSelect::new(
        &fl!("select-topics-dialog"),
        display.iter().map(|x| x.as_str()).collect(),
    )
    .with_help_message(&fl!("tips"))
    .with_formatter(formatter)
    .with_default(&default)
    .with_page_size(20)
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
