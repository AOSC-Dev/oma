use apt_auth_config::AuthConfig;
use chrono::Local;
use oma_console::pager::PagerExit;
use oma_console::print::Action;
use oma_console::success;
use oma_fetch::DownloadProgressControl;
use oma_history::connect_db;
use oma_history::create_db_file;
use oma_history::write_history_entry;
use oma_history::SummaryType;
use oma_pm::apt::AptConfig;
use oma_pm::apt::CommitDownloadConfig;
use oma_pm::apt::OmaApt;
use oma_pm::apt::OmaAptArgs;
use oma_pm::apt::OmaAptError;

use oma_pm::apt::SummarySort;
#[cfg(feature = "aosc")]
use oma_pm::apt::Upgrade;

use oma_pm::matches::PackagesMatcher;
use oma_utils::dpkg::dpkg_arch;
#[cfg(not(feature = "aosc"))]
use tracing::debug;

use tracing::info;
use tracing::warn;

use crate::color_formatter;
use crate::error::OutputError;
use crate::fl;
use crate::install_progress::NoInstallProgressManager;
use crate::install_progress::OmaInstallProgressManager;
use crate::pb::NoProgressBar;
use crate::pb::OmaMultiProgressBar;
use crate::pb::OmaProgressBar;
use crate::subcommand::utils::autoremovable_tips;
use crate::subcommand::utils::is_terminal;
use crate::table::table_for_install_pending;
use crate::utils::dbus_check;
use crate::utils::root;
use crate::OmaArgs;
use crate::UpgradeArgs;
use crate::HTTP_CLIENT;

use super::remove::ask_user_do_as_i_say;
use super::utils::handle_features;
use super::utils::handle_no_result;
use super::utils::is_nothing_to_do;
use super::utils::lock_oma;
use super::utils::no_check_dbus_warn;
use super::utils::RefreshRequest;

pub fn execute(
    pkgs_unparse: Vec<String>,
    args: UpgradeArgs,
    oma_args: OmaArgs,
) -> Result<i32, OutputError> {
    root()?;
    lock_oma()?;

    let OmaArgs {
        dry_run,
        network_thread,
        no_progress,
        no_check_dbus,
        protect_essentials,
        another_apt_options,
    } = oma_args;

    let fds = if !no_check_dbus {
        Some(dbus_check(args.yes)?)
    } else {
        no_check_dbus_warn();
        None
    };

    let apt_config = AptConfig::new();

    let auth_config = AuthConfig::system(&args.sysroot)?;

    RefreshRequest {
        client: &HTTP_CLIENT,
        dry_run,
        no_progress,
        limit: network_thread,
        sysroot: &args.sysroot,
        _refresh_topics: !args.no_refresh_topcs,
        config: &apt_config,
        auth_config: &auth_config,
    }
    .run()?;

    if args.yes {
        warn!("{}", fl!("automatic-mode-warn"));
    }

    let local_debs = pkgs_unparse
        .iter()
        .filter(|x| x.ends_with(".deb"))
        .map(|x| x.to_owned())
        .collect::<Vec<_>>();

    let pkgs_unparse = pkgs_unparse.iter().map(|x| x.as_str()).collect::<Vec<_>>();
    let mut retry_times = 1;

    let oma_apt_args = OmaAptArgs::builder()
        .sysroot(args.sysroot.clone())
        .dpkg_force_confnew(args.force_confnew)
        .force_yes(args.force_yes)
        .yes(args.yes)
        .another_apt_options(another_apt_options)
        .dpkg_force_unsafe_io(args.force_unsafe_io)
        .build();

    #[cfg(not(feature = "aosc"))]
    debug!("Upgrade mode is using: {:?}", args.mode);

    let arch = dpkg_arch(&args.sysroot)?;

    loop {
        let mut apt = OmaApt::new(
            local_debs.clone(),
            oma_apt_args.clone(),
            dry_run,
            AptConfig::new(),
        )?;

        #[cfg(feature = "aosc")]
        apt.upgrade(Upgrade::FullUpgrade)?;

        #[cfg(not(feature = "aosc"))]
        apt.upgrade(args.mode.into())?;

        let matcher = PackagesMatcher::builder()
            .cache(&apt.cache)
            .filter_candidate(true)
            .filter_downloadable_candidate(false)
            .select_dbg(false)
            .native_arch(&arch)
            .build();

        let (pkgs, no_result) = matcher.match_pkgs(pkgs_unparse.clone())?;

        let no_marked_install = apt.install(&pkgs, false)?;

        if !no_marked_install.is_empty() {
            for (pkg, version) in no_marked_install {
                info!(
                    "{}",
                    fl!("already-installed", name = pkg, version = version)
                );
            }
        }

        handle_no_result(&args.sysroot, no_result, no_progress)?;

        let pb = if !no_progress || is_terminal() {
            OmaProgressBar::new_spinner(Some(fl!("resolving-dependencies"))).into()
        } else {
            None
        };

        apt.resolve(false, args.remove_config)?;

        if args.autoremove {
            apt.autoremove(false)?;
            apt.resolve(false, args.remove_config)?;
        }

        if let Some(pb) = pb {
            pb.inner.finish_and_clear()
        }

        let op = apt.summary(
            SummarySort::Operation,
            |pkg| {
                if protect_essentials {
                    false
                } else {
                    ask_user_do_as_i_say(pkg).unwrap_or(false)
                }
            },
            |features| handle_features(features, protect_essentials).unwrap_or(false),
        )?;

        apt.check_disk_size(&op)?;

        let op_after = op.clone();

        let install = &op.install;
        let remove = &op.remove;
        let disk_size = &op.disk_size;
        let (ar_count, ar_size) = op.autoremovable;

        if is_nothing_to_do(install, remove, false) {
            autoremovable_tips(ar_count, ar_size)?;
            return Ok(0);
        }

        if retry_times == 1 {
            match table_for_install_pending(install, remove, disk_size, !args.yes, dry_run)? {
                PagerExit::NormalExit => {}
                x @ PagerExit::Sigint => return Ok(x.into()),
                x @ PagerExit::DryRun => return Ok(x.into()),
            }
        }

        let typ = SummaryType::Upgrade(
            pkgs.iter()
                .map(|x| format!("{} {}", x.raw_pkg.fullname(true), x.version_raw.version()))
                .collect::<Vec<_>>(),
        );

        let start_time = Local::now().timestamp();

        let progress_manager: &dyn DownloadProgressControl = if !no_progress {
            &OmaMultiProgressBar::default()
        } else {
            &NoProgressBar::default()
        };

        match apt.commit(
            &HTTP_CLIENT,
            CommitDownloadConfig {
                network_thread: Some(network_thread),
                auth: &auth_config,
            },
            progress_manager,
            if no_progress || !is_terminal() {
                Box::new(NoInstallProgressManager)
            } else {
                Box::new(OmaInstallProgressManager)
            },
            op,
        ) {
            Ok(()) => {
                write_history_entry(
                    op_after,
                    typ,
                    {
                        let db = create_db_file(args.sysroot)?;
                        connect_db(db, true)?
                    },
                    dry_run,
                    start_time,
                    true,
                )?;

                let cmd = color_formatter().color_str("oma undo", Action::Emphasis);

                if !dry_run {
                    success!("{}", fl!("history-tips-1"));
                    info!("{}", fl!("history-tips-2", cmd = cmd.to_string()));
                }

                autoremovable_tips(ar_count, ar_size)?;

                drop(fds);
                return Ok(0);
            }
            Err(e) => match e {
                OmaAptError::AptErrors(_)
                | OmaAptError::AptError(_)
                | OmaAptError::AptCxxException(_) => {
                    if retry_times == 3 {
                        write_history_entry(
                            op_after,
                            SummaryType::Upgrade(
                                pkgs.iter()
                                    .map(|x| {
                                        format!(
                                            "{} {}",
                                            x.raw_pkg.fullname(true),
                                            x.version_raw.version()
                                        )
                                    })
                                    .collect::<Vec<_>>(),
                            ),
                            {
                                let db = create_db_file(args.sysroot)?;
                                connect_db(db, true)?
                            },
                            dry_run,
                            start_time,
                            false,
                        )?;
                        let cmd = color_formatter().color_str("oma undo", Action::Emphasis);
                        info!("{}", fl!("history-tips-2", cmd = cmd.to_string()));

                        return Err(OutputError::from(e));
                    }
                    warn!("{e}, retrying ...");
                    retry_times += 1;
                }
                _ => {
                    drop(fds);
                    return Err(OutputError::from(e));
                }
            },
        }
    }
}
