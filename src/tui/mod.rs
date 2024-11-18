use std::{path::Path, time::Duration};

use apt_auth_config::AuthConfig;
use oma_console::{
    indicatif::ProgressBar,
    pager::{exit_tui, prepare_create_tui},
    pb::spinner_style,
};
use oma_history::SummaryType;
use oma_pm::{
    apt::{AptConfig, OmaApt, OmaAptArgs, Upgrade},
    search::IndiciumSearch,
};
use oma_utils::dbus::{create_dbus_connection, take_wake_lock};
use tui_inner::{Task, Tui};

use crate::{
    error::OutputError,
    find_another_oma, fl,
    subcommand::utils::{lock_oma, no_check_dbus_warn, CommitRequest, RefreshRequest},
    utils::{check_battery, root},
    HTTP_CLIENT, RT,
};

mod state;
mod tui_inner;

pub struct TuiArgs {
    pub sysroot: String,
    pub no_progress: bool,
    pub dry_run: bool,
    pub network_thread: usize,
    pub no_check_dbus: bool,
    pub another_apt_options: Vec<String>,
}

pub fn execute(tui: TuiArgs) -> Result<i32, OutputError> {
    if find_another_oma().is_ok() {
        return Err(OutputError {
            description: "".to_string(),
            source: None,
        });
    }

    if Path::new("/run/lock/oma.lock").exists() {
        return Err(OutputError {
            description: fl!("failed-to-lock-oma"),
            source: None,
        });
    }

    root()?;

    let conn = RT.block_on(create_dbus_connection())?;
    check_battery(&conn, false);

    let TuiArgs {
        sysroot,
        no_progress,
        dry_run,
        network_thread,
        no_check_dbus,
        another_apt_options,
    } = tui;

    let apt_config = AptConfig::new();
    let auth_config = AuthConfig::system(&sysroot)?;

    RefreshRequest {
        client: &HTTP_CLIENT,
        dry_run,
        no_progress,
        limit: network_thread,
        sysroot: &sysroot,
        _refresh_topics: true,
        config: &apt_config,
        auth_config: &auth_config,
    }
    .run()?;

    let oma_apt_args = OmaAptArgs::builder()
        .sysroot(sysroot.clone())
        .another_apt_options(another_apt_options)
        .build();

    let mut apt = OmaApt::new(vec![], oma_apt_args, false, apt_config)?;

    let (sty, inv) = spinner_style();
    let pb = ProgressBar::new_spinner().with_style(sty);
    pb.enable_steady_tick(inv);
    pb.set_message(fl!("reading-database"));

    let a = apt.count_pending_upgradable_and_autoremovable_pkgs()?;
    let installed = apt.installed_packages()?;

    let searcher = IndiciumSearch::new(&apt.cache, |n| {
        pb.set_message(fl!("reading-database-with-count", count = n));
    })?;
    pb.finish_and_clear();

    let mut terminal = prepare_create_tui().map_err(|e| OutputError {
        description: "BUG: Failed to create crossterm instance".to_string(),
        source: Some(Box::new(e)),
    })?;

    let tui = Tui::new(&apt, a, installed, searcher);

    let Task {
        execute_apt,
        install,
        remove,
        upgrade,
        autoremove,
    } = tui.run(&mut terminal, Duration::from_millis(250)).unwrap();

    exit_tui(&mut terminal).map_err(|e| OutputError {
        description: "BUG: Failed to exit tui".to_string(),
        source: Some(Box::new(e)),
    })?;

    let mut code = 0;

    if execute_apt {
        let _fds = if !no_check_dbus {
            let fds = RT.block_on(take_wake_lock(&conn, &fl!("changing-system"), "oma"))?;
            Some(fds)
        } else {
            no_check_dbus_warn();
            None
        };

        lock_oma()?;

        if upgrade {
            apt.upgrade(Upgrade::FullUpgrade)?;
        }

        apt.install(&install, false)?;
        apt.remove(
            remove
                .iter()
                .flat_map(|x| x.into_oma_package_without_version()),
            false,
            !autoremove,
        )?;

        code = CommitRequest {
            apt,
            dry_run,
            request_type: SummaryType::Changes,
            no_fixbroken: false,
            network_thread,
            no_progress,
            sysroot,
            fix_dpkg_status: true,
            protect_essential: true,
            client: &HTTP_CLIENT,
            yes: false,
            remove_config: false,
            auth_config: &auth_config,
        }
        .run()?;
    }

    Ok(code)
}
