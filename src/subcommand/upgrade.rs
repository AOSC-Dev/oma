use crate::completions::pkgnames_and_path_completions;
use crate::config::OmaConfig;
use crate::core::commit_changes::CommitChanges;
use crate::core::refresh::Refresh;
use crate::exit_handle::ExitHandle;
use clap_complete::ArgValueCompleter;
use oma_pm::oma_apt::PackageSort;
use spdlog::{debug, info, warn};

use apt_auth_config::AuthConfig;
use clap::Args;
use oma_pm::apt::AptConfig;
use oma_pm::apt::OmaApt;
use oma_pm::apt::OmaAptArgs;
use oma_pm::apt::Upgrade as AptUpgrade;

use oma_pm::matches::GetArchMethod;
use oma_pm::matches::PackagesMatcher;

use crate::dbus::dbus_check;
use crate::error::OutputError;
use crate::fl;
use crate::root::root;

use super::utils::handle_no_result;
use super::utils::lock_oma;
use crate::args::CliExecuter;

#[derive(Debug, Args)]
pub(crate) struct Upgrade {
    /// Package(s) to install
    #[arg(add = ArgValueCompleter::new(pkgnames_and_path_completions), help = fl!("clap-install-packages-help"))]
    #[arg(help_heading = &**crate::args::ARG_HELP_HEADING_MUST)]
    packages: Vec<String>,
    /// Do not fix apt broken status
    #[arg(long, help = fl!("clap-no-fixbroken-help"))]
    no_fixbroken: bool,
    /// Do not fix dpkg broken status
    #[arg(long, help = fl!("clap-no-fix-dpkg-status-help"))]
    no_fix_dpkg_status: bool,
    /// Install package(s) without fsync(2)
    #[arg(
        long,
        help = &**crate::args::FORCE_UNSAFE_IO_TRANSLATE
    )]
    force_unsafe_io: bool,
    /// Do not refresh repository metadata
    #[arg(long, help = fl!("clap-no-refresh-help"))]
    no_refresh: bool,
    /// Ignore repository and package dependency issues
    #[arg(long, help = fl!("clap-force-yes-help"))]
    force_yes: bool,
    /// Replace configuration file(s) in the system those shipped in the package(s) to be installed (invokes `dpkg --force-confnew`)
    #[arg(long, help = fl!("clap-force-confnew-help"))]
    force_confnew: bool,
    /// Auto remove unnecessary package(s)
    #[arg(long, help = fl!("clap-autoremove-help"))]
    autoremove: bool,
    /// Remove package(s) also remove configuration file(s), like apt purge
    #[arg(long, visible_alias = "purge", help = fl!("clap-remove-config-help"))]
    remove_config: bool,
    /// Bypass confirmation prompts
    #[arg(short, long, help = fl!("clap-yes-help"))]
    yes: bool,
    #[cfg(not(feature = "aosc"))]
    /// Do not allow removal of packages during upgrade (like `apt upgrade')
    #[arg(long, help = fl!("clap-no-remove-help"))]
    no_remove: bool,
    /// Only download dependencies, not install
    #[arg(long, short, help = fl!("clap-download-only-help"))]
    download_only: bool,
}

impl CliExecuter for Upgrade {
    fn execute(self, config: OmaConfig) -> Result<ExitHandle, OutputError> {
        let Upgrade {
            no_fixbroken,
            force_unsafe_io,
            no_refresh,
            force_yes,
            force_confnew,
            autoremove,
            remove_config,
            yes,
            packages,
            #[cfg(not(feature = "aosc"))]
            no_remove,
            no_fix_dpkg_status,
            download_only,
        } = self;

        let _lock_fd = if !config.dry_run {
            root()?;
            Some(lock_oma(&config.sysroot)?)
        } else {
            None
        };

        let _fds = dbus_check(false, &config)?;

        let apt_config = AptConfig::new();

        let auth_config = AuthConfig::system(&config.sysroot)?;

        if !no_refresh {
            Refresh::builder()
                .config(&config)
                .apt_config(&apt_config)
                .auth_config(&auth_config)
                .build()
                .run()?;
        }

        if yes {
            warn!("{}", fl!("automatic-mode-warn"));
        }

        let local_debs = packages
            .iter()
            .filter(|x| x.ends_with(".deb"))
            .map(|x| x.to_owned())
            .collect::<Vec<_>>();

        let pkgs_unparse = packages.iter().map(|x| x.as_str()).collect::<Vec<_>>();

        let oma_apt_args = OmaAptArgs::builder()
            .sysroot(config.sysroot.to_string_lossy().to_string())
            .dpkg_force_confnew(force_confnew)
            .force_yes(force_yes)
            .yes(yes)
            .another_apt_options(&config.apt_options)
            .dpkg_force_unsafe_io(force_unsafe_io)
            .build();

        let mut apt = OmaApt::new(local_debs, oma_apt_args, config.dry_run, AptConfig::new())?;

        let matcher = PackagesMatcher::builder()
            .cache(&apt.cache)
            .filter_candidate(true)
            .filter_downloadable_candidate(false)
            .select_dbg(false)
            .native_arch(GetArchMethod::SpecifySysroot(&config.sysroot))
            .build();

        let (pkgs, no_result) = matcher.match_pkgs_and_versions(pkgs_unparse.clone())?;

        handle_no_result(no_result, config.no_progress())?;

        let no_marked_install = apt.install(&pkgs, false)?;

        if !no_marked_install.is_empty() {
            for (pkg, version) in no_marked_install {
                info!(
                    "{}",
                    fl!("already-installed", name = pkg, version = version)
                );
            }
        }

        #[cfg(feature = "aosc")]
        let mode = AptUpgrade::FullUpgrade;

        #[cfg(not(feature = "aosc"))]
        let mode = if no_remove {
            AptUpgrade::Upgrade
        } else {
            AptUpgrade::FullUpgrade
        };

        debug!("Upgrade mode is using: {:?}", mode);
        apt.upgrade(mode)?;

        let held_count = apt
            .cache
            .packages(&PackageSort::default().upgradable())
            .filter(|pkg| !pkg.marked_upgrade())
            .count();

        let exit = CommitChanges::builder()
            .apt(apt)
            .no_fixbroken(no_fixbroken)
            .check_tum(true)
            .yes(yes)
            .remove_config(remove_config)
            .autoremove(autoremove)
            .maybe_auth_config(Some(&auth_config))
            .fix_dpkg_status(!no_fix_dpkg_status)
            .download_only(download_only)
            .is_upgrade(true)
            .config(&config)
            .build()
            .run()?;

        let apt = OmaApt::new(
            vec![],
            OmaAptArgs::builder().build(),
            config.dry_run,
            AptConfig::new(),
        )?;

        let (_, manual_held) = apt.count_pending_upgradable_pkgs();

        if manual_held != 0 {
            info!("{}", fl!("upgrade-after-held-tips", count = manual_held));
        }

        if held_count != manual_held {
            let resolver_held = held_count - manual_held;
            info!("{}", fl!("upgrade-after-held-tips", count = resolver_held));
        }

        Ok(exit)
    }
}
