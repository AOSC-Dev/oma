use oma_console::{info, warn};
use oma_pm::apt::{AptArgsBuilder, OmaApt, OmaAptArgsBuilder};

use crate::fl;
use crate::{
    error::OutputError,
    history::SummaryType,
    utils::{create_async_runtime, dbus_check, root},
    RemoveArgs,
};

use super::utils::{handle_no_result, normal_commit};

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

    let oma_apt_args = OmaAptArgsBuilder::default().build()?;
    let mut apt = OmaApt::new(vec![], oma_apt_args, dry_run)?;
    let (pkgs, no_result) = apt.select_pkg(pkgs, false, true)?;
    handle_no_result(no_result);

    let context = apt.remove(&pkgs, !args.keep_config, protect, true, args.no_autoremove)?;

    if !context.is_empty() {
        for c in context {
            info!("{}", fl!("no-need-to-remove", name = c));
        }
    }

    normal_commit(
        apt,
        dry_run,
        SummaryType::Remove(
            pkgs.iter()
                .map(|x| format!("{} {}", x.raw_pkg.name(), x.version_raw.version()))
                .collect::<Vec<_>>(),
        ),
        AptArgsBuilder::default()
            .yes(args.yes)
            .force_yes(args.force_yes)
            .no_progress(no_progress)
            .build()?,
        false,
        network_thread,
        no_progress
    )?;

    Ok(0)
}
