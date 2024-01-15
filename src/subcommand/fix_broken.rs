use oma_history::SummaryType;
use oma_pm::apt::{AptArgsBuilder, OmaApt, OmaAptArgsBuilder};

use crate::{
    error::OutputError,
    utils::{create_async_runtime, dbus_check, root},
    OmaArgs,
};

use super::utils::{lock_oma, no_check_dbus_warn, normal_commit, NormalCommitArgs};

pub fn execute(oma_args: OmaArgs, sysroot: String) -> Result<i32, OutputError> {
    root()?;
    lock_oma()?;

    let OmaArgs {
        dry_run,
        network_thread,
        no_progress,
        download_pure_db: _,
        no_check_dbus,
        ..
    } = oma_args;

    if !no_check_dbus {
        let rt = create_async_runtime()?;
        dbus_check(&rt, false)?;
    } else {
        no_check_dbus_warn();
    }

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
