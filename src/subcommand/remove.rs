use std::path::PathBuf;

use anyhow::anyhow;
use clap::Args;
use dialoguer::console::style;
use dialoguer::theme::ColorfulTheme;
use dialoguer::{Confirm, Input};
use oma_history::SummaryType;
use oma_pm::apt::{AptConfig, OmaApt, OmaAptArgs};
use oma_pm::matches::{GetArchMethod, PackagesMatcher};
use tracing::{info, warn};

use crate::config::Config;
use crate::fl;
use crate::{
    error::OutputError,
    utils::{dbus_check, root},
};

use super::utils::{
    auth_config, create_progress_spinner, handle_no_result, lock_oma, no_check_dbus_warn,
    CommitChanges,
};
use crate::args::CliExecuter;

#[derive(Debug, Args)]
pub struct Remove {
    /// Package(s) to remove
    packages: Vec<String>,
    /// Bypass confirmation prompts
    #[arg(short, long)]
    yes: bool,
    /// Run oma in “dry-run” mode. Useful for testing changes and operations without making changes to the system
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
    /// Fix apt resolver broken status
    #[arg(short, long)]
    fix_broken: bool,
    /// Do not fix dpkg broken status
    #[arg(short, long)]
    no_fix_dpkg_status: bool,
    /// Install package(s) without fsync(2)
    #[arg(long)]
    force_unsafe_io: bool,
    /// Ignore repository and package dependency issues
    #[arg(long)]
    force_yes: bool,
    /// Replace configuration file(s) in the system those shipped in the package(s) to be installed (invokes `dpkg --force-confnew`)
    #[arg(long)]
    force_confnew: bool,
    /// Do not auto remove unnecessary package(s)
    #[arg(long)]
    no_autoremove: bool,
    /// Remove package(s) also remove configuration file(s), like apt purge
    #[arg(long, visible_alias = "purge")]
    remove_config: bool,
}

#[derive(Debug, Args)]
pub struct Purge {
    /// Package(s) to remove
    packages: Vec<String>,
    /// Bypass confirmation prompts
    #[arg(short, long)]
    yes: bool,
    /// Run oma in “dry-run” mode. Useful for testing changes and operations without making changes to the system
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
    /// Fix apt resolver broken status
    #[arg(short, long)]
    fix_broken: bool,
    /// Do not fix dpkg broken status
    #[arg(short, long)]
    no_fix_dpkg_status: bool,
    /// Install package(s) without fsync(2)
    #[arg(long)]
    force_unsafe_io: bool,
    /// Ignore repository and package dependency issues
    #[arg(long)]
    force_yes: bool,
    /// Replace configuration file(s) in the system those shipped in the package(s) to be installed (invokes `dpkg --force-confnew`)
    #[arg(long)]
    force_confnew: bool,
    /// Do not auto remove unnecessary package(s)
    #[arg(long)]
    no_autoremove: bool,
}

impl From<Purge> for Remove {
    fn from(value: Purge) -> Self {
        let Purge {
            packages,
            yes,
            dry_run,
            no_check_dbus,
            sysroot,
            apt_options,
            fix_broken,
            force_unsafe_io,
            force_yes,
            force_confnew,
            no_autoremove,
            no_fix_dpkg_status,
        } = value;

        Self {
            packages,
            yes,
            dry_run,
            no_check_dbus,
            sysroot,
            apt_options,
            fix_broken,
            force_unsafe_io,
            force_yes,
            force_confnew,
            no_autoremove,
            no_fix_dpkg_status,
            remove_config: true,
        }
    }
}

impl CliExecuter for Purge {
    fn execute(self, config: &Config, no_progress: bool) -> Result<i32, OutputError> {
        let remove = Remove::from(self);
        remove.execute(config, no_progress)
    }
}

impl CliExecuter for Remove {
    fn execute(self, config: &Config, no_progress: bool) -> Result<i32, OutputError> {
        let Remove {
            packages,
            yes,
            dry_run,
            no_check_dbus,
            sysroot,
            apt_options,
            fix_broken,
            force_unsafe_io,
            force_yes,
            force_confnew,
            no_autoremove,
            remove_config,
            no_fix_dpkg_status,
        } = self;

        if !dry_run {
            root()?;
            lock_oma()?;
        }

        let _fds = if !no_check_dbus && !config.no_check_dbus() && !dry_run {
            Some(dbus_check(yes))
        } else {
            no_check_dbus_warn();
            None
        };

        if yes {
            warn!("{}", fl!("automatic-mode-warn"));
        }

        let oma_apt_args = OmaAptArgs::builder()
            .yes(yes)
            .force_yes(force_yes)
            .sysroot(sysroot.to_string_lossy().to_string())
            .another_apt_options(apt_options)
            .dpkg_force_unsafe_io(force_unsafe_io)
            .dpkg_force_confnew(force_confnew)
            .build();

        let mut apt = OmaApt::new(vec![], oma_apt_args, dry_run, AptConfig::new())?;
        let matcher = PackagesMatcher::builder()
            .cache(&apt.cache)
            .filter_candidate(false)
            .filter_downloadable_candidate(false)
            .select_dbg(false)
            .native_arch(GetArchMethod::SpecifySysroot(&sysroot))
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

        let pb = create_progress_spinner(no_progress, fl!("resolving-dependencies"));

        let context = apt.remove(pkgs, remove_config, no_autoremove)?;

        if let Some(pb) = pb {
            pb.inner.finish_and_clear()
        }

        if !context.is_empty() {
            for c in context {
                info!("{}", fl!("no-need-to-remove", name = c));
            }
        }

        handle_no_result(&sysroot, no_result, no_progress)?;

        let auth_config = auth_config(&sysroot);
        let auth_config = auth_config.as_ref();

        CommitChanges::builder()
            .apt(apt)
            .dry_run(dry_run)
            .request_type(SummaryType::Remove)
            .no_fixbroken(!fix_broken)
            .no_progress(no_progress)
            .sysroot(sysroot.to_string_lossy().to_string())
            .protect_essential(config.protect_essentials())
            .yes(yes)
            .remove_config(remove_config)
            .autoremove(!no_autoremove)
            .network_thread(config.network_thread())
            .maybe_auth_config(auth_config)
            .fix_dpkg_status(!no_fix_dpkg_status)
            .build()
            .run()
    }
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
