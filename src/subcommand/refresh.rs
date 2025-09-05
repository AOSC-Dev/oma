use std::path::PathBuf;

use clap::Args;
use oma_pm::apt::{AptConfig, OmaApt, OmaAptArgs};
use tracing::info;

use crate::config::Config;
use crate::{HTTP_CLIENT, fl, success};
use crate::{error::OutputError, utils::root};

use super::utils::{Refresh as RefreshInner, auth_config, create_progress_spinner, space_tips};
use crate::args::CliExecuter;

#[derive(Debug, Args)]
pub struct Refresh {
    #[cfg(feature = "aosc")]
    /// Do not refresh topics manifest.json file
    #[arg(long, help = fl!("clap-no-refresh-topics-help"))]
    no_refresh_topics: bool,
    /// Set sysroot target directory
    #[arg(from_global, help = fl!("clap-sysroot-help"))]
    sysroot: PathBuf,
    /// Run oma in "dry-run" mode. Useful for testing changes and operations without making changes to the system
    #[arg(from_global, help = fl!("clap-dry-run-help"), long_help = fl!("clap-dry-run-long-help"))]
    dry_run: bool,
    /// Setup download threads (default as 4)
    #[arg(from_global, help = fl!("clap-download-threads-help"))]
    download_threads: Option<usize>,
    /// Print help
    #[arg(long, short, action = clap::ArgAction::HelpLong, help = fl!("clap-help"))]
    help: bool,
}

impl CliExecuter for Refresh {
    fn execute(self, config: &Config, no_progress: bool) -> Result<i32, OutputError> {
        let Refresh {
            #[cfg(feature = "aosc")]
            no_refresh_topics,
            sysroot,
            dry_run,
            download_threads,
            ..
        } = self;

        if dry_run {
            info!("Running in dry-run mode, Exit.");
            return Ok(0);
        }

        root()?;

        let apt_config = AptConfig::new();
        let auth_config = auth_config(&sysroot);
        let auth_config = auth_config.as_ref();

        let sysroot_str = sysroot.to_string_lossy();
        let builder = RefreshInner::builder()
            .client(&HTTP_CLIENT)
            .dry_run(false)
            .no_progress(no_progress)
            .network_thread(download_threads.unwrap_or_else(|| config.network_thread()))
            .sysroot(&sysroot_str)
            .config(&apt_config)
            .maybe_auth_config(auth_config);

        #[cfg(feature = "aosc")]
        let refresh = builder
            .refresh_topics(!no_refresh_topics && !config.no_refresh_topics())
            .build();

        #[cfg(not(feature = "aosc"))]
        let refresh = builder.build();

        refresh.run()?;

        let oma_apt_args = OmaAptArgs::builder()
            .sysroot(sysroot.to_string_lossy().to_string())
            .build();

        let apt = OmaApt::new(vec![], oma_apt_args, false, apt_config)?;

        let pb = create_progress_spinner(no_progress, fl!("reading-database"));

        let (upgradable, upgradable_but_held) = apt.count_pending_upgradable_pkgs();
        let autoremovable = apt.count_pending_autoremovable_pkgs();

        if let Some(pb) = pb {
            pb.inner.finish_and_clear();
        }

        let mut s = vec![];

        if upgradable != 0 && upgradable_but_held == 0 {
            s.push(fl!("packages-can-be-upgrade", len = upgradable));
        } else if upgradable != 0 {
            s.push(fl!(
                "packages-can-be-upgrade-has-held",
                len = upgradable,
                held_count = upgradable_but_held
            ));
        }

        if autoremovable != 0 {
            s.push(fl!("packages-can-be-removed", len = autoremovable));
        }

        if s.is_empty() {
            success!("{}", fl!("successfully-refresh"));
        } else {
            let s = s.join(&fl!("comma"));
            success!("{}", fl!("successfully-refresh-with-tips", s = s));
        }

        space_tips(&apt, sysroot);

        Ok(0)
    }
}
