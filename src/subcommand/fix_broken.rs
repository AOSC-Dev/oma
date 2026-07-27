use clap::Args;
use oma_pm::apt::{OmaApt, OmaAptArgs};

use crate::{
    config::OmaConfig, core::operation_pipeline::Pipeline, error::OutputError,
    exit_handle::ExitHandle, fl,
};

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
        let FixBroken {
            force_unsafe_io,
            force_yes,
            force_confnew,
            autoremove,
            remove_config,
            no_fix_dpkg_status,
            no_clean,
        } = self;

        Pipeline::builder()
            .config(&config)
            .always_escalate(true)
            .is_fixbroken(true)
            .no_fixbroken(false)
            .fix_dpkg_status(!no_fix_dpkg_status)
            .autoremove(autoremove)
            .remove_config(remove_config)
            .no_clean(no_clean)
            .build()
            .run(|ctx| {
                let oma_apt_args = OmaAptArgs::builder()
                    .sysroot(ctx.config.sysroot.to_string_lossy().to_string())
                    .dpkg_force_unsafe_io(force_unsafe_io)
                    .force_yes(force_yes)
                    .dpkg_force_confnew(force_confnew)
                    .another_apt_options(&ctx.config.apt_options)
                    .build();
                let apt = OmaApt::new(vec![], oma_apt_args, ctx.config.dry_run)?;

                ctx.commit(apt)
            })
    }
}
