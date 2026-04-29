use clap::Args;
use oma_pm::apt::{AptConfig, OmaApt, OmaAptArgs};

use crate::{
    config::OmaConfig, core::commit_changes::CommitChanges, dbus::dbus_check, error::OutputError,
    exit_handle::ExitHandle, fl, root::root,
};

use super::utils::{auth_config, lock_oma};
use crate::args::CliExecuter;

#[derive(Debug, Args)]
pub struct FixBroken {
    /// Do not fix dpkg broken status
    #[arg(short, long, help = fl!("clap-no-fix-dpkg-status-help"))]
    no_fix_dpkg_status: bool,
    /// Install package(s) without fsync(2)
    #[arg(
        long,
        help = &**crate::args::FORCE_UNSAFE_IO_TRANSLATE
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
    /// Do not clean local package cache
    #[arg(long, help = fl!("clap-noclean-help"), env = "OMA_NO_CLEAN", value_parser = clap::builder::FalseyValueParser::new())]
    no_clean: bool,
}

impl CliExecuter for FixBroken {
    fn execute(self, config: OmaConfig) -> Result<ExitHandle, OutputError> {
        root()?;

        let FixBroken {
            force_unsafe_io,
            force_yes,
            force_confnew,
            autoremove,
            remove_config,
            no_fix_dpkg_status,
            no_clean,
        } = self;

        let _lock_fd = lock_oma(&config.sysroot)?;

        let mut _fds = dbus_check(false, &config)?;

        let auth_config = auth_config(&config.sysroot);

        let oma_apt_args = OmaAptArgs::builder()
            .sysroot(config.sysroot.to_string_lossy().to_string())
            .dpkg_force_unsafe_io(force_unsafe_io)
            .force_yes(force_yes)
            .dpkg_force_confnew(force_confnew)
            .another_apt_options(&config.apt_options)
            .build();
        let apt = OmaApt::new(vec![], oma_apt_args, config.dry_run, AptConfig::new())?;

        CommitChanges::builder()
            .apt(apt)
            .no_fixbroken(false)
            .is_fixbroken(true)
            .fix_dpkg_status(!no_fix_dpkg_status)
            .yes(false)
            .autoremove(autoremove)
            .remove_config(remove_config)
            .maybe_auth_config(auth_config.as_ref().as_ref())
            .config(&config)
            .no_clean(no_clean)
            .build()
            .run()
    }
}
