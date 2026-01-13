use std::path::PathBuf;

use clap::Args;
use oma_pm::apt::{AptConfig, OmaApt, OmaAptArgs};

use crate::{
    config::Config,
    error::OutputError,
    fl,
    utils::{ExitHandle, dbus_check, root},
};

use super::utils::{CommitChanges, auth_config, lock_oma};
use crate::args::CliExecuter;

#[derive(Debug, Args)]
pub struct FixBroken {
    /// Do not fix dpkg broken status
    #[arg(short, long, help = fl!("clap-no-fix-dpkg-status-help"))]
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
    /// Remove package(s) also remove configuration file(s), like apt purge
    #[arg(long, visible_alias = "purge", help = fl!("clap-remove-config-help"))]
    remove_config: bool,
    /// Run oma in "dry-run" mode. Useful for testing changes and operations without making changes to the system
    #[arg(from_global, help = fl!("clap-dry-run-help"), long_help = fl!("clap-dry-run-long-help"))]
    dry_run: bool,
    /// Run oma do not check dbus
    #[arg(from_global, help = fl!("clap-no-check-dbus-help"))]
    no_check_dbus: bool,
    /// Run oma do not check battery status
    #[arg(from_global, help = fl!("clap-no-check-battery-help"))]
    no_check_battery: bool,
    /// Run oma do not take wake lock
    #[arg(from_global, help = fl!("clap-no-take-wake-lock-help"))]
    no_take_wake_lock: bool,
    /// Set sysroot target directory
    #[arg(from_global, help = fl!("clap-sysroot-help"))]
    sysroot: PathBuf,
    /// Set apt options
    #[arg(from_global, help = fl!("clap-apt-options-help"))]
    apt_options: Vec<String>,
    /// Setup download threads (default as 4)
    #[arg(from_global, help = fl!("clap-download-threads-help"))]
    download_threads: Option<usize>,
}

impl CliExecuter for FixBroken {
    fn execute(self, config: &Config, no_progress: bool) -> Result<ExitHandle, OutputError> {
        root()?;

        let FixBroken {
            force_unsafe_io,
            force_yes,
            force_confnew,
            autoremove,
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

        lock_oma(&sysroot)?;

        let mut _fds = dbus_check(
            false,
            config,
            no_check_dbus,
            dry_run,
            no_take_wake_lock,
            no_check_battery,
        )?;

        let auth_config = auth_config(&sysroot);
        let auth_config = auth_config.as_ref();

        let oma_apt_args = OmaAptArgs::builder()
            .sysroot(sysroot.to_string_lossy().to_string())
            .dpkg_force_unsafe_io(force_unsafe_io)
            .force_yes(force_yes)
            .dpkg_force_confnew(force_confnew)
            .another_apt_options(apt_options)
            .build();
        let apt = OmaApt::new(vec![], oma_apt_args, dry_run, AptConfig::new())?;

        CommitChanges::builder()
            .apt(apt)
            .dry_run(dry_run)
            .no_fixbroken(false)
            .is_fixbroken(true)
            .no_progress(no_progress)
            .sysroot(sysroot.to_string_lossy().to_string())
            .fix_dpkg_status(!no_fix_dpkg_status)
            .protect_essential(config.protect_essentials())
            .yes(false)
            .autoremove(autoremove)
            .remove_config(remove_config)
            .maybe_auth_config(auth_config)
            .network_thread(download_threads.unwrap_or_else(|| config.network_thread()))
            .build()
            .run()
    }
}
