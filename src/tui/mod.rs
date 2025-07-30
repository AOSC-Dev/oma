use std::{
    path::{Path, PathBuf},
    time::Duration,
};

use clap::Args;
use oma_console::pager::{exit_tui, prepare_create_tui};
use oma_pm::{
    apt::{AptConfig, OmaApt, OmaAptArgs, Upgrade},
    search::IndiciumSearch,
};
use oma_utils::dbus::take_wake_lock;
use tracing::info;
use tui_inner::{Task, Tui as TuiInner};

use crate::{
    GlobalOptions,
    args::CliExecuter,
    subcommand::utils::{auth_config, create_progress_spinner, no_check_dbus_warn},
    utils::{check_battery_disabled_warn, connect_dbus_impl, no_take_wake_lock_warn},
};
use crate::{
    HTTP_CLIENT, RT,
    config::Config,
    error::OutputError,
    find_another_oma, fl,
    subcommand::utils::{CommitChanges, Refresh, lock_oma},
    utils::{check_battery, root},
};

mod state;
mod tui_inner;

#[derive(Debug, Args)]
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
    #[cfg(feature = "aosc")]
    /// Do not refresh topics manifest.json file
    #[arg(long)]
    no_refresh_topics: bool,
    /// Remove package(s) also remove configuration file(s), like apt purge
    #[arg(long, visible_alias = "purge")]
    remove_config: bool,
    /// Run oma in "dry-run" mode. Useful for testing changes and operations without making changes to the system
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
    /// Setup download threads (default as 4)
    #[arg(from_global)]
    download_threads: Option<usize>,
    /// Run oma do not check battery status
    #[arg(from_global)]
    no_check_battery: bool,
    /// Run oma do not check battery status
    #[arg(from_global)]
    no_take_wake_lock: bool,
}

impl From<&GlobalOptions> for Tui {
    fn from(value: &GlobalOptions) -> Self {
        Self {
            fix_broken: Default::default(),
            force_unsafe_io: Default::default(),
            no_refresh: Default::default(),
            force_yes: Default::default(),
            force_confnew: Default::default(),
            #[cfg(feature = "aosc")]
            no_refresh_topics: Default::default(),
            remove_config: Default::default(),
            no_fix_dpkg_status: Default::default(),
            dry_run: value.dry_run,
            no_check_dbus: value.no_check_dbus,
            sysroot: value.sysroot.clone(),
            apt_options: value.apt_options.clone(),
            download_threads: value.download_threads,
            no_check_battery: value.no_check_battery,
            no_take_wake_lock: value.no_take_wake_lock,
        }
    }
}

impl CliExecuter for Tui {
    fn execute(self, config: &Config, no_progress: bool) -> Result<i32, OutputError> {
        let Tui {
            fix_broken,
            force_unsafe_io,
            no_refresh,
            force_yes,
            force_confnew,
            #[cfg(feature = "aosc")]
            no_refresh_topics,
            remove_config,
            dry_run,
            no_check_dbus,
            sysroot,
            apt_options,
            no_fix_dpkg_status,
            download_threads,
            no_check_battery,
            no_take_wake_lock,
        } = self;

        if dry_run {
            info!("Running in dry-run mode, Exit.");
            return Ok(0);
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

        let conn = if !no_check_dbus && !config.no_check_dbus() {
            let conn = connect_dbus_impl();

            if let Some(conn) = &conn {
                if !no_check_battery && !config.no_check_battery() {
                    check_battery(conn, false);
                } else {
                    check_battery_disabled_warn();
                }
            }

            conn
        } else {
            None
        };

        let apt_config = AptConfig::new();
        let auth_config = auth_config(&sysroot);
        let auth_config = auth_config.as_ref();

        if !no_refresh {
            let sysroot = sysroot.to_string_lossy();
            let builder = Refresh::builder()
                .client(&HTTP_CLIENT)
                .dry_run(false)
                .no_progress(no_progress)
                .network_thread(download_threads.unwrap_or_else(|| config.network_thread()))
                .sysroot(&sysroot)
                .config(&apt_config)
                .maybe_auth_config(auth_config);

            #[cfg(feature = "aosc")]
            let refresh = builder
                .refresh_topics(!no_refresh_topics && !config.no_refresh_topics())
                .build();

            #[cfg(not(feature = "aosc"))]
            let refresh = builder.build();

            refresh.run()?;
        }

        let oma_apt_args = OmaAptArgs::builder()
            .sysroot(sysroot.to_string_lossy().to_string())
            .another_apt_options(apt_options)
            .dpkg_force_confnew(force_confnew)
            .dpkg_force_unsafe_io(force_unsafe_io)
            .force_yes(force_yes)
            .build();

        let mut apt = OmaApt::new(vec![], oma_apt_args, false, apt_config)?;

        let pb = create_progress_spinner(no_progress, fl!("reading-database"));

        let upgradable = apt.count_pending_upgradable_pkgs()?;
        let autoremovable = apt.count_pending_autoremovable_pkgs();
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

        let tui = TuiInner::new(&apt, installed, upgradable, autoremovable, searcher);

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

        let mut code = 0;

        if execute_apt {
            let _fds = if !no_check_dbus && !config.no_check_dbus() && !dry_run {
                if !config.no_take_wake_lock() && no_take_wake_lock {
                    conn.map(|conn| {
                        RT.block_on(take_wake_lock(&conn, &fl!("changing-system"), "oma"))
                    })
                } else {
                    no_take_wake_lock_warn();
                    None
                }
            } else {
                no_check_dbus_warn();
                None
            };

            lock_oma()?;

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

            code = CommitChanges::builder()
                .apt(apt)
                .dry_run(dry_run)
                .no_fixbroken(!fix_broken)
                .no_progress(no_progress)
                .sysroot(sysroot.to_string_lossy().to_string())
                .fix_dpkg_status(!no_fix_dpkg_status)
                .protect_essential(config.protect_essentials())
                .yes(false)
                .remove_config(remove_config)
                .autoremove(autoremove)
                .maybe_auth_config(auth_config)
                .network_thread(download_threads.unwrap_or_else(|| config.network_thread()))
                .check_update(upgrade)
                .build()
                .run()?;
        }

        Ok(code)
    }
}
