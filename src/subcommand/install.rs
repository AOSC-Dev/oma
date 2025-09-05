use std::path::PathBuf;

use clap::Args;
use clap_complete::ArgValueCompleter;
use oma_pm::apt::AptConfig;
use oma_pm::apt::OmaApt;
use oma_pm::apt::OmaAptArgs;
use oma_pm::matches::GetArchMethod;
use oma_pm::matches::PackagesMatcher;
use tracing::info;
use tracing::warn;

use crate::HTTP_CLIENT;
use crate::config::Config;
use crate::error::OutputError;
use crate::fl;
use crate::utils::dbus_check;
use crate::utils::pkgnames_and_path_completions;
use crate::utils::root;

use super::utils::CommitChanges;
use super::utils::Refresh;
use super::utils::auth_config;
use super::utils::handle_no_result;
use super::utils::lock_oma;
use crate::args::CliExecuter;

#[derive(Debug, Args)]
pub struct Install {
    /// Package(s) to install
    #[arg(
        add = ArgValueCompleter::new(pkgnames_and_path_completions),
        help = fl!("clap-install-packages-help"),
        value_name = fl!("clap-packages-value-name")
    )]
    packages: Vec<String>,
    /// Install recommended package(s)
    #[arg(long, help = fl!("clap-install-recommends-help"))]
    install_recommends: bool,
    /// Reinstall package(s)
    #[arg(long, requires = "packages", help = fl!("clap-reinstall-help"))]
    reinstall: bool,
    /// Install suggested package(s)
    #[arg(long, help = fl!("clap-install-suggests-help"))]
    install_suggests: bool,
    /// Do not install recommended package(s)
    #[arg(long, conflicts_with = "install_recommends", help = fl!("clap-no-install-recommends-help"))]
    no_install_recommends: bool,
    /// Do not install suggested package(s)
    #[arg(long, conflicts_with = "install_suggests", help = fl!("clap-no-install-suggests-help"))]
    no_install_suggests: bool,
    /// Bypass confirmation prompts
    #[arg(short, long, help = fl!("clap-yes-help"))]
    yes: bool,
    /// Install debug symbol package
    #[arg(long, help = fl!("clap-install-dbg-help"))]
    install_dbg: bool,
    /// Run oma in "dry-run" mode. Useful for testing changes and operations without making changes to the system
    #[arg(from_global, help = fl!("clap-dry-run-help"), long_help = fl!("clap-dry-run-long-help"))]
    dry_run: bool,
    /// Run oma do not check dbus
    #[arg(from_global, help = fl!("clap-no-check-dbus-help"))]
    no_check_dbus: bool,
    /// Set sysroot target directory
    #[arg(from_global, help = fl!("clap-sysroot-help"))]
    sysroot: PathBuf,
    /// Set apt options
    #[arg(from_global, help = fl!("clap-apt-options-help"))]
    apt_options: Vec<String>,
    /// Resolve broken dependencies in the system
    #[arg(short, long, help = fl!("clap-fix-broken-help"))]
    fix_broken: bool,
    /// Do not fix dpkg broken status
    #[arg(short, long, help = fl!("clap-no-fix-dpkg-status-help"))]
    no_fix_dpkg_status: bool,
    /// Install package(s) without fsync(2)
    #[arg(long, help = fl!("clap-force-unsafe-io-help"))]
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
    #[cfg(feature = "aosc")]
    /// Do not refresh topics manifest.json file
    #[arg(long, help = fl!("clap-no-refresh-topics-help"))]
    no_refresh_topics: bool,
    /// Auto remove unnecessary package(s)
    #[arg(long, help = fl!("clap-autoremove-help"))]
    autoremove: bool,
    /// Remove package(s) also remove configuration file(s), like apt purge
    #[arg(long, visible_alias = "purge", help = fl!("clap-remove-config-help"))]
    remove_config: bool,
    /// Setup download threads (default as 4)
    #[arg(from_global, help = fl!("clap-download-threads-help"))]
    download_threads: Option<usize>,
    /// Run oma do not check battery status
    #[arg(from_global, help = fl!("clap-no-check-battery-help"))]
    no_check_battery: bool,
    /// Run oma do not take wake lock
    #[arg(from_global, help = fl!(clap-no-take-wake-lock-help))]
    no_take_wake_lock: bool,
}

impl CliExecuter for Install {
    fn execute(self, config: &Config, no_progress: bool) -> Result<i32, OutputError> {
        let Install {
            packages,
            install_recommends,
            reinstall,
            install_suggests,
            no_install_recommends,
            no_install_suggests,
            yes,
            install_dbg,
            dry_run,
            no_check_dbus,
            sysroot,
            apt_options,
            fix_broken,
            force_unsafe_io,
            no_refresh,
            force_yes,
            force_confnew,
            #[cfg(feature = "aosc")]
            no_refresh_topics,
            autoremove,
            remove_config,
            no_fix_dpkg_status,
            download_threads,
            no_check_battery,
            no_take_wake_lock,
        } = self;

        if !dry_run {
            root()?;
            lock_oma()?;
        }

        let _fds = dbus_check(
            yes,
            config,
            no_check_dbus,
            dry_run,
            no_take_wake_lock,
            no_check_battery,
        )?;

        let apt_config = AptConfig::new();

        let auth_config = auth_config(&sysroot);
        let auth_config = auth_config.as_ref();

        if !no_refresh {
            let sysroot = sysroot.to_string_lossy();
            let builder = Refresh::builder()
                .client(&HTTP_CLIENT)
                .dry_run(dry_run)
                .no_progress(no_progress)
                .network_thread(download_threads.unwrap_or_else(|| config.network_thread()))
                .sysroot(&sysroot)
                .config(&apt_config)
                .maybe_auth_config(auth_config);

            #[cfg(feature = "aosc")]
            let refresh = builder
                .refresh_topics(!no_refresh_topics && !config.no_refresh_topics())
                .build();

            #[cfg(not(feature = "aosc"))]
            let refresh = builder.build();

            refresh.run()?;
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
            .sysroot(sysroot.to_string_lossy().to_string())
            .install_recommends(install_recommends)
            .install_suggests(install_suggests)
            .no_install_recommends(no_install_recommends)
            .no_install_suggests(no_install_suggests)
            .yes(yes)
            .force_yes(force_yes)
            .dpkg_force_confnew(force_confnew)
            .another_apt_options(apt_options)
            .dpkg_force_unsafe_io(force_unsafe_io)
            .build();

        let mut apt = OmaApt::new(local_debs, oma_apt_args, dry_run, apt_config)?;
        let matcher = PackagesMatcher::builder()
            .cache(&apt.cache)
            .filter_candidate(true)
            .filter_downloadable_candidate(false)
            .select_dbg(install_dbg)
            .native_arch(GetArchMethod::SpecifySysroot(&sysroot))
            .build();

        let (pkgs, no_result) = matcher.match_pkgs_and_versions(pkgs_unparse)?;

        handle_no_result(&sysroot, no_result, no_progress)?;

        let no_marked_install = apt.install(&pkgs, reinstall)?;

        if !no_marked_install.is_empty() {
            for (pkg, version) in no_marked_install {
                info!(
                    "{}",
                    fl!("already-installed", name = pkg, version = version)
                );
            }
        }

        CommitChanges::builder()
            .apt(apt)
            .dry_run(dry_run)
            .no_fixbroken(!fix_broken)
            .no_progress(no_progress)
            .sysroot(sysroot.to_string_lossy().to_string())
            .protect_essential(config.protect_essentials())
            .yes(yes)
            .remove_config(remove_config)
            .autoremove(autoremove)
            .network_thread(download_threads.unwrap_or_else(|| config.network_thread()))
            .maybe_auth_config(auth_config)
            .fix_dpkg_status(!no_fix_dpkg_status)
            .build()
            .run()
    }
}
