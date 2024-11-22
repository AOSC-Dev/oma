use apt_auth_config::AuthConfig;
use oma_history::SummaryType;
use oma_pm::apt::AptConfig;
use oma_pm::apt::OmaApt;
use oma_pm::apt::OmaAptArgs;
use oma_pm::matches::PackagesMatcher;
use oma_utils::dpkg::dpkg_arch;
use tracing::info;
use tracing::warn;

use crate::error::OutputError;
use crate::fl;
use crate::utils::dbus_check;
use crate::utils::root;
use crate::InstallArgs;
use crate::OmaArgs;
use crate::HTTP_CLIENT;

use super::utils::handle_no_result;
use super::utils::lock_oma;
use super::utils::no_check_dbus_warn;
use super::utils::CommitRequest;
use super::utils::RefreshRequest;

pub fn execute(
    input: Vec<String>,
    args: InstallArgs,
    oma_args: OmaArgs,
) -> Result<i32, OutputError> {
    root()?;
    lock_oma()?;

    let OmaArgs {
        dry_run,
        network_thread,
        no_progress,
        no_check_dbus,
        protect_essentials: protect_essential,
        another_apt_options,
        ..
    } = oma_args;

    let fds = if !no_check_dbus {
        Some(dbus_check(args.yes)?)
    } else {
        no_check_dbus_warn();
        None
    };

    let apt_config = AptConfig::new();

    let auth_config = AuthConfig::system(&args.sysroot)?;

    if !args.no_refresh {
        RefreshRequest {
            client: &HTTP_CLIENT,
            dry_run,
            no_progress,
            limit: network_thread,
            sysroot: &args.sysroot,
            _refresh_topics: !args.no_refresh_topic,
            config: &apt_config,
            auth_config: &auth_config,
        }
        .run()?;
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

    let oma_apt_args = OmaAptArgs::builder()
        .sysroot(args.sysroot.clone())
        .install_recommends(args.install_recommends)
        .install_suggests(args.install_suggests)
        .no_install_recommends(args.no_install_recommends)
        .no_install_suggests(args.no_install_suggests)
        .yes(args.yes)
        .force_yes(args.force_yes)
        .dpkg_force_confnew(args.force_confnew)
        .another_apt_options(another_apt_options)
        .dpkg_force_unsafe_io(args.force_unsafe_io)
        .build();

    let mut apt = OmaApt::new(local_debs, oma_apt_args, dry_run, apt_config)?;
    let arch = dpkg_arch(&args.sysroot)?;
    let matcher = PackagesMatcher::builder()
        .cache(&apt.cache)
        .filter_candidate(true)
        .filter_downloadable_candidate(false)
        .select_dbg(args.install_dbg)
        .native_arch(&arch)
        .build();

    let (pkgs, no_result) = matcher.match_pkgs_and_versions(pkgs_unparse)?;

    handle_no_result(&args.sysroot, no_result, no_progress)?;

    let no_marked_install = apt.install(&pkgs, args.reinstall)?;

    if !no_marked_install.is_empty() {
        for (pkg, version) in no_marked_install {
            info!(
                "{}",
                fl!("already-installed", name = pkg, version = version)
            );
        }
    }

    let request = CommitRequest {
        apt,
        dry_run,
        request_type: SummaryType::Install(
            pkgs.iter()
                .map(|x| format!("{} {}", x.raw_pkg.fullname(true), x.version_raw.version()))
                .collect::<Vec<_>>(),
        ),
        no_fixbroken: args.no_fixbroken,
        network_thread,
        no_progress,
        sysroot: args.sysroot,
        fix_dpkg_status: true,
        protect_essential,
        client: &HTTP_CLIENT,
        yes: args.yes,
        remove_config: args.remove_config,
        auth_config: &auth_config,
    };

    let code = request.run()?;

    drop(fds);

    Ok(code)
}
