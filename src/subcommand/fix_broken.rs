use oma_history::SummaryType;
use oma_pm::apt::{AptArgsBuilder, OmaApt, OmaAptArgsBuilder};

use crate::{
    error::OutputError,
    utils::{create_async_runtime, dbus_check, root},
};

use super::utils::{normal_commit, NormalCommitArgs};

pub fn execute(
    dry_run: bool,
    network_thread: usize,
    no_progress: bool,
    sysroot: String,
) -> Result<i32, OutputError> {
    root()?;

    let rt = create_async_runtime()?;
    dbus_check(&rt)?;

    let oma_apt_args = OmaAptArgsBuilder::default()
        .sysroot(sysroot.clone())
        .build()?;
    let apt = OmaApt::new(vec![], oma_apt_args, dry_run)?;
    apt.resolve(false)?;

    let args = NormalCommitArgs {
        apt,
        dry_run,
        typ: SummaryType::FixBroken,
        apt_args: AptArgsBuilder::default().no_progress(no_progress).build()?,
        no_fixbroken: false,
        network_thread,
        no_progress,
        sysroot,
    };

    normal_commit(args)?;

    Ok(0)
}
