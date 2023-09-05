use oma_console::info;
use oma_console::warn;
use oma_pm::apt::AptArgsBuilder;
use oma_pm::apt::OmaApt;
use oma_pm::apt::OmaAptArgsBuilder;

use crate::fl;
use crate::history::SummaryType;
use crate::utils::create_async_runtime;
use crate::utils::dbus_check;
use crate::utils::root;
use crate::InstallArgs;
use crate::Result;

use super::utils::handle_no_result;
use super::utils::normal_commit;
use super::utils::refresh;

pub fn execute(
    pkgs_unparse: Vec<String>,
    args: InstallArgs,
    dry_run: bool,
    network_thread: usize,
) -> Result<i32> {
    root()?;

    let rt = create_async_runtime()?;
    dbus_check(&rt)?;

    if !args.no_refresh {
        refresh(dry_run)?;
    }

    if args.yes {
        warn!("{}", fl!("automatic-mode-warn"));
    }

    let local_debs = pkgs_unparse
        .iter()
        .filter(|x| x.ends_with(".deb"))
        .map(|x| x.to_owned())
        .collect::<Vec<_>>();

    let pkgs_unparse = pkgs_unparse.iter().map(|x| x.as_str()).collect::<Vec<_>>();

    let oma_apt_args = OmaAptArgsBuilder::default()
        .install_recommends(args.install_recommends)
        .install_suggests(args.install_suggests)
        .no_install_recommends(args.no_install_recommends)
        .no_install_suggests(args.no_install_suggests)
        .build()?;

    let mut apt = OmaApt::new(local_debs, oma_apt_args, dry_run)?;
    let (pkgs, no_result) = apt.select_pkg(pkgs_unparse, args.install_dbg, true)?;
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
        .build()?;

    normal_commit(
        apt,
        dry_run,
        SummaryType::Install(
            pkgs.iter()
                .map(|x| format!("{} {}", x.raw_pkg.name(), x.version_raw.version()))
                .collect::<Vec<_>>(),
        ),
        apt_args,
        args.no_fixbroken,
        network_thread,
    )?;

    Ok(0)
}
