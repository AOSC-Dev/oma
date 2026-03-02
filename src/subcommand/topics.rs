use std::{fmt::Display, path::Path, sync::atomic::Ordering};

use clap::{ArgAction, ArgGroup, Args};
use dialoguer::console::style;
use inquire::{
    formatter::MultiOptionFormatter,
    ui::{Color, RenderConfig, StyleSheet, Styled},
};
use oma_pm::{
    apt::{AptConfig, OmaApt, OmaAptArgs},
    matches::{GetArchMethod, PackagesMatcher},
    oma_apt::PkgSelectedState,
    pkginfo::OmaPackageWithoutVersion,
    utils::pkg_is_current_kernel,
};
use oma_utils::dpkg::dpkg_arch;
use once_cell::sync::OnceCell;
use spdlog::{debug, error, info, warn};
use sysinfo::System;
use tokio::task::spawn_blocking;

use crate::{
    HTTP_CLIENT, NOT_ALLOW_CTRLC, RT,
    config::OmaConfig,
    error::OutputError,
    subcommand::utils::multiselect,
    utils::{ExitHandle, ExitStatus, dbus_check, root},
};

use super::utils::{
    CommitChanges, Refresh, auth_config, create_progress_spinner, lock_oma, select_tui_display_msg,
    tui_select_list_size,
};

use crate::args::CliExecuter;

use crate::fl;
use anyhow::Context;
use oma_topics::{Topic, TopicManager};

#[derive(Debug, Args)]
#[command(group(
    ArgGroup::new("in_or_out")
        .args(&["opt_in", "opt_out"])
        .multiple(true)
))]
pub struct Topics {
    /// Enroll in one or more topic(s), delimited by space
    #[arg(long, action = ArgAction::Append, help = fl!("clap-topics-opt-in-help"))]
    opt_in: Vec<String>,
    /// Withdraw from one or more topic(s) and rollback to stable versions, delimited by space
    #[arg(long, action = ArgAction::Append, help = fl!("clap-topics-opt-out-help"))]
    opt_out: Vec<String>,
    /// Do not fix apt broken status
    #[arg(long, help = fl!("clap-no-fixbroken-help"))]
    no_fixbroken: bool,
    /// Do not fix dpkg broken status
    #[arg(long, help = fl!("clap-no-fix-dpkg-status-help"))]
    no_fix_dpkg_status: bool,
    /// Install package(s) without fsync(2)
    #[arg(
        long,
        help = &**crate::args::FORCE_UNSAGE_IO_TRANSLATE
    )]
    force_unsafe_io: bool,
    /// Ignore repository and package dependency issues
    #[arg(long, help = fl!("clap-force-yes-help"))]
    force_yes: bool,
    /// Replace configuration file(s) in the system those shipped in the package(s) to be installed (invokes `dpkg --force-confnew`)
    #[arg(long, help = fl!("clap-force-confnew-help"))]
    force_confnew: bool,
    /// Auto remove unnecessary package(s)
    #[arg(long, help = fl!("clap-autoremove-help"))]
    autoremove: bool,
    /// Display all topics on list (include draft status topics)
    #[arg(long, help = fl!("clap-topics-all-help"))]
    all: bool,
    /// Remove package(s) also remove configuration file(s), like apt purge
    #[arg(long, visible_alias = "purge", help = fl!("clap-remove-config-help"))]
    remove_config: bool,
    /// Only download dependencies, not install
    #[arg(long, short, help = fl!("clap-download-only-help"))]
    download_only: bool,
    /// Always write status to atm file and sources.list
    #[arg(long, help = fl!("clap-topics-always-write-status-help"))]
    always_write_status: bool,
    /// Only apply topics change to sources list file, not apply system change
    #[arg(long, help = fl!("clap-topics-only-apply-sources-list-help"))]
    only_apply_sources_list: bool,
    /// Bypass confirmation prompts
    ///
    /// Note that this parameter depends on the `--opt-out` or `--opt-in` parameter, otherwise it is invalid.
    #[arg(short, long, requires = "in_or_out", help = fl!("clap-yes-help"), long_help = fl!("clap-topics-yes-long-help"))]
    yes: bool,
}

struct TopicChanged {
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

        write!(f, "{s}")?;

        Ok(())
    }
}

impl CliExecuter for Topics {
    fn execute(self, config: OmaConfig) -> Result<ExitHandle, OutputError> {
        let Topics {
            mut opt_in,
            mut opt_out,
            no_fixbroken,
            force_unsafe_io,
            force_yes,
            force_confnew,
            autoremove,
            remove_config,
            all,
            no_fix_dpkg_status,
            always_write_status,
            only_apply_sources_list,
            yes,
            download_only,
        } = self;

        if !config.dry_run {
            root()?;
            lock_oma(&config.sysroot)?;
        }

        let _fds = dbus_check(false, &config)?;

        let dpkg_arch = dpkg_arch(&config.sysroot)?;
        let mut tm =
            TopicManager::new_blocking(&HTTP_CLIENT, &config.sysroot, &dpkg_arch, config.dry_run)?;

        let topics_changed = RT.block_on(topics_inner(
            &mut opt_in,
            &mut opt_out,
            config.no_progress(),
            &mut tm,
            all,
        ))?;

        let enabled_pkgs = topics_changed.enabled_pkgs;
        let downgrade_pkgs = topics_changed.downgrade_pkgs;

        debug!("enabled_pkgs = {enabled_pkgs:?}");
        debug!("downgrade_pkgs = {downgrade_pkgs:?}");

        if !opt_in.is_empty() || !opt_out.is_empty() {
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
            RT.block_on(tm.write_enabled(false))?;
        }

        let auth_config = auth_config(&config.sysroot);
        let auth_config = auth_config.as_ref();
        let apt_config = AptConfig::new();

        let code = Ok(()).and_then(|_| -> Result<ExitHandle, OutputError> {
            refresh(
                config.download_threads,
                config.no_progress(),
                config.dry_run,
                &config.sysroot,
                &apt_config,
                auth_config,
                config.apt_options.clone(),
            )?;

            if only_apply_sources_list {
                return Ok(ExitHandle::default().ring(true));
            }

            let oma_apt_args = OmaAptArgs::builder()
                .sysroot(config.sysroot.to_string_lossy().to_string())
                .another_apt_options(config.apt_options.clone())
                .dpkg_force_unsafe_io(force_unsafe_io)
                .dpkg_force_confnew(force_confnew)
                .force_yes(force_yes)
                .build();

            let mut apt = OmaApt::new(vec![], oma_apt_args, false, apt_config)?;

            let mut pkgs = vec![];
            let mut remove_pkgs = vec![];

            let matcher = PackagesMatcher::builder()
                .cache(&apt.cache)
                .native_arch(GetArchMethod::SpecifySysroot(&config.sysroot))
                .select_dbg(false)
                .build();

            let mut held_packages = vec![];

            let image_name = OnceCell::new();
            let kernel_ver = OnceCell::new();

            for pkg in downgrade_pkgs {
                let Some(pkg) = apt.cache.get(&pkg) else {
                    continue;
                };

                if enabled_pkgs.contains(&pkg.name().to_string()) || !pkg.is_installed() {
                    continue;
                }

                if pkg.selected_state() == PkgSelectedState::Hold {
                    held_packages.push(pkg.fullname(true));
                    continue;
                }

                let pkg_name = pkg.name();
                let pkginfo = matcher.find_candidate_by_pkgname(pkg_name)?;

                // linux-kernel-VER 包在关闭 topic 的时候应该直接删除
                if pkg_name.starts_with("linux-kernel-") {
                    let installed_version = pkg.installed().unwrap();
                    let installed_version = installed_version.version();
                    let candidate = pkginfo.version_raw.version();
                    let (inst_ver_no_rel, _) = installed_version
                        .split_once('-')
                        .unwrap_or((installed_version, "0"));
                    let (cand_ver_no_rel, _) =
                        candidate.split_once('-').unwrap_or((candidate, "0"));

                    if inst_ver_no_rel == cand_ver_no_rel {
                        pkgs.push(pkginfo);
                        continue;
                    }

                    let current_kernel_ver = kernel_ver.get_or_try_init(|| {
                        System::kernel_version().context("Failed to get kernel version")
                    })?;

                    debug!("kernel version = {current_kernel_ver}");

                    let is_current_kernel = pkg_is_current_kernel(
                        &config.sysroot,
                        &image_name,
                        pkg_name,
                        current_kernel_ver,
                    );

                    debug!("Deleting kernel package name = {pkg_name}");

                    // 如果现在删除的版本是正在使用的内核版本，将拒绝操作
                    if is_current_kernel {
                        return Err(OutputError {
                            description: fl!(
                                "not-allow-delete-using-kernel",
                                ver = current_kernel_ver
                            ),
                            source: None,
                        });
                    }

                    remove_pkgs.push(OmaPackageWithoutVersion {
                        raw_pkg: unsafe { pkg.unique() },
                    });

                    continue;
                }

                pkgs.push(pkginfo);
            }

            for pkg in enabled_pkgs {
                let Some(pkg) = apt.cache.get(&pkg) else {
                    continue;
                };

                if !pkg.is_installed() {
                    continue;
                }

                if pkg.selected_state() == PkgSelectedState::Hold {
                    held_packages.push(pkg.fullname(true));
                    continue;
                }

                let pkginfo = matcher.find_candidate_by_pkgname(pkg.name())?;

                pkgs.push(pkginfo);
            }

            apt.install(&pkgs, false)?;
            apt.remove(remove_pkgs, remove_config, !autoremove)?;

            let code = CommitChanges::builder()
                .apt(apt)
                .dry_run(config.dry_run)
                .no_fixbroken(!no_fixbroken)
                .no_progress(config.no_progress())
                .sysroot(config.sysroot.to_string_lossy().to_string())
                .fix_dpkg_status(!no_fix_dpkg_status)
                .protect_essential(config.protect_essentials)
                .yes(yes)
                .remove_config(remove_config)
                .autoremove(autoremove)
                .network_thread(config.download_threads)
                .maybe_auth_config(auth_config)
                .check_tum(true)
                .topics_enabled(opt_in)
                .topics_disabled(opt_out)
                .download_only(download_only)
                .build()
                .run()?;

            if !held_packages.is_empty() {
                let count = held_packages.len();
                info!("{}", fl!("topics-held-tips", count = count));
                debug!("held packages: {held_packages:?}");
            }

            Ok(code)
        });

        match code {
            Ok(ref x) => {
                if x.get_status() != ExitStatus::Success && !always_write_status {
                    NOT_ALLOW_CTRLC.store(true, Ordering::Relaxed);
                    error!("{}", fl!("topics-unchanged"));
                    revert_sources_list(&tm)?;
                    RT.block_on(tm.write_enabled(true))?;
                    refresh(
                        config.download_threads,
                        config.no_progress(),
                        config.dry_run,
                        &config.sysroot,
                        &AptConfig::new(),
                        auth_config,
                        config.apt_options.clone(),
                    )?;
                }
            }
            Err(e) => {
                if !always_write_status && !download_only {
                    error!("{}", fl!("topics-unchanged"));
                    revert_sources_list(&tm)?;
                    RT.block_on(tm.write_enabled(true))?;
                }
                return Err(e);
            }
        };

        code
    }
}

fn refresh<'a>(
    network_threads: usize,
    no_progress: bool,
    dry_run: bool,
    sysroot: &'a Path,
    apt_config: &'a AptConfig,
    auth_config: Option<&'a apt_auth_config::AuthConfig>,
    apt_options: Vec<String>,
) -> Result<(), OutputError> {
    Refresh::builder()
        .client(&HTTP_CLIENT)
        .dry_run(dry_run)
        .no_progress(no_progress)
        .network_thread(network_threads)
        .sysroot(&sysroot.to_string_lossy())
        .refresh_topics(true)
        .config(apt_config)
        .maybe_auth_config(auth_config)
        .apt_options(apt_options)
        .build()
        .run()
}

fn revert_sources_list(tm: &TopicManager<'_>) -> Result<(), OutputError> {
    RT.block_on(tm.write_sources_list(
        &fl!("do-not-edit-topic-sources-list"),
        true,
        async |topic, mirror| {
            warn!(
                "{}",
                fl!("topic-not-in-mirror", topic = topic, mirror = mirror)
            );
            warn!("{}", fl!("skip-write-mirror"));
        },
    ))?;
    Ok(())
}

async fn topics_inner(
    opt_in: &mut Vec<String>,
    opt_out: &mut Vec<String>,
    no_progress: bool,
    tm: &mut TopicManager<'_>,
    all: bool,
) -> Result<TopicChanged, OutputError> {
    refresh_topics(no_progress, tm).await?;

    let all_topics = tm
        .available_topics()
        .into_iter()
        .map(|x| x.to_owned())
        .collect::<Vec<_>>();

    let enabled_topics = Box::from(tm.enabled_topics());

    if opt_in.is_empty() && opt_out.is_empty() {
        (*opt_in, *opt_out) =
            spawn_blocking(move || select_prompt(all_topics, &enabled_topics, all))
                .await
                .unwrap()?;
    }

    for i in opt_in {
        tm.add(i)?;
    }

    let mut downgrade_pkgs = vec![];
    for i in opt_out {
        let removed_topic = tm.remove(i)?;
        downgrade_pkgs.extend(removed_topic.packages);
    }

    let enabled_pkgs = tm
        .enabled_topics()
        .iter()
        .flat_map(|x| x.packages.clone())
        .collect::<Vec<_>>();

    Ok(TopicChanged {
        enabled_pkgs,
        downgrade_pkgs,
    })
}

fn select_prompt(
    all_topics: Vec<Topic>,
    enabled_topics: &[Topic],
    all: bool,
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
            all || ((x.description.is_some() && !x.draft.is_some_and(|x| x))
                || enabled_topics.contains(x))
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

    let ans = multiselect(
        &fl!("select-topics-dialog"),
        display,
        formatter,
        render_config,
        page_size,
        default,
    )?;

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
    let pb = create_progress_spinner(no_progress, fl!("refreshing-topic-metadata"));

    tm.refresh().await?;
    tm.remove_closed_topics()?;
    tm.write_enabled(false).await?;

    if let Some(pb) = pb {
        pb.inner.finish_and_clear();
    }

    Ok(())
}
