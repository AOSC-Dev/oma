use oma_pm::apt::{AptArgsBuilder, OmaApt, OmaAptArgsBuilder};

use crate::{
    error::OutputError,
    history::SummaryType,
    utils::{create_async_runtime, dbus_check, root},
};

use super::utils::normal_commit;

pub fn execute(dry_run: bool, network_thread: usize) -> Result<i32, OutputError> {
    root()?;

    let rt = create_async_runtime()?;
    dbus_check(&rt)?;

    let oma_apt_args = OmaAptArgsBuilder::default().build()?;
    let apt = OmaApt::new(vec![], oma_apt_args, dry_run)?;
    apt.resolve(false)?;

    normal_commit(
        apt,
        dry_run,
        SummaryType::FixBroken,
        AptArgsBuilder::default().build()?,
        false,
        network_thread,
    )?;

    Ok(0)
}
