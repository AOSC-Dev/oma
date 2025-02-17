use std::fs::remove_dir_all;
use std::path::PathBuf;

use crate::subcommand::utils::create_progress_spinner;
use crate::success;
use crate::{config::Config, fl};
use clap::Args;
use oma_pm::apt::{AptConfig, OmaApt, OmaAptArgs};
use tracing::{debug, info};

use crate::{error::OutputError, utils::root};

use crate::args::CliExecuter;

#[derive(Debug, Args)]
pub struct Clean {
    /// Keep downloadable packages
    #[arg(long, conflicts_with = "keep_downloadable_and_installed")]
    keep_downloadable: bool,
    /// Keep downloadable and installed packages
    #[arg(long, conflicts_with = "keep_downloadable")]
    keep_downloadable_and_installed: bool,
    /// Keep installed packages
    #[arg(long, conflicts_with = "keep_downloadable_and_installed")]
    keep_installed: bool,
    /// Set sysroot target directory
    #[arg(from_global)]
    sysroot: PathBuf,
    /// Set apt options
    #[arg(from_global)]
    apt_options: Vec<String>,
    /// Run oma in “dry-run” mode. Useful for testing changes and operations without making changes to the system
    #[arg(from_global)]
    dry_run: bool,
}

impl CliExecuter for Clean {
    fn execute(self, _config: &Config, no_progress: bool) -> Result<i32, OutputError> {
        let Clean {
            sysroot,
            apt_options,
            dry_run,
            keep_downloadable,
            keep_downloadable_and_installed,
            keep_installed,
        } = self;

        if dry_run {
            info!("Running in dry-run mode, Exit.");
            return Ok(0);
        }

        root()?;

        let apt_config = AptConfig::new();
        let oma_apt_args = OmaAptArgs::builder()
            .sysroot(sysroot.to_string_lossy().to_string())
            .another_apt_options(apt_options)
            .build();

        let apt = OmaApt::new(vec![], oma_apt_args, false, apt_config)?;
        let download_dir = apt.get_archive_dir();
        let dir = std::fs::read_dir(download_dir).map_err(|e| OutputError {
            description: format!("Failed to read dir: {}", download_dir.display()),
            source: Some(Box::new(e)),
        })?;

        let pb = create_progress_spinner(no_progress, fl!("cleaning"));

        remove_dir_all(download_dir.join("partial")).ok();

        for i in dir
            .flatten()
            .filter(|x| x.path().extension().is_some_and(|name| name == "deb"))
        {
            if !keep_downloadable && !keep_downloadable_and_installed && !keep_installed {
                remove_deb(&i);
                continue;
            }

            let file_name = i.file_name();
            let file_name = file_name.to_string_lossy();
            let mut file_name = file_name.splitn(3, '_');

            let Some((pkg, version)) = Some(()).and_then(|_| {
                let package = file_name.next()?;
                let version = file_name.next()?;

                Some((package, version))
            }) else {
                debug!(
                    "Failed to get package name or version: {}, will delete this file",
                    i.path().display()
                );
                remove_deb(&i);
                continue;
            };

            let version = version.replace("%3a", ":");

            let Some(version) = apt.cache.get(pkg).and_then(|pkg| pkg.get_version(&version)) else {
                remove_deb(&i);
                continue;
            };

            if !version.is_installed()
                && !version.is_downloadable()
                && keep_downloadable_and_installed
            {
                remove_deb(&i);
                continue;
            }

            if (!version.is_downloadable() && keep_downloadable)
                || (!version.is_installed() && keep_installed)
            {
                remove_deb(&i);
                continue;
            }
        }

        if let Some(pb) = pb {
            pb.inner.finish_and_clear();
        }

        success!("{}", fl!("clean-successfully"));

        Ok(0)
    }
}

fn remove_deb(i: &std::fs::DirEntry) {
    std::fs::remove_file(i.path()).ok();
}
