use std::path::PathBuf;

use oma_console::{due_to, success};
use oma_pm::apt::{InstallPackageEvent, OmaApt, OmaAptArgsBuilder};
use reqwest::Client;
use tracing::error;

use crate::subcommand::utils::handle_event_without_progressbar;
use crate::{error::OutputError, pb, subcommand::utils::handle_no_result, utils::multibar};
use crate::{fl, OmaArgs};

pub fn execute(
    keyword: Vec<&str>,
    path: Option<PathBuf>,
    oma_args: OmaArgs,
    client: &Client,
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

    let oma_apt_args = OmaAptArgsBuilder::default().build()?;
    let mut apt = OmaApt::new(vec![], oma_apt_args, dry_run)?;
    let (pkgs, no_result) = apt.select_pkg(&keyword, false, true, true)?;
    handle_no_result(no_result)?;

    let (mb, pb_map, global_is_set) = multibar();
    let (success, failed) = apt.download(
        client,
        pkgs,
        Some(network_thread),
        Some(&path),
        dry_run,
        |count, event, total| {
            let event = InstallPackageEvent::from(event);
            if !no_progress {
                pb!(event, mb, pb_map, count, total, global_is_set);
            } else {
                handle_event_without_progressbar(event);
            }
        },
    )?;

    if let Some(gpb) = pb_map.get(&0) {
        gpb.finish_and_clear();
    }

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
