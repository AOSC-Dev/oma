use oma_console::success;
use oma_pm::apt::{OmaApt, OmaAptArgsBuilder};

use crate::{error::OutputError, utils::root};
use crate::{fl, OmaArgs};

use super::utils::refresh;

pub fn execute(oma_args: OmaArgs, sysroot: String) -> Result<i32, OutputError> {
    root()?;

    let OmaArgs {
        dry_run: _,
        network_thread: _,
        no_progress,
        download_pure_db,
        ..
    } = oma_args;

    refresh(false, no_progress, download_pure_db, &sysroot)?;

    let oma_apt_args = OmaAptArgsBuilder::default().sysroot(sysroot).build()?;
    let apt = OmaApt::new(vec![], oma_apt_args, false)?;
    let (upgradable, removable) = apt.available_action()?;
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
