use std::time::Duration;

use oma_console::{
    indicatif::ProgressBar,
    pager::{exit_tui, prepare_create_tui},
    pb::spinner_style,
};
use oma_history::SummaryType;
use oma_pm::{
    apt::{AptArgs, AptConfig, OmaApt, OmaAptArgs},
    search::IndiciumSearch,
};
use reqwest::Client;
use tui::Tui;

use crate::{
    error::OutputError,
    fl,
    subcommand::utils::{lock_oma, no_check_dbus_warn, CommitRequest, RefreshRequest},
    utils::{create_async_runtime, dbus_check, root},
};

mod state;
mod tui;

pub struct TuiArgs {
    pub sysroot: String,
    pub no_progress: bool,
    pub dry_run: bool,
    pub network_thread: usize,
    pub client: Client,
    pub no_check_dbus: bool,
}

pub fn execute(tui: TuiArgs) -> Result<i32, OutputError> {
    root()?;

    let TuiArgs {
        sysroot,
        no_progress,
        dry_run,
        network_thread,
        client,
        no_check_dbus,
    } = tui;

    let apt_config = AptConfig::new();

    RefreshRequest {
        client: &client,
        dry_run,
        no_progress,
        limit: network_thread,
        sysroot: &sysroot,
        _refresh_topics: true,
        config: &apt_config,
    }
    .run()?;

    let oma_apt_args = OmaAptArgs::builder().sysroot(sysroot.clone()).build();

    let mut apt = OmaApt::new(vec![], oma_apt_args, false, apt_config)?;

    let (sty, inv) = spinner_style();
    let pb = ProgressBar::new_spinner().with_style(sty);
    pb.enable_steady_tick(inv);
    pb.set_message(fl!("reading-database"));

    let a = apt.available_action()?;
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
    let (execute_apt, install, remove) =
        tui.run(&mut terminal, Duration::from_millis(250)).unwrap();

    exit_tui(&mut terminal).map_err(|e| OutputError {
        description: "BUG: Failed to exit tui".to_string(),
        source: Some(Box::new(e)),
    })?;

    let mut code = 0;

    if execute_apt {
        let fds = if !no_check_dbus {
            let rt = create_async_runtime()?;
            Some(dbus_check(&rt, false)?)
        } else {
            no_check_dbus_warn();
            None
        };

        lock_oma()?;
        apt.upgrade()?;
        apt.install(&install, false)?;
        apt.remove(&remove, false, false)?;

        let apt_args = AptArgs::builder().no_progress(no_progress).build();

        code = CommitRequest {
            apt,
            dry_run,
            typ: SummaryType::Changes,
            apt_args,
            no_fixbroken: false,
            network_thread,
            no_progress,
            sysroot,
            fix_dpkg_status: true,
            protect_essential: true,
            client: &client,
        }
        .run()?;

        drop(fds);
    }

    Ok(code)
}
