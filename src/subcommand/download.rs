use std::path::PathBuf;

use oma_console::{due_to, success};
use oma_fetch::DownloadProgressControl;
use oma_pm::apt::{AptConfig, OmaApt, OmaAptArgs};
use tracing::error;

use crate::pb::{NoProgressBar, OmaMultiProgressBar};
use crate::{error::OutputError, subcommand::utils::handle_no_result};
use crate::{fl, OmaArgs, HTTP_CLIENT};

pub fn execute(
    keyword: Vec<&str>,
    path: Option<PathBuf>,
    oma_args: OmaArgs,
) -> Result<i32, OutputError> {
    let OmaArgs {
        dry_run,
        network_thread,
        no_progress,
        ..
    } = oma_args;

    let path = path.unwrap_or_else(|| PathBuf::from("."));

    let path = path.canonicalize().map_err(|e| OutputError {
        description: format!("Failed to canonicalize path: {}", path.display()),
        source: Some(Box::new(e)),
    })?;

    let apt_config = AptConfig::new();
    let oma_apt_args = OmaAptArgs::builder().build();
    let mut apt = OmaApt::new(vec![], oma_apt_args, dry_run, apt_config)?;
    let (pkgs, no_result) = apt.select_pkg(&keyword, false, true, true)?;
    handle_no_result("/", no_result, no_progress)?;

    let progress_manager: &dyn DownloadProgressControl = if !no_progress {
        &OmaMultiProgressBar::default()
    } else {
        &NoProgressBar::default()
    };

    let (success, failed) = apt.download(
        &HTTP_CLIENT,
        pkgs,
        Some(network_thread),
        Some(&path),
        dry_run,
        progress_manager,
    )?;

    if !success.is_empty() {
        success!(
            "{}",
            fl!(
                "successfully-download-to-path",
                len = success.len(),
                path = path.display().to_string()
            )
        );
    }

    if !failed.is_empty() {
        let len = failed.len();
        for f in failed {
            let e = OutputError::from(f);
            error!("{e}");
            if let Some(s) = e.source {
                due_to!("{s}");
            }
        }

        return Err(OutputError {
            description: fl!("download-failed-with-len", len = len),
            source: None,
        });
    }

    Ok(0)
}
