use clap::Args;
use oma_pm::apt::{AptConfig, OmaApt, OmaAptArgs};
use spdlog::info;

use crate::config::OmaConfig;
use crate::core::space_tips;
use crate::exit_handle::ExitHandle;
use crate::{error::OutputError, root::root};
use crate::{fl, success};

use super::utils::{auth_config, create_progress_spinner};
use crate::args::CliExecuter;
use crate::core::refresh::Refresh as RefreshInner;

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

        RefreshInner::builder()
            .config(&config)
            .apt_config(&apt_config)
            .maybe_auth_config(auth_config)
            .build()
            .run()?;

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
                "packages-can-be-upgrade-has-manual-held",
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
