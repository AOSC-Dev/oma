use std::path::PathBuf;

use oma_console::success;
use oma_pm::apt::{OmaApt, OmaAptArgsBuilder};

use crate::fl;
use crate::subcommand::utils::handle_event_without_progressbar;
use crate::{error::OutputError, pb, subcommand::utils::handle_no_result, utils::multibar};

use oma_console::error;

pub fn execute(
    keyword: Vec<&str>,
    path: Option<PathBuf>,
    dry_run: bool,
    no_progress: bool,
) -> Result<i32, OutputError> {
    let oma_apt_args = OmaAptArgsBuilder::default().build()?;
    let mut apt = OmaApt::new(vec![], oma_apt_args, dry_run)?;
    let (pkgs, no_result) = apt.select_pkg(&keyword, false, true)?;
    handle_no_result(no_result);

    let (mb, pb_map, global_is_set) = multibar();
    let (success, failed) = apt.download(
        pkgs,
        None,
        path.as_deref(),
        dry_run,
        |count, event, total| {
            if !no_progress {
                pb!(event, mb, pb_map, count, total, global_is_set)
            } else {
                handle_event_without_progressbar(event);
            }
        },
    )?;

    let pbc = pb_map.clone();

    if let Some(gpb) = pbc.clone().get(&0) {
        gpb.finish_and_clear();
    }

    let path = path
        .unwrap_or_else(|| PathBuf::from("."))
        .canonicalize()?
        .display()
        .to_string();

    if !success.is_empty() {
        success!(
            "{}",
            fl!(
                "successfully-download-to-path",
                len = success.len(),
                path = path
            )
        );
    }

    if !failed.is_empty() {
        let len = failed.len();
        for f in failed {
            let e = OutputError::from(f);
            let (err, _) = e.inner();
            error!("{err}");
        }

        return Err(OutputError::new(
            fl!("download-failed-with-len", len = len),
            None,
        ));
    }

    Ok(0)
}
