use oma_history::SummaryType;
use oma_pm::apt::AptArgsBuilder;
use oma_pm::apt::OmaApt;
use oma_pm::apt::OmaAptArgsBuilder;
use reqwest::Client;
use tracing::info;
use tracing::warn;

use crate::error::OutputError;
use crate::fl;
use crate::utils::create_async_runtime;
use crate::utils::dbus_check;
use crate::utils::root;
use crate::InstallArgs;
use crate::OmaArgs;

use super::utils::handle_no_result;
use super::utils::lock_oma;
use super::utils::no_check_dbus_warn;
use super::utils::normal_commit;
use super::utils::refresh;
use super::utils::NormalCommitArgs;

pub fn execute(
    input: Vec<String>,
    args: InstallArgs,
    oma_args: OmaArgs,
    client: Client,
) -> Result<i32, OutputError> {
    root()?;
    lock_oma()?;

    let OmaArgs {
        dry_run,
        network_thread,
        no_progress,
        no_check_dbus,
        ..
    } = oma_args;

    let fds = if !no_check_dbus {
        let rt = create_async_runtime()?;
        dbus_check(&rt, args.yes)?
    } else {
        no_check_dbus_warn();
        None
    };

    if !args.no_refresh {
        refresh(&client, dry_run, no_progress, network_thread, &args.sysroot)?;
    }

    if args.yes {
        warn!("{}", fl!("automatic-mode-warn"));
    }

    let local_debs = input
        .iter()
        .filter(|x| x.ends_with(".deb"))
        .map(|x| x.to_owned())
        .collect::<Vec<_>>();

    let pkgs_unparse = input.iter().map(|x| x.as_str()).collect::<Vec<_>>();

    let oma_apt_args = OmaAptArgsBuilder::default()
        .sysroot(args.sysroot.clone())
        .install_recommends(args.install_recommends)
        .install_suggests(args.install_suggests)
        .no_install_recommends(args.no_install_recommends)
        .no_install_suggests(args.no_install_suggests)
        .build()?;

    let mut apt = OmaApt::new(local_debs, oma_apt_args, dry_run)?;
    let (pkgs, no_result) = apt.select_pkg(&pkgs_unparse, args.install_dbg, true, false)?;
    handle_no_result(no_result);

    let no_marked_install = apt.install(&pkgs, args.reinstall)?;

    if !no_marked_install.is_empty() {
        for (pkg, version) in no_marked_install {
            info!(
                "{}",
                fl!("already-installed", name = pkg, version = version)
            );
        }
    }

    let apt_args = AptArgsBuilder::default()
        .yes(args.yes)
        .force_yes(args.force_yes)
        .dpkg_force_all(args.dpkg_force_all)
        .dpkg_force_confnew(args.force_confnew)
        .no_progress(no_progress)
        .build()?;

    let args = NormalCommitArgs {
        apt,
        dry_run,
        typ: SummaryType::Install(
            pkgs.iter()
                .map(|x| format!("{} {}", x.raw_pkg.name(), x.version_raw.version()))
                .collect::<Vec<_>>(),
        ),
        apt_args,
        no_fixbroken: args.no_fixbroken,
        network_thread,
        no_progress,
        sysroot: args.sysroot,
        fix_dpkg_status: true,
    };

    normal_commit(args, &client)?;

    drop(fds);

    Ok(0)
}
