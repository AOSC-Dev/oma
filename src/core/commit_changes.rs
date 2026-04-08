use std::{fs, path::Path, sync::atomic::Ordering, thread};

use ahash::{HashMap, HashSet};
use apt_auth_config::AuthConfig;
use bon::Builder;
use chrono::Local;
use dialoguer::{Confirm, theme::ColorfulTheme};
use flume::unbounded;
use oma_console::{indicatif::HumanBytes, pager::PagerExit, print::Action};
use oma_history::{DATABASE_PATH, HistoryInfo};
use oma_pm::{
    CommitConfig,
    apt::{
        AptConfig, InstallEntry, InstallProgressOpt, OmaApt, OmaAptArgs, OmaAptError, RemoveEntry,
    },
    oma_apt::PackageSort,
    sort::SummarySort,
};
use spdlog::{debug, error, info, warn};

#[cfg(feature = "aosc")]
use crate::utils::get_lists_dir;
use crate::{
    NOT_ALLOW_CTRLC, color_formatter,
    config::OmaConfig,
    core::space_tips,
    error::OutputError,
    exit_handle::{ExitHandle, ExitStatus},
    fl,
    install_progress::{NoInstallProgressManager, OmaInstallProgressManager, osc94_progress},
    lang::{DEFAULT_LANGUAGE, SYSTEM_LANG},
    msg,
    pb::{NoProgressBar, OmaMultiProgressBar, RenderPackagesDownloadProgress},
    subcommand::{
        remove::ask_user_do_as_i_say,
        utils::{create_progress_spinner, download_message},
    },
    success,
    table::table_for_install_pending,
};

#[derive(Builder)]
pub(crate) struct CommitChanges<'a> {
    apt: OmaApt,
    #[builder(default)]
    is_fixbroken: bool,
    #[builder(default)]
    is_undo: bool,
    #[builder(default = true)]
    no_fixbroken: bool,
    #[builder(default = true)]
    fix_dpkg_status: bool,
    #[builder(default)]
    yes: bool,
    #[builder(default)]
    remove_config: bool,
    #[builder(default)]
    autoremove: bool,
    auth_config: Option<&'a AuthConfig>,
    #[builder(default)]
    check_tum: bool,
    #[builder(default)]
    topics_enabled: Vec<String>,
    #[builder(default)]
    topics_disabled: Vec<String>,
    #[builder(default)]
    download_only: bool,
    #[builder(default)]
    is_upgrade: bool,
    config: &'a OmaConfig,
}

impl CommitChanges<'_> {
    pub fn run(self) -> Result<ExitHandle, OutputError> {
        let CommitChanges {
            mut apt,
            is_fixbroken,
            is_undo,
            no_fixbroken,
            fix_dpkg_status,
            yes,
            remove_config,
            autoremove,
            auth_config,
            check_tum,
            topics_enabled,
            topics_disabled,
            download_only,
            is_upgrade,
            config,
        } = self;

        fix_broken(
            &mut apt,
            no_fixbroken,
            config.no_progress(),
            fix_dpkg_status,
            remove_config,
            autoremove,
            is_upgrade,
        )?;

        let dry_run = config.dry_run;

        let op = apt.summary(
            SummarySort::default().names().operation(),
            |pkg| {
                if dry_run {
                    true
                } else if config.protect_essentials {
                    false
                } else {
                    ask_user_do_as_i_say(pkg).unwrap_or(false)
                }
            },
            |features| {
                if dry_run {
                    true
                } else {
                    handle_features(features, config.protect_essentials).unwrap_or(false)
                }
            },
        )?;

        debug!("{op}");

        apt.check_disk_size(&op)?;

        let install = &op.install;
        let remove = &op.remove;
        let disk_size = &op.disk_size_delta;
        let (ar_count, ar_size) = op.autoremovable;
        let (suggest, recommend) = (&op.suggest, &op.recommend);

        if is_nothing_to_do(install, remove, !no_fixbroken) {
            autoremovable_tips(ar_count, ar_size);
            return Ok(ExitHandle::default().ring(true));
        }

        apt.init_dbus_status()?;

        if check_tum {
            #[cfg(feature = "aosc")]
            let tum = oma_tum::get_tum(get_lists_dir(&AptConfig::new()))?;

            #[cfg(feature = "aosc")]
            let matches_tum = Some(oma_tum::get_matches_tum(&tum, &op));

            #[cfg(not(feature = "aosc"))]
            let matches_tum = None;

            match table_for_install_pending(
                install,
                remove,
                *disk_size,
                matches_tum,
                !yes,
                dry_run,
                config.yn_mode,
            )? {
                PagerExit::NormalExit => {}
                x => return Ok(ExitHandle::default().status(ExitStatus::Other(x.into()))),
            }
        } else {
            match table_for_install_pending(
                install,
                remove,
                *disk_size,
                None,
                !yes,
                dry_run,
                config.yn_mode,
            )? {
                PagerExit::NormalExit => {}
                x => return Ok(ExitHandle::default().status(ExitStatus::Other(x.into()))),
            }
        }

        let start_time = Local::now().timestamp();

        let (tx, rx) = unbounded();

        let no_progress = config.no_progress();

        thread::spawn(move || {
            let mut pb: Box<dyn RenderPackagesDownloadProgress> = if no_progress {
                Box::new(NoProgressBar::default())
            } else {
                Box::new(OmaMultiProgressBar::default())
            };
            pb.render_progress(&rx, false);
        });

        let res = apt.commit(
            InstallProgressOpt::TermLike(if config.no_progress() {
                Box::new(NoInstallProgressManager)
            } else {
                Box::new(OmaInstallProgressManager::new(yes))
            }),
            &op,
            config.http_client()?,
            CommitConfig {
                network_thread: Some(config.download_threads),
                auth_config,
                download_only,
            },
            download_message(),
            async |event| {
                if let Err(e) = tx.send_async(event).await {
                    debug!("Send progress channel got error: {}; maybe check archive work still in progress", e);
                }
            },
        );

        osc94_progress(100.0, true);

        let history = oma_history::History::new(
            config.sysroot.join(DATABASE_PATH),
            true,
            self.config.dry_run,
        );

        match res {
            Ok(_) => {
                NOT_ALLOW_CTRLC.store(true, Ordering::Relaxed);

                let apt = OmaApt::new(
                    vec![],
                    OmaAptArgs::builder()
                        .sysroot(config.sysroot.to_string_lossy().to_string())
                        .build(),
                    false,
                    AptConfig::new(),
                )?;

                if download_only {
                    space_tips(&apt, &config.sysroot);

                    let len = op.install.len();
                    let path = apt.get_archive_dir().to_string_lossy();

                    success!(
                        "{}",
                        fl!("successfully-download-to-path", len = len, path = path)
                    );

                    return Ok(ExitHandle::default().ring(true));
                }

                write_oma_installed_status(&apt, &config.sysroot)?;
                autoremovable_tips(ar_count, ar_size);

                let mut history = history?;
                history.write(HistoryInfo {
                    summary: &op,
                    start_time,
                    success: true,
                    is_fix_broken: is_fixbroken,
                    is_undo,
                    topics_enabled,
                    topics_disabled,
                })?;

                history_success_tips(dry_run);
                display_suggest_tips(suggest, recommend);
                space_tips(&apt, &config.sysroot);

                Ok(ExitHandle::default().ring(true))
            }
            Err(e) => {
                if let OmaAptError::FailedToDownload(_) = e {
                    return Err(e.into());
                }

                if download_only {
                    return Err(e.into());
                }

                let apt = OmaApt::new(
                    vec![],
                    OmaAptArgs::builder().build(),
                    false,
                    AptConfig::new(),
                )?;

                NOT_ALLOW_CTRLC.store(true, Ordering::Relaxed);
                undo_tips();

                let mut history = history?;
                history.write(HistoryInfo {
                    summary: &op,
                    start_time,
                    success: false,
                    is_fix_broken: is_fixbroken,
                    is_undo,
                    topics_enabled,
                    topics_disabled,
                })?;

                space_tips(&apt, &config.sysroot);
                Err(e.into())
            }
        }
    }
}

fn fix_broken(
    apt: &mut OmaApt,
    no_fixbroken: bool,
    no_progress: bool,
    fix_dpkg_status: bool,
    remove_config: bool,
    autoremove: bool,
    is_upgrade: bool,
) -> Result<(), OutputError> {
    let pb = create_progress_spinner(no_progress, fl!("resolving-dependencies"));

    let res = Ok(()).and_then(|_| -> Result<(), OmaAptError> {
        let solver = apt.config.find("APT::Solver", "internal");

        if autoremove {
            apt.autoremove(remove_config)?;
        }

        if !(no_fixbroken || (solver == "3.0" && is_upgrade)) {
            apt.fix_resolver_broken();
        }

        if fix_dpkg_status {
            let (needs_reconfigure, needs_retrigger) = apt.is_needs_fix_dpkg_status()?;
            if needs_retrigger || needs_reconfigure {
                if let Some(ref pb) = pb {
                    pb.inner.finish_and_clear()
                }
                info!("{}", fl!("fixing-status"));
                apt.fix_dpkg_status(needs_reconfigure, needs_retrigger)?;
            }
        }

        if solver == "3.0" && is_upgrade {
            return Ok(());
        }

        apt.resolve(no_fixbroken, remove_config)?;

        Ok(())
    });

    if let Some(pb) = pb {
        pb.inner.finish_and_clear();
    }

    res?;

    Ok(())
}

fn is_nothing_to_do(install: &[InstallEntry], remove: &[RemoveEntry], fix_broken: bool) -> bool {
    if install.is_empty() && remove.is_empty() {
        if fix_broken {
            success!("{}", fl!("success"));
        } else {
            success!("{}", fl!("no-need-to-do-anything"));
        }

        return true;
    }

    false
}

fn autoremovable_tips(count: u64, total_size: u64) {
    if count == 0 {
        return;
    }

    let total_size = HumanBytes(total_size).to_string();
    let cmd1 = color_formatter()
        .color_str("oma list --autoremovable", Action::Emphasis)
        .to_string();
    let cmd2 = color_formatter()
        .color_str("oma mark manual <packages>", Action::Note)
        .to_string();
    let cmd3 = color_formatter()
        .color_str("oma autoremove", Action::Secondary)
        .to_string();
    let count = color_formatter()
        .color_str(count, Action::Secondary)
        .to_string();
    let total_size = color_formatter()
        .color_str(total_size, Action::Secondary)
        .to_string();
    info!(
        "{}",
        fl!(
            "autoremove-tips-1",
            count = count,
            size = total_size,
            cmd = cmd1
        )
    );
    info!("{}", fl!("autoremove-tips-2", cmd1 = cmd2, cmd2 = cmd3));
}

fn write_oma_installed_status(apt: &OmaApt, sysroot: impl AsRef<Path>) -> anyhow::Result<()> {
    let status_file = sysroot.as_ref().join("var/lib/oma/installed");
    let status_file_manual = sysroot.as_ref().join("var/lib/oma/installed-manual");
    let parent = status_file.parent().unwrap();

    if !parent.is_dir() {
        fs::create_dir_all(parent)?;
    }

    let pkgs = apt
        .cache
        .packages(&PackageSort::default().installed())
        .map(|x| x.fullname(false))
        .collect::<Vec<_>>();

    let manual_pkgs = apt
        .cache
        .packages(&PackageSort::default().manually_installed())
        .map(|x| x.fullname(false))
        .collect::<Vec<_>>();

    if status_file.exists() {
        fs::copy(&status_file, parent.join("installed-old"))?;
    }

    if status_file_manual.exists() {
        fs::copy(&status_file_manual, parent.join("installed-manual-old"))?;
    }

    fs::write(status_file, pkgs.join("\n"))?;
    fs::write(status_file_manual, manual_pkgs.join("\n"))?;

    Ok(())
}

fn history_success_tips(dry_run: bool) {
    if !dry_run {
        success!("{}", fl!("history-tips-1"));
        undo_tips();
    }
}

fn undo_tips() {
    let cmd = color_formatter().color_str("oma undo", Action::Emphasis);
    info!("{}", fl!("history-tips-2", cmd = cmd.to_string()));
}

fn display_suggest_tips(suggest: &[(String, String)], recommend: &[(String, String)]) {
    let suggest_and_recommends = suggest.iter().chain(recommend).collect::<Vec<_>>();

    if !suggest_and_recommends.is_empty() {
        info!("{}", fl!("suggest"));
        for (pkg, desc) in suggest_and_recommends {
            msg!("{}: {}", pkg, desc);
        }
    }
}

fn handle_features(features: &HashSet<Box<str>>, protect: bool) -> Result<bool, OutputError> {
    debug!("{:?}", features);

    let theme = ColorfulTheme::default();

    let features = match format_features(features) {
        Ok(v) => v,
        Err(e) => {
            warn!("{e}");

            if protect {
                error!("{}", fl!("features-without-value"));
                return Ok(false);
            }

            warn!("{}", fl!("features-without-value"));

            let delete = Confirm::with_theme(&theme)
                .with_prompt(fl!("features-continue-prompt"))
                .default(false)
                .interact()
                .map_err(|_| anyhow::anyhow!(""))?;

            return Ok(delete);
        }
    };

    if protect {
        error!("{}", fl!("features-tips-1"));
        msg!("\n{}\n", features);
        return Ok(false);
    }

    warn!("{}", fl!("features-tips-1"));
    msg!("\n{}\n", features);

    let delete = Confirm::with_theme(&theme)
        .with_prompt(fl!("features-continue-prompt"))
        .default(false)
        .interact()
        .map_err(|_| anyhow::anyhow!(""))?;

    Ok(delete)
}

pub fn format_features(features: &HashSet<Box<str>>) -> anyhow::Result<String> {
    let mut res = String::new();
    let features_data = std::fs::read_to_string("/usr/share/aosc-os/features.toml")?;
    let features_data: HashMap<Box<str>, HashMap<Box<str>, Box<str>>> =
        toml::from_str(&features_data)?;

    let lang = &*SYSTEM_LANG;

    for (index, f) in features.iter().enumerate() {
        if let Some(v) = features_data.get(f) {
            let text = v
                .get(lang.as_str())
                .unwrap_or_else(|| v.get(DEFAULT_LANGUAGE).unwrap());

            res.push_str(&format!("  * {text}"));
        } else {
            res.push_str(&format!("  * {f}"));
        }

        if index != features.len() - 1 {
            res.push('\n');
        }
    }

    Ok(res)
}
