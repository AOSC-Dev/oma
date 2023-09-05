use std::path::PathBuf;

use oma_console::{error, success};
use oma_pm::apt::{OmaApt, OmaAptArgsBuilder};

use crate::fl;
use crate::{error::OutputError, pb, subcommand::utils::handle_no_result, utils::multibar};

pub fn execute(
    keyword: Vec<&str>,
    path: Option<PathBuf>,
    dry_run: bool,
) -> Result<i32, OutputError> {
    let oma_apt_args = OmaAptArgsBuilder::default().build()?;
    let mut apt = OmaApt::new(vec![], oma_apt_args, dry_run)?;
    let (pkgs, no_result) = apt.select_pkg(keyword, false, true)?;
    handle_no_result(no_result);

    let (mb, pb_map, global_is_set) = multibar();
    let (success, failed) = apt.download(
        pkgs,
        None,
        path.as_deref(),
        dry_run,
        |count, event, total| pb!(event, mb, pb_map, count, total, global_is_set),
    )?;

    let pbc = pb_map.clone();

    if let Some(gpb) = pbc.clone().get(&0) {
        gpb.finish_and_clear();
    }

    if !failed.is_empty() {
        // TODO: 翻译
        error!("Have {} packages download failed.", failed.len());
    }

    let path = path
        .unwrap_or_else(|| PathBuf::from("."))
        .canonicalize()?
        .display()
        .to_string();

    success!(
        "{}",
        fl!(
            "successfully-download-to-path",
            len = success.len(),
            path = path
        )
    );

    Ok(0)
}
