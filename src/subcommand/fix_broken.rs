use apt_auth_config::AuthConfig;
use oma_history::SummaryType;
use oma_pm::apt::{AptConfig, OmaApt, OmaAptArgs};

use crate::{
    error::OutputError,
    utils::{dbus_check, root},
    OmaArgs, HTTP_CLIENT,
};

use super::utils::{lock_oma, no_check_dbus_warn, CommitRequest};

pub fn execute(oma_args: OmaArgs, sysroot: String) -> Result<i32, OutputError> {
    root()?;
    lock_oma()?;

    let OmaArgs {
        dry_run,
        network_thread,
        no_progress,
        no_check_dbus,
        protect_essentials: protect_essential,
        another_apt_options,
    } = oma_args;

    let mut fds = None;

    if !no_check_dbus {
        fds = Some(dbus_check(false)?);
    } else {
        no_check_dbus_warn();
    }

    let auth_config = AuthConfig::system(&sysroot)?;

    let oma_apt_args = OmaAptArgs::builder()
        .sysroot(sysroot.clone())
        .another_apt_options(another_apt_options)
        .build();
    let apt = OmaApt::new(vec![], oma_apt_args, dry_run, AptConfig::new())?;

    let request = CommitRequest {
        apt,
        dry_run,
        request_type: SummaryType::FixBroken,
        no_fixbroken: false,
        network_thread,
        no_progress,
        sysroot,
        fix_dpkg_status: true,
        protect_essential,
        client: &HTTP_CLIENT,
        yes: false,
        remove_config: false,
        auth_config: &auth_config,
    };

    let code = request.run()?;

    drop(fds);

    Ok(code)
}
