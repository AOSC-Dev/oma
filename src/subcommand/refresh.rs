use std::path::PathBuf;

use clap::Args;
use oma_pm::apt::{AptConfig, OmaApt, OmaAptArgs};
use tracing::info;

use crate::config::Config;
use crate::{error::OutputError, utils::root};
use crate::{fl, success, HTTP_CLIENT};

use super::utils::{auth_config, create_progress_spinner, Refresh as RefreshInner};
use crate::args::CliExecuter;

#[derive(Debug, Args)]
pub struct Refresh {
    #[cfg(feature = "aosc")]
    /// Do not refresh topics manifest.json file
    #[arg(long)]
    no_refresh_topics: bool,
    /// Set sysroot target directory
    #[arg(from_global)]
    sysroot: PathBuf,
    /// Run oma in “dry-run” mode. Useful for testing changes and operations without making changes to the system
    #[arg(from_global)]
    dry_run: bool,
}

impl CliExecuter for Refresh {
    fn execute(self, config: &Config, no_progress: bool) -> Result<i32, OutputError> {
        let Refresh {
            #[cfg(feature = "aosc")]
            no_refresh_topics,
            sysroot,
            dry_run,
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
            .network_thread(config.network_thread())
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

        let upgradable = apt.count_pending_upgradable_pkgs()?;
        let autoremovable = apt.count_pending_autoremovable_pkgs();

        if let Some(pb) = pb {
            pb.inner.finish_and_clear();
        }

        let mut s = vec![];

        if upgradable != 0 {
            s.push(fl!("packages-can-be-upgrade", len = upgradable));
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

        Ok(0)
    }
}
