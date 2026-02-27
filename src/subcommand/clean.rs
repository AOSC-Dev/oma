use std::fs::{self};
use std::io::ErrorKind;
use std::path::Path;

use crate::config::OmaConfig;
use crate::subcommand::utils::create_progress_spinner;

use crate::fl;
use crate::success;
use crate::utils::ExitHandle;
use clap::Args;
use fs_extra::dir::get_size;
use oma_console::indicatif::HumanBytes;
use oma_pm::apt::{AptConfig, OmaApt, OmaAptArgs};
use spdlog::{debug, error, info};

use crate::{error::OutputError, utils::root};

use crate::args::CliExecuter;

#[derive(Debug, Args)]
pub struct Clean {
    /// Keep downloadable packages
    #[arg(long, conflicts_with = "keep_downloadable_and_installed", help = fl!("clap-clean-keep-downloadable-help"))]
    keep_downloadable: bool,
    /// Keep downloadable and installed packages
    #[arg(long, conflicts_with = "keep_downloadable", help = fl!("clap-clean-keep-downloadable-and-installed-help"))]
    keep_downloadable_and_installed: bool,
    /// Keep installed packages
    #[arg(long, conflicts_with = "keep_downloadable_and_installed", help = fl!("clap-clean-keep-installed-help"))]
    keep_installed: bool,
}

impl CliExecuter for Clean {
    fn execute(self, config: OmaConfig) -> Result<ExitHandle, OutputError> {
        let Clean {
            keep_downloadable,
            keep_downloadable_and_installed,
            keep_installed,
        } = self;

        let no_progress = config.no_progress();

        let OmaConfig {
            sysroot,
            apt_options,
            dry_run,
            ..
        } = config;

        if dry_run {
            info!("Running in dry-run mode, Exit.");
            return Ok(ExitHandle::default());
        }

        root()?;

        let apt_config = AptConfig::new();
        let oma_apt_args = OmaAptArgs::builder()
            .sysroot(sysroot.to_string_lossy().to_string())
            .another_apt_options(apt_options)
            .build();

        let apt = OmaApt::new(vec![], oma_apt_args, false, apt_config)?;
        let download_dir = apt.get_archive_dir();
        let dir = fs::read_dir(download_dir).map_err(|e| OutputError {
            description: format!("Failed to read dir: {}", download_dir.display()),
            source: Some(Box::new(e)),
        })?;

        let pb = create_progress_spinner(no_progress, fl!("cleaning"));

        let mut total_clean_size = 0;

        remove(&download_dir.join("partial"), &mut total_clean_size);

        for i in dir
            .flatten()
            .filter(|x| x.path().extension().is_some_and(|name| name == "deb"))
        {
            if !keep_downloadable && !keep_downloadable_and_installed && !keep_installed {
                remove(&i.path(), &mut total_clean_size);
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
                remove(&i.path(), &mut total_clean_size);
                continue;
            };

            let version = version.replace("%3a", ":");

            let Some(version) = apt.cache.get(pkg).and_then(|pkg| pkg.get_version(&version)) else {
                remove(&i.path(), &mut total_clean_size);
                continue;
            };

            if !version.is_installed()
                && !version.is_downloadable()
                && keep_downloadable_and_installed
            {
                remove(&i.path(), &mut total_clean_size);
                continue;
            }

            if (!version.is_downloadable() && keep_downloadable)
                || (!version.is_installed() && keep_installed)
            {
                remove(&i.path(), &mut total_clean_size);
                continue;
            }
        }

        if let Some(pb) = pb {
            pb.inner.finish_and_clear();
        }

        if total_clean_size != 0 {
            let size = HumanBytes(total_clean_size).to_string();
            success!("{}", fl!("clean-successfully", size = size));
        } else {
            info!("{}", fl!("clean-zero"));
        }

        Ok(ExitHandle::default().ring(true))
    }
}

fn remove(i: &Path, total_size: &mut u64) {
    let size = get_size(i);

    let result = if i.is_dir() {
        fs::remove_dir_all(i)
    } else {
        fs::remove_file(i)
    };

    match result {
        Ok(_) => *total_size += size.unwrap_or(0),
        Err(e) => {
            if e.kind() == ErrorKind::NotFound {
                return;
            }
            error!(
                "Failed to delete {} {}: {}",
                if i.is_dir() { "dir" } else { "file" },
                i.display(),
                e
            );
        }
    }
}
