use anyhow::anyhow;
use clap::Args;
use clap_complete::ArgValueCompleter;
use dialoguer::console::style;
use dialoguer::theme::ColorfulTheme;
use dialoguer::{Confirm, Input};
use oma_pm::apt::{AptConfig, OmaApt, OmaAptArgs};
use oma_pm::matches::{GetArchMethod, PackagesMatcher};
use spdlog::{info, warn};

use crate::config::OmaConfig;
use crate::fl;
use crate::utils::{ExitHandle, pkgnames_remove_completions};
use crate::{
    error::OutputError,
    utils::{dbus_check, root},
};

use super::utils::{
    CommitChanges, auth_config, create_progress_spinner, handle_no_result, lock_oma,
};
use crate::args::CliExecuter;

#[derive(Debug, Args)]
pub struct Remove {
    /// Package(s) to remove
    #[arg(add = ArgValueCompleter::new(pkgnames_remove_completions), help = fl!("clap-remove-packages-help"))]
    #[arg(help_heading = &**crate::args::ARG_HELP_HEADING_MUST)]
    packages: Vec<String>,
    /// Bypass confirmation prompts
    #[arg(short, long, help = fl!("clap-yes-help"))]
    yes: bool,
    /// Resolve broken dependencies in the system
    #[arg(short, long, help = fl!("clap-fix-broken-help"))]
    fix_broken: bool,
    /// Fix dpkg broken status
    #[arg(long, help = fl!("clap-fix-dpkg-status-help"))]
    fix_dpkg_status: bool,
    /// Install package(s) without fsync(2)
    #[arg(
        long,
        help = fl!("clap-force-unsafe-io-help", dangerous = crate::console::style(format!("{}", fl!("clap-dangerous"))).red().to_string()))]
    force_unsafe_io: bool,
    /// Ignore repository and package dependency issues
    #[arg(long, help = fl!("clap-force-yes-help"))]
    force_yes: bool,
    /// Replace configuration file(s) in the system those shipped in the package(s) to be installed (invokes `dpkg --force-confnew`)
    #[arg(long, help = fl!("clap-force-confnew-help"))]
    force_confnew: bool,
    /// Do not auto remove unnecessary package(s)
    #[arg(long, help = fl!("clap-no-autoremove-help"))]
    no_autoremove: bool,
    /// Remove package(s) also remove configuration file(s), like apt purge
    #[arg(long, visible_alias = "purge", help = fl!("clap-remove-config-help"))]
    remove_config: bool,
}

#[derive(Debug, Args)]
pub struct Purge {
    /// Package(s) to remove and purge configurations
    #[arg(add = ArgValueCompleter::new(pkgnames_remove_completions), help = fl!("clap-purge-packages-help"))]
    packages: Vec<String>,
    /// Bypass confirmation prompts
    #[arg(short, long, help = fl!("clap-yes-help"))]
    yes: bool,
    /// Resolve broken dependencies in the system
    #[arg(short, long, help = fl!("clap-fix-broken-help"))]
    fix_broken: bool,
    /// Fix dpkg broken status
    #[arg(long, help = fl!("clap-fix-dpkg-status-help"))]
    fix_dpkg_status: bool,
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
    /// Do not auto remove unnecessary package(s)
    #[arg(long, help = fl!("clap-no-autoremove-help"))]
    no_autoremove: bool,
}

impl From<Purge> for Remove {
    fn from(value: Purge) -> Self {
        let Purge {
            packages,
            yes,
            fix_broken,
            force_unsafe_io,
            force_yes,
            force_confnew,
            no_autoremove,
            fix_dpkg_status,
        } = value;

        Self {
            packages,
            yes,
            fix_broken,
            force_unsafe_io,
            force_yes,
            force_confnew,
            no_autoremove,
            fix_dpkg_status,
            remove_config: true,
        }
    }
}

impl CliExecuter for Purge {
    fn execute(self, config: OmaConfig) -> Result<ExitHandle, OutputError> {
        let remove = Remove::from(self);
        remove.execute(config)
    }
}

impl CliExecuter for Remove {
    fn execute(self, config: OmaConfig) -> Result<ExitHandle, OutputError> {
        let Remove {
            packages,
            yes,
            fix_broken,
            force_unsafe_io,
            force_yes,
            force_confnew,
            no_autoremove,
            remove_config,
            fix_dpkg_status,
        } = self;

        if !config.dry_run {
            root()?;
            lock_oma(&config.sysroot)?;
        }

        let _fds = dbus_check(yes, &config)?;

        if yes {
            warn!("{}", fl!("automatic-mode-warn"));
        }

        let oma_apt_args = OmaAptArgs::builder()
            .yes(yes)
            .force_yes(force_yes)
            .sysroot(config.sysroot.to_string_lossy().to_string())
            .another_apt_options(config.apt_options.clone())
            .dpkg_force_unsafe_io(force_unsafe_io)
            .dpkg_force_confnew(force_confnew)
            .build();

        let mut apt = OmaApt::new(vec![], oma_apt_args, config.dry_run, AptConfig::new())?;
        let matcher = PackagesMatcher::builder()
            .cache(&apt.cache)
            .filter_candidate(false)
            .filter_downloadable_candidate(false)
            .select_dbg(false)
            .native_arch(GetArchMethod::SpecifySysroot(&config.sysroot))
            .build();

        let mut pkgs = vec![];
        let mut no_result = vec![];

        for i in &packages {
            let res = matcher.match_pkgs_from_glob(i)?;
            if res.is_empty() {
                no_result.push(i.as_str());
            } else {
                pkgs.extend(res);
            }
        }

        let pb = create_progress_spinner(config.no_progress(), fl!("resolving-dependencies"));

        #[cfg(feature = "aosc")]
        check_is_current_kernel_deleting(&config, &pkgs, &pb)?;

        let context = apt.remove(pkgs, remove_config, no_autoremove)?;

        if let Some(pb) = pb {
            pb.inner.finish_and_clear()
        }

        if !context.is_empty() {
            for c in context {
                info!("{}", fl!("no-need-to-remove", name = c));
            }
        }

        handle_no_result(no_result, config.no_progress())?;

        let auth_config = auth_config(&config.sysroot);
        let auth_config = auth_config.as_ref();

        CommitChanges::builder()
            .apt(apt)
            .dry_run(config.dry_run)
            .no_fixbroken(!fix_broken)
            .no_progress(config.no_progress())
            .sysroot(config.sysroot.to_string_lossy().to_string())
            .protect_essential(config.protect_essentials)
            .yes(yes)
            .remove_config(remove_config)
            .autoremove(!no_autoremove)
            .network_thread(config.download_threads)
            .maybe_auth_config(auth_config)
            .fix_dpkg_status(fix_dpkg_status)
            .build()
            .run()
    }
}

#[cfg(feature = "aosc")]
fn check_is_current_kernel_deleting(
    config: &OmaConfig,
    pkgs: &[oma_pm::pkginfo::OmaPackageWithoutVersion],
    pb: &Option<crate::pb::OmaProgressBar>,
) -> Result<(), OutputError> {
    use anyhow::Context;
    use once_cell::sync::OnceCell;

    if let Some(pb) = pb {
        pb.inner.finish_and_clear();
    }

    let image_name = OnceCell::new();
    let current_kernel_ver = OnceCell::new();

    for pkg in pkgs
        .iter()
        .filter(|pkg| pkg.raw_pkg.name().starts_with("linux-kernel-"))
    {
        let current_kernel_ver =
            current_kernel_ver.get_or_try_init(|| -> anyhow::Result<String> {
                sysinfo::System::kernel_version().context("Failed to get kernel version")
            })?;

        if oma_pm::utils::pkg_is_current_kernel(
            &config.sysroot,
            &image_name,
            pkg.raw_pkg.name(),
            current_kernel_ver,
        ) && (config.protect_essentials
            || !ask_user_delete_current_kernel(pkg.raw_pkg.name()).unwrap_or(false))
        {
            return Err(OutputError {
                description: fl!("not-allow-delete-using-kernel", ver = current_kernel_ver),
                source: None,
            });
        }
    }

    Ok(())
}

/// "Yes Do as I say" steps
pub fn ask_user_do_as_i_say(pkg: &str) -> anyhow::Result<bool> {
    let theme = ColorfulTheme::default();

    warn!("{}", fl!("essential-tips", pkg = pkg));

    let delete = Confirm::with_theme(&theme)
        .with_prompt(fl!("essential-continue"))
        .default(false)
        .interact()
        .map_err(|_| anyhow!(""))?;

    if !delete {
        return Ok(false);
    }

    info!(
        "{}",
        fl!(
            "yes-do-as-i-say",
            input = style("Do as I say!").bold().to_string()
        ),
    );

    if Input::<String>::with_theme(&theme)
        .with_prompt(fl!("yes-do-as-i-say-prompt"))
        .interact()?
        != "Do as I say!"
    {
        info!("{}", fl!("yes-do-as-i-say-abort"));
        return Ok(false);
    }

    Ok(true)
}

#[cfg(feature = "aosc")]
fn ask_user_delete_current_kernel(pkg: &str) -> anyhow::Result<bool> {
    let theme = ColorfulTheme::default();

    warn!("{}", fl!("delete-current-kernel-tips", kernel = pkg));

    let delete = Confirm::with_theme(&theme)
        .with_prompt(fl!("essential-continue"))
        .default(false)
        .interact()
        .map_err(|_| anyhow!(""))?;

    if !delete {
        return Ok(false);
    }

    Ok(true)
}
