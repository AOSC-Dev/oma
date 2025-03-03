use std::path::PathBuf;

use clap::Args;
use dialoguer::{Select, theme::ColorfulTheme};
use oma_pm::{
    apt::{AptConfig, OmaApt, OmaAptArgs},
    pkginfo::OmaPackage,
};

use crate::fl;
use crate::{
    HTTP_CLIENT,
    config::Config,
    error::OutputError,
    utils::{dbus_check, root},
};
use anyhow::anyhow;

use super::utils::{
    CommitChanges, Refresh, auth_config, lock_oma, no_check_dbus_warn, tui_select_list_size,
};
use crate::args::CliExecuter;

#[derive(Debug, Args)]
pub struct Pick {
    /// Package to pick specific version for
    #[arg(required = true)]
    package: String,
    /// Fix apt broken status
    #[arg(short, long)]
    fix_broken: bool,
    /// Do not fix dpkg broken status
    #[arg(short, long)]
    no_fix_dpkg_status: bool,
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
    /// Run oma in "dry-run" mode. Useful for testing changes and operations without making changes to the system
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
}

impl CliExecuter for Pick {
    fn execute(self, config: &Config, no_progress: bool) -> Result<i32, OutputError> {
        let Pick {
            package,
            fix_broken,
            force_unsafe_io,
            no_refresh,
            force_yes,
            force_confnew,
            #[cfg(feature = "aosc")]
            no_refresh_topics,
            autoremove,
            remove_config,
            dry_run,
            no_check_dbus,
            sysroot,
            apt_options,
            no_fix_dpkg_status,
        } = self;

        if !dry_run {
            root()?;
            lock_oma()?;
        }

        let _fds = if !no_check_dbus && !config.no_check_dbus() && !dry_run {
            Some(dbus_check(false)?)
        } else {
            no_check_dbus_warn();
            None
        };

        let apt_config = AptConfig::new();

        let auth_config = auth_config(&sysroot);
        let auth_config = auth_config.as_ref();

        if !no_refresh {
            let sysroot = sysroot.to_string_lossy();
            let builder = Refresh::builder()
                .client(&HTTP_CLIENT)
                .dry_run(dry_run)
                .no_progress(no_progress)
                .network_thread(config.network_thread())
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

        let oma_apt_args = OmaAptArgs::builder()
            .sysroot(sysroot.to_string_lossy().to_string())
            .another_apt_options(apt_options)
            .dpkg_force_confnew(force_confnew)
            .dpkg_force_unsafe_io(force_unsafe_io)
            .force_yes(force_yes)
            .build();
        let mut apt = OmaApt::new(vec![], oma_apt_args, dry_run, apt_config)?;

        let pkg = apt
            .cache
            .get(&package)
            .ok_or_else(|| anyhow!(fl!("can-not-get-pkg-from-database", name = package)))?;

        let versions = pkg.versions().collect::<Vec<_>>();
        let versions_str = versions
            .iter()
            .map(|x| x.version().to_string())
            .collect::<Vec<_>>();

        let mut v = vec![];
        for i in 0..versions.len() {
            for j in 1..versions.len() {
                if i == j {
                    continue;
                }

                if versions_str[i] == versions_str[j] {
                    v.push((i, j));
                }
            }
        }

        let mut version_str_display = versions_str.clone();
        for (a, b) in v {
            if let Some(uri) = versions[a].uris().next() {
                version_str_display[a] = format!("{} (from: {uri})", versions_str[a]);
            }

            if let Some(uri) = versions[b].uris().next() {
                version_str_display[b] = format!("{} (from: {uri})", versions_str[b]);
            }
        }

        let theme = ColorfulTheme::default();
        let mut dialoguer = Select::with_theme(&theme)
            .items(&versions_str)
            .with_prompt(fl!("pick-tips", pkgname = pkg.fullname(true)));

        let pos = if let Some(installed) = pkg.installed() {
            versions_str
                .iter()
                .position(|x| x == installed.version())
                .unwrap_or(0)
        } else {
            0
        };

        dialoguer = dialoguer.default(pos);

        let size = tui_select_list_size();
        dialoguer = dialoguer.max_length(size.into());

        let sel = dialoguer.interact().map_err(|_| anyhow!(""))?;
        let version = pkg.get_version(&versions_str[sel]).unwrap();

        let pkgs = vec![OmaPackage::new(&version, &pkg).map_err(|e| OutputError {
            description: e.to_string(),
            source: None,
        })?];

        apt.install(&pkgs, false)?;

        CommitChanges::builder()
            .apt(apt)
            .dry_run(dry_run)
            .no_fixbroken(fix_broken)
            .no_progress(no_progress)
            .sysroot(sysroot.to_string_lossy().to_string())
            .fix_dpkg_status(!no_fix_dpkg_status)
            .protect_essential(config.protect_essentials())
            .yes(false)
            .remove_config(remove_config)
            .autoremove(autoremove)
            .network_thread(config.network_thread())
            .maybe_auth_config(auth_config)
            .build()
            .run()
    }
}
