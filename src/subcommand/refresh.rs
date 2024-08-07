use oma_console::indicatif::ProgressBar;
use oma_console::pb::oma_spinner;
use oma_console::success;
use oma_pm::apt::{OmaApt, OmaAptArgsBuilder};
use reqwest::Client;

use crate::{error::OutputError, utils::root};
use crate::{fl, OmaArgs};

use super::utils::refresh;

pub fn execute(
    oma_args: OmaArgs,
    sysroot: String,
    client: Client,
    no_refresh_topics: bool,
) -> Result<i32, OutputError> {
    root()?;

    let OmaArgs {
        dry_run: _,
        network_thread,
        no_progress,
        ..
    } = oma_args;

    refresh(
        &client,
        false,
        no_progress,
        network_thread,
        &sysroot,
        !no_refresh_topics,
    )?;

    let oma_apt_args = OmaAptArgsBuilder::default().sysroot(sysroot).build()?;
    let apt = OmaApt::new(vec![], oma_apt_args, false)?;

    let (style, inv) = oma_spinner(false);

    let pb = ProgressBar::new_spinner().with_style(style);
    pb.enable_steady_tick(inv);
    pb.set_message(fl!("reading-database"));

    let (upgradable, removable) = apt.available_action()?;
    pb.finish_and_clear();

    let mut s = vec![];

    if upgradable != 0 {
        s.push(fl!("packages-can-be-upgrade", len = upgradable));
    }

    if removable != 0 {
        s.push(fl!("packages-can-be-removed", len = removable));
    }

    if s.is_empty() {
        success!("{}", fl!("successfully-refresh"));
    } else {
        let s = s.join(&fl!("comma"));
        success!("{}", fl!("successfully-refresh-with-tips", s = s));
    }

    Ok(0)
}
