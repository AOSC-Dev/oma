use anyhow::anyhow;
use dialoguer::console::style;
use dialoguer::theme::ColorfulTheme;
use dialoguer::{Confirm, Input};
use oma_history::SummaryType;
use oma_pm::apt::{AptArgsBuilder, OmaApt, OmaAptArgsBuilder};
use tracing::{info, warn};

use crate::fl;
use crate::{
    error::OutputError,
    utils::{create_async_runtime, dbus_check, root},
    RemoveArgs,
};

use super::utils::{handle_no_result, normal_commit, NormalCommitArgs};

pub fn execute(
    pkgs: Vec<&str>,
    args: RemoveArgs,
    dry_run: bool,
    protect: bool,
    network_thread: usize,
    no_progress: bool,
) -> Result<i32, OutputError> {
    root()?;

    let rt = create_async_runtime()?;
    dbus_check(&rt)?;

    if args.yes {
        warn!("{}", fl!("automatic-mode-warn"));
    }

    let oma_apt_args = OmaAptArgsBuilder::default()
        .sysroot(args.sysroot.clone())
        .build()?;
    let mut apt = OmaApt::new(vec![], oma_apt_args, dry_run)?;
    let (pkgs, no_result) = apt.select_pkg(&pkgs, false, true, false)?;
    handle_no_result(no_result);

    let context = apt.remove(&pkgs, args.remove_config, args.no_autoremove, |pkg| {
        if protect {
            true
        } else {
            ask_user_do_as_i_say(pkg).unwrap_or(false)
        }
    })?;

    if !context.is_empty() {
        for c in context {
            info!("{}", fl!("no-need-to-remove", name = c));
        }
    }

    let args = NormalCommitArgs {
        apt,
        dry_run,
        typ: SummaryType::Remove(
            pkgs.iter()
                .map(|x| format!("{} {}", x.raw_pkg.name(), x.version_raw.version()))
                .collect::<Vec<_>>(),
        ),
        apt_args: AptArgsBuilder::default()
            .yes(args.yes)
            .force_yes(args.force_yes)
            .no_progress(no_progress)
            .build()?,
        no_fixbroken: false,
        network_thread,
        no_progress,
        sysroot: args.sysroot,
    };

    normal_commit(args)?;

    Ok(0)
}

/// "Yes Do as I say" steps
fn ask_user_do_as_i_say(pkg: &str) -> anyhow::Result<bool> {
    let theme = ColorfulTheme::default();
    let delete = Confirm::with_theme(&theme)
        .with_prompt(format!("DELETE THIS PACKAGE? PACKAGE {pkg} IS ESSENTIAL!",))
        .default(false)
        .interact()
        .map_err(|_| anyhow!(""))?;
    if !delete {
        info!("Not confirmed.");
        return Ok(false);
    }
    info!(
        "If you are absolutely sure, please type the following: {}",
        style("Do as I say!").bold()
    );

    if Input::<String>::with_theme(&theme)
        .with_prompt("Your turn")
        .interact()?
        != "Do as I say!"
    {
        info!("Prompt answered incorrectly. Not confirmed.");
        return Ok(false);
    }

    Ok(true)
}
