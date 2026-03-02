use clap::Args;
use oma_pm::apt::{AptConfig, OmaApt, OmaAptArgs};
use spdlog::info;

use crate::config::OmaConfig;
use crate::utils::ExitHandle;
use crate::{HTTP_CLIENT, fl, success};
use crate::{error::OutputError, utils::root};

use super::utils::{Refresh as RefreshInner, auth_config, create_progress_spinner, space_tips};
use crate::args::CliExecuter;

#[derive(Debug, Args)]
pub struct Refresh;

impl CliExecuter for Refresh {
    fn execute(self, config: OmaConfig) -> Result<ExitHandle, OutputError> {
        if config.dry_run {
            info!("Running in dry-run mode, Exit.");
            return Ok(ExitHandle::default());
        }

        root()?;

        let apt_config = AptConfig::new();
        let auth_config = auth_config(&config.sysroot);
        let auth_config = auth_config.as_ref();

        let sysroot_str = config.sysroot.to_string_lossy();
        let builder = RefreshInner::builder()
            .client(&HTTP_CLIENT)
            .dry_run(false)
            .no_progress(config.no_progress())
            .network_thread(config.download_threads)
            .sysroot(&sysroot_str)
            .config(&apt_config)
            .apt_options(config.apt_options.clone())
            .maybe_auth_config(auth_config);

        #[cfg(feature = "aosc")]
        let refresh = builder.refresh_topics(!config.no_refresh_topics).build();

        #[cfg(not(feature = "aosc"))]
        let refresh = builder.build();

        refresh.run()?;

        let oma_apt_args = OmaAptArgs::builder()
            .sysroot(config.sysroot.to_string_lossy().to_string())
            .build();

        let apt = OmaApt::new(vec![], oma_apt_args, false, apt_config)?;

        let pb = create_progress_spinner(config.no_progress(), fl!("reading-database"));

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

        space_tips(&apt, config.sysroot);

        Ok(ExitHandle::default().ring(true))
    }
}
