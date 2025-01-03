use std::path::PathBuf;

use crate::subcommand::utils::create_progress_spinner;
use crate::success;
use crate::{config::Config, fl};
use clap::Args;
use oma_pm::apt::{AptConfig, OmaApt, OmaAptArgs};
use tracing::info;

use crate::{error::OutputError, utils::root};

use crate::args::CliExecuter;

#[derive(Debug, Args)]
pub struct Clean {
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

        for i in dir.flatten() {
            if i.path().extension().and_then(|x| x.to_str()) == Some("deb") {
                std::fs::remove_file(i.path()).ok();
            }
        }

        if let Some(pb) = pb {
            pb.inner.finish_and_clear();
        }

        success!("{}", fl!("clean-successfully"));

        Ok(0)
    }
}
