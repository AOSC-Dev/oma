use std::time::Duration;

use clap::Args;
use oma_console::pager::{exit_tui, prepare_create_tui};
use oma_pm::{
    apt::{AptConfig, OmaApt, OmaAptArgs, Upgrade},
    search::{IndiciumSearch, SearchType},
};
use render::{Task, Tui as TuiInner};
use spdlog::info;

use crate::{
    args::CliExecuter,
    config::OmaConfig,
    exit_handle::ExitHandle,
    subcommand::utils::{auth_config, create_progress_spinner},
    tui::render::PackageStatus,
};
use crate::{
    core::{commit_changes::CommitChanges, refresh::Refresh},
    dbus::dbus_check,
    error::OutputError,
    fl,
    root::root,
    subcommand::utils::lock_oma,
};

mod key_binding;
mod render;
mod state;
mod window;

#[derive(Debug, Args, Default)]
pub struct Tui {
    /// Fix apt broken status
    #[arg(short, long)]
    fix_broken: bool,
    /// Do not fix dpkg broken status
    #[arg(short, long)]
    no_fix_dpkg_status: bool,
    /// Install package(s) without fsync(2)
    #[arg(long)]
    force_unsafe_io: bool,
    /// Do not refresh repository metadata
    #[arg(long)]
    no_refresh: bool,
    /// Ignore repository and package dependency issues
    #[arg(long)]
    force_yes: bool,
    /// Replace configuration file(s) in the system those shipped in the package(s) to be installed (invokes `dpkg --force-confnew`)
    #[arg(long)]
    force_confnew: bool,
    /// Remove package(s) also remove configuration file(s), like apt purge
    #[arg(long, visible_alias = "purge")]
    remove_config: bool,
    /// Do not clean local package cache
    #[arg(long, help = fl!("clap-noclean-help"), env = "OMA_NO_CLEAN", value_parser = clap::builder::FalseyValueParser::new())]
    no_clean: bool,
}

impl CliExecuter for Tui {
    fn execute(self, config: OmaConfig) -> Result<ExitHandle, OutputError> {
        let Tui {
            fix_broken,
            force_unsafe_io,
            no_refresh,
            force_yes,
            force_confnew,
            remove_config,
            no_fix_dpkg_status,
            no_clean,
        } = self;

        if config.dry_run {
            info!("Running in dry-run mode, Exit.");
            return Ok(ExitHandle::default());
        }

        root()?;
        let _lock_fd = lock_oma(&config.sysroot)?;
        let _fds = dbus_check(false, &config)?;

        let apt_config = AptConfig::new();
        let sysroot = &config.sysroot;
        let auth_config = auth_config(sysroot);
        let auth_config = auth_config.as_ref();

        if !no_refresh {
            Refresh::builder()
                .config(&config)
                .apt_config(&apt_config)
                .maybe_auth_config(auth_config)
                .build()
                .run()?;
        }

        let oma_apt_args = OmaAptArgs::builder()
            .sysroot(sysroot.to_string_lossy().to_string())
            .another_apt_options(&config.apt_options)
            .dpkg_force_confnew(force_confnew)
            .dpkg_force_unsafe_io(force_unsafe_io)
            .force_yes(force_yes)
            .build();

        let mut apt = OmaApt::new(vec![], oma_apt_args, false, apt_config)?;

        let pb = create_progress_spinner(config.no_progress(), fl!("reading-database"));

        let (upgradable, upgradable_but_held) = apt.count_pending_upgradable_pkgs();
        let autoremove = apt.count_pending_autoremovable_pkgs();
        let installed = apt.count_installed_packages();

        let searcher = IndiciumSearch::new(
            &apt.cache,
            |n| {
                if let Some(ref pb) = pb {
                    pb.inner
                        .set_message(fl!("reading-database-with-count", count = n));
                }
            },
            SearchType::Live,
        )?;

        if let Some(pb) = pb {
            pb.inner.finish_and_clear();
        }

        let mut terminal = prepare_create_tui().map_err(|e| OutputError {
            description: "BUG: Failed to create crossterm instance".to_string(),
            source: Some(Box::new(e)),
        })?;

        let tui = TuiInner::new(
            &apt,
            PackageStatus {
                installed,
                upgradable,
                upgradable_but_held,
                autoremove,
            },
            searcher,
        );

        let Task {
            execute_apt,
            install,
            remove,
            upgrade,
            autoremove,
        } = tui.run(&mut terminal, Duration::from_millis(250)).unwrap();

        exit_tui(&mut terminal).map_err(|e| OutputError {
            description: "BUG: Failed to exit tui".to_string(),
            source: Some(Box::new(e)),
        })?;

        let mut exit = ExitHandle::default();

        if execute_apt {
            if upgrade {
                apt.upgrade(Upgrade::FullUpgrade)?;
            }

            apt.install(&install, false)?;
            apt.remove(
                remove
                    .iter()
                    .flat_map(|x| x.into_oma_package_without_version()),
                false,
                !autoremove,
            )?;

            exit = CommitChanges::builder()
                .apt(apt)
                .no_fixbroken(!fix_broken)
                .fix_dpkg_status(!no_fix_dpkg_status)
                .yes(false)
                .remove_config(remove_config)
                .autoremove(autoremove)
                .maybe_auth_config(auth_config)
                .check_tum(upgrade)
                .is_upgrade(upgrade)
                .config(&config)
                .no_clean(no_clean)
                .build()
                .run()?;
        }

        Ok(exit)
    }
}
