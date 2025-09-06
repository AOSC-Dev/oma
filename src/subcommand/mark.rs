use std::{borrow::Cow, path::PathBuf};

use clap::{Args, ValueEnum};
use clap_complete::ArgValueCompleter;
use oma_console::print::Action;
use oma_pm::{
    apt::{AptConfig, OmaApt, OmaAptArgs},
    matches::{GetArchMethod, PackagesMatcher},
};
use tracing::info;

use crate::{
    color_formatter,
    config::Config,
    error::OutputError,
    success,
    utils::{pkgnames_completions, root},
};

use super::utils::handle_no_result;
use crate::args::CliExecuter;
use crate::fl;

#[derive(Debug, Copy, Clone, Eq, PartialEq, ValueEnum)]
pub enum MarkAction {
    /// Lock package version(s), this will prevent the specified package(s) from being updated or downgraded
    #[value(help = fl!("clap-mark-hold"))]
    Hold,
    /// Unlock package version(s), this will undo the “hold” status on the specified package(s)
    #[value(help = fl!("clap-mark-unhold"))]
    Unhold,
    /// Mark package(s) as manually installed, this will prevent the specified package(s) from being removed when all reverse dependencies were removed
    #[value(help = fl!("clap-mark-manual"))]
    Manual,
    /// Mark package(s) as automatically installed, this will mark the specified package(s) for removal when all reverse dependencies were removed
    #[value(help = fl!("clap-mark-auto"))]
    Auto,
}

#[derive(Debug, Args)]
pub struct Mark {
    /// Mark status for one or multiple package(s)
    #[arg(
        required = true,
        help = fl!("clap-mark-help"),
        long_help = fl!("clap-mark-long-help")
    )]
    action: MarkAction,
    /// Package(s) to mark status for
    #[arg(required = true, add = ArgValueCompleter::new(pkgnames_completions), help = fl!("clap-mark-packages-help"))]
    packages: Vec<String>,
    /// Run oma in "dry-run" mode. Useful for testing changes and operations without making changes to the system
    #[arg(from_global, help = fl!("clap-dry-run-help"), long_help = fl!("clap-dry-run-long-help"))]
    dry_run: bool,
    /// Set sysroot target directory
    #[arg(from_global, help = fl!("clap-sysroot-help"))]
    sysroot: PathBuf,
    /// Set apt options
    #[arg(from_global, help = fl!("clap-apt-options-help"))]
    apt_options: Vec<String>,
    /// Print help
    #[arg(long, short, action = clap::ArgAction::HelpLong, help = fl!("clap-help"))]
    help: bool,
}

impl CliExecuter for Mark {
    fn execute(self, _config: &Config, no_progress: bool) -> Result<i32, OutputError> {
        let Mark {
            action,
            packages,
            dry_run,
            sysroot,
            apt_options,
            ..
        } = self;

        if !dry_run {
            root()?;
        }

        let oma_apt_args = OmaAptArgs::builder()
            .sysroot(sysroot.to_string_lossy().to_string())
            .another_apt_options(apt_options)
            .build();

        let apt = OmaApt::new(vec![], oma_apt_args, false, AptConfig::new())?;

        let set = match action {
            MarkAction::Hold | MarkAction::Unhold => apt
                .mark_version_status(&packages, action == MarkAction::Hold, dry_run)?
                .into_iter()
                .map(|(x, y)| (Cow::Borrowed(x), y))
                .collect::<Vec<_>>(),
            MarkAction::Auto | MarkAction::Manual => {
                let matcher = PackagesMatcher::builder()
                    .cache(&apt.cache)
                    .native_arch(GetArchMethod::SpecifySysroot(&sysroot))
                    .build();

                let (pkgs, no_result) =
                    matcher.match_pkgs_and_versions(packages.iter().map(|x| x.as_str()))?;

                handle_no_result(&sysroot, no_result, no_progress)?;

                apt.mark_install_status(pkgs, action == MarkAction::Auto, dry_run)?
                    .into_iter()
                    .map(|(x, y)| (Cow::Owned(x), y))
                    .collect()
            }
        };

        for (pkg, is_set) in set {
            let pkg = color_formatter()
                .color_str(pkg, Action::Emphasis)
                .to_string();

            match action {
                MarkAction::Hold if is_set => {
                    success!("{}", fl!("set-to-hold", name = pkg));
                }
                MarkAction::Hold => {
                    info!("{}", fl!("already-hold", name = pkg));
                }
                MarkAction::Unhold if is_set => {
                    success!("{}", fl!("set-to-unhold", name = pkg));
                }
                MarkAction::Unhold => {
                    info!("{}", fl!("already-unhold", name = pkg));
                }
                MarkAction::Auto if is_set => {
                    success!("{}", fl!("setting-auto", name = pkg));
                }
                MarkAction::Auto => {
                    info!("{}", fl!("already-auto", name = pkg));
                }
                MarkAction::Manual if is_set => {
                    success!("{}", fl!("setting-manual", name = pkg));
                }
                MarkAction::Manual => {
                    info!("{}", fl!("already-manual", name = pkg));
                }
            };
        }

        Ok(0)
    }
}
