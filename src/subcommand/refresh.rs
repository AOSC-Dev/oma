use oma_console::indicatif::ProgressBar;
use oma_console::pb::spinner_style;
use oma_console::success;
use oma_pm::apt::{AptConfig, OmaApt, OmaAptArgsBuilder};
use reqwest::Client;

use crate::{error::OutputError, utils::root};
use crate::{fl, OmaArgs};

use super::utils::{refresh, RefreshRequest};

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

    let apt_config = AptConfig::new();

    let req = RefreshRequest {
        client: &client,
        dry_run: false,
        no_progress,
        limit: network_thread,
        sysroot: &sysroot,
        _refresh_topics: !no_refresh_topics,
        config: &apt_config,
    };

    refresh(req)?;

    let oma_apt_args = OmaAptArgsBuilder::default().sysroot(sysroot).build()?;
    let apt = OmaApt::new(vec![], oma_apt_args, false, apt_config)?;

    let (style, inv) = spinner_style();

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
