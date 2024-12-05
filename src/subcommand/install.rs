use std::path::PathBuf;

use apt_auth_config::AuthConfig;
use clap::Args;
use oma_history::SummaryType;
use oma_pm::apt::AptConfig;
use oma_pm::apt::OmaApt;
use oma_pm::apt::OmaAptArgs;
use oma_pm::matches::GetArchMethod;
use oma_pm::matches::PackagesMatcher;
use tracing::info;
use tracing::warn;

use crate::config::Config;
use crate::error::OutputError;
use crate::fl;
use crate::utils::dbus_check;
use crate::utils::root;
use crate::HTTP_CLIENT;

use super::utils::handle_no_result;
use super::utils::lock_oma;
use super::utils::no_check_dbus_warn;
use super::utils::CommitChanges;
use super::utils::Refresh;
use crate::args::CliExecuter;

#[derive(Debug, Args)]
pub struct Install {
    /// Package(s) to install
    packages: Vec<String>,
    /// Install recommended package(s)
    #[arg(long)]
    install_recommends: bool,
    /// Reinstall package(s)
    #[arg(long, requires = "packages")]
    reinstall: bool,
    /// Install suggested package(s)
    #[arg(long)]
    install_suggests: bool,
    /// Do not install recommended package(s)
    #[arg(long, conflicts_with = "install_recommends")]
    no_install_recommends: bool,
    /// Do not install suggested package(s)
    #[arg(long, conflicts_with = "install_suggests")]
    no_install_suggests: bool,
    /// Bypass confirmation prompts
    #[arg(short, long)]
    yes: bool,
    /// Install debug symbol package
    #[arg(long)]
    install_dbg: bool,
    /// Run oma in “dry-run” mode. Useful for testing changes and operations without making changes to the system
    #[arg(from_global)]
    dry_run: bool,
    /// Run oma do not check dbus
    #[arg(from_global)]
    no_check_dbus: bool,
    /// Set sysroot target directory
    #[arg(from_global)]
    sysroot: PathBuf,
    /// Set apt options
    #[arg(from_global)]
    apt_options: Vec<String>,
    /// Fix apt broken status
    #[arg(short, long)]
    fix_broken: bool,
    /// Install package(s) without fsync(2)
    #[arg(long)]
    force_unsafe_io: bool,
    /// Do not refresh repository metadata
    #[arg(long)]
    no_refresh: bool,
    /// Ignore repository and package dependency issues
    #[arg(long)]
    force_yes: bool,
    /// Replace configuration file(s) in the system those shipped in the package(s) to be installed (invokes `dpkg --force-confnew`)
    #[arg(long)]
    force_confnew: bool,
    #[cfg(feature = "aosc")]
    /// Do not refresh topics manifest.json file
    #[arg(long)]
    no_refresh_topics: bool,
    /// Auto remove unnecessary package(s)
    #[arg(long)]
    autoremove: bool,
    /// Remove package(s) also remove configuration file(s), like apt purge
    #[arg(long, visible_alias = "purge")]
    remove_config: bool,
}

impl CliExecuter for Install {
    fn execute(self, config: &Config, no_progress: bool) -> Result<i32, OutputError> {
        root()?;
        lock_oma()?;

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
        } = self;

        let _fds = if !no_check_dbus && !config.no_check_dbus() {
            Some(dbus_check(yes)?)
        } else {
            no_check_dbus_warn();
            None
        };

        let apt_config = AptConfig::new();

        let auth_config = AuthConfig::system(&sysroot)?;

        if !no_refresh {
            let sysroot = sysroot.to_string_lossy();
            let builder = Refresh::builder()
                .client(&HTTP_CLIENT)
                .dry_run(dry_run)
                .no_progress(no_progress)
                .network_thread(config.network_thread())
                .sysroot(&sysroot)
                .config(&apt_config)
                .auth_config(&auth_config);

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
            .request_type(SummaryType::Install(
                pkgs.iter()
                    .map(|x| format!("{} {}", x.raw_pkg.fullname(true), x.version_raw.version()))
                    .collect::<Vec<_>>(),
            ))
            .no_fixbroken(!fix_broken)
            .network_thread(config.network_thread())
            .no_progress(no_progress)
            .sysroot(sysroot.to_string_lossy().to_string())
            .fix_dpkg_status(true)
            .protect_essential(config.protect_essentials())
            .client(&HTTP_CLIENT)
            .yes(yes)
            .remove_config(remove_config)
            .auth_config(&auth_config)
            .autoremove(autoremove)
            .build()
            .run()
    }
}
