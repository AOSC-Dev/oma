use std::{path::Path, time::Duration};

use clap::Args;
use oma_console::pager::{exit_tui, prepare_create_tui};
use oma_pm::{
    apt::{AptConfig, OmaApt, OmaAptArgs, Upgrade},
    search::IndiciumSearch,
};
use oma_utils::{
    dbus::{InhibitTypeUnion, take_wake_lock},
    is_termux,
};
use spdlog::info;
use tui_inner::{Task, Tui as TuiInner};

use crate::{
    HTTP_CLIENT, RT,
    error::OutputError,
    find_another_oma, fl,
    subcommand::utils::{CommitChanges, Refresh, lock_oma},
    utils::{ask_continue_no_use_battery, root},
};
use crate::{
    args::CliExecuter,
    config::OmaConfig,
    config_file::{BatteryTristate, TakeWakeLockTristate},
    subcommand::utils::{auth_config, create_progress_spinner, no_check_dbus_warn},
    tui::tui_inner::PackageStatus,
    utils::{
        ExitHandle, check_battery_disabled_warn, connect_dbus_impl, is_battery,
        no_take_wake_lock_warn,
    },
};

mod state;
mod tui_inner;

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
        } = self;

        if config.dry_run {
            info!("Running in dry-run mode, Exit.");
            return Ok(ExitHandle::default());
        }

        if find_another_oma().is_ok() {
            return Err(OutputError {
                description: "".to_string(),
                source: None,
            });
        }

        if Path::new("/run/lock/oma.lock").exists() {
            return Err(OutputError {
                description: fl!("failed-to-lock-oma"),
                source: None,
            });
        }

        root()?;

        let conn = if config.no_check_dbus {
            let conn = connect_dbus_impl();

            if !is_termux() {
                match config.check_battery {
                    BatteryTristate::Ask => {
                        if let Some(conn) = conn.as_ref() {
                            ask_continue_no_use_battery(conn, false)
                        }
                    }
                    BatteryTristate::Warn => {
                        if let Some(conn) = conn.as_ref()
                            && is_battery(conn)
                        {
                            check_battery_disabled_warn();
                        }
                    }
                    BatteryTristate::Ignore => {}
                }
            }

            conn
        } else {
            None
        };

        let apt_config = AptConfig::new();
        let sysroot = &config.sysroot;
        let auth_config = auth_config(sysroot);
        let auth_config = auth_config.as_ref();

        if !no_refresh {
            let sysroot = sysroot.to_string_lossy();
            let builder = Refresh::builder()
                .client(&HTTP_CLIENT)
                .dry_run(false)
                .no_progress(config.no_progress())
                .network_thread(config.download_threads)
                .sysroot(&sysroot)
                .config(&apt_config)
                .apt_options(config.apt_options.clone())
                .maybe_auth_config(auth_config);

            #[cfg(feature = "aosc")]
            let refresh = builder.refresh_topics(!config.no_refresh_topics).build();

            #[cfg(not(feature = "aosc"))]
            let refresh = builder.build();

            refresh.run()?;
        }

        let oma_apt_args = OmaAptArgs::builder()
            .sysroot(sysroot.to_string_lossy().to_string())
            .another_apt_options(config.apt_options.clone())
            .dpkg_force_confnew(force_confnew)
            .dpkg_force_unsafe_io(force_unsafe_io)
            .force_yes(force_yes)
            .build();

        let mut apt = OmaApt::new(vec![], oma_apt_args, false, apt_config)?;

        let pb = create_progress_spinner(config.no_progress(), fl!("reading-database"));

        let (upgradable, upgradable_but_held) = apt.count_pending_upgradable_pkgs();
        let autoremove = apt.count_pending_autoremovable_pkgs();
        let installed = apt.count_installed_packages();

        let searcher = IndiciumSearch::new(&apt.cache, |n| {
            if let Some(ref pb) = pb {
                pb.inner
                    .set_message(fl!("reading-database-with-count", count = n));
            }
        })?;

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
            let _fds = if !config.no_check_dbus && !config.dry_run && !is_termux() {
                match config.take_wake_lock {
                    TakeWakeLockTristate::Yes => conn.map(|conn| {
                        RT.block_on(take_wake_lock(
                            &conn,
                            InhibitTypeUnion::all(),
                            &fl!("changing-system"),
                            "oma",
                        ))
                        .ok()
                    }),
                    TakeWakeLockTristate::Warn => {
                        no_take_wake_lock_warn();
                        None
                    }
                    TakeWakeLockTristate::Ignore => None,
                }
            } else {
                if !is_termux() {
                    no_check_dbus_warn();
                }
                None
            };

            lock_oma(sysroot)?;

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
                .dry_run(config.dry_run)
                .no_fixbroken(!fix_broken)
                .no_progress(config.no_progress())
                .sysroot(sysroot.to_string_lossy().to_string())
                .fix_dpkg_status(!no_fix_dpkg_status)
                .protect_essential(config.protect_essentials)
                .yes(false)
                .remove_config(remove_config)
                .autoremove(autoremove)
                .maybe_auth_config(auth_config)
                .network_thread(config.download_threads)
                .check_tum(upgrade)
                .is_upgrade(upgrade)
                .build()
                .run()?;
        }

        Ok(exit)
    }
}
