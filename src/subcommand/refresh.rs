use oma_console::success;
use oma_pm::apt::{OmaApt, OmaAptArgsBuilder};

use crate::fl;
use crate::{error::OutputError, utils::root};

use super::utils::refresh;

pub fn execute() -> Result<i32, OutputError> {
    root()?;
    refresh(false)?;

    let oma_apt_args = OmaAptArgsBuilder::default().build()?;
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
