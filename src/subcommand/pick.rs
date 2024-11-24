use apt_auth_config::AuthConfig;
use dialoguer::{theme::ColorfulTheme, Select};
use oma_history::SummaryType;
use oma_pm::{
    apt::{AptConfig, OmaApt, OmaAptArgs},
    pkginfo::OmaPackage,
};

use crate::{
    error::OutputError,
    utils::{dbus_check, root},
    HTTP_CLIENT,
};
use crate::{fl, OmaArgs};
use anyhow::anyhow;

use super::utils::{
    lock_oma, no_check_dbus_warn, tui_select_list_size, CommitChanges, RefreshRequest,
};

pub fn execute(
    pkg_str: &str,
    no_refresh: bool,
    oma_args: OmaArgs,
    sysroot: String,
    no_refresh_topic: bool,
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

    let _fds = if !no_check_dbus {
        Some(dbus_check(false)?)
    } else {
        no_check_dbus_warn();
        None
    };

    let apt_config = AptConfig::new();

    let auth_config = AuthConfig::system(&sysroot)?;

    if !no_refresh {
        RefreshRequest {
            client: &HTTP_CLIENT,
            dry_run,
            no_progress,
            limit: network_thread,
            sysroot: &sysroot,
            _refresh_topics: !no_refresh_topic,
            config: &apt_config,
            auth_config: &auth_config,
        }
        .run()?;
    }

    let oma_apt_args = OmaAptArgs::builder()
        .sysroot(sysroot.clone())
        .another_apt_options(another_apt_options)
        .build();
    let mut apt = OmaApt::new(vec![], oma_apt_args, dry_run, apt_config)?;
    let pkg = apt
        .cache
        .get(pkg_str)
        .ok_or_else(|| anyhow!(fl!("can-not-get-pkg-from-database", name = pkg_str)))?;

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
        .request_type(SummaryType::Remove(
            pkgs.iter()
                .map(|x| format!("{} {}", x.raw_pkg.fullname(true), x.version_raw.version()))
                .collect::<Vec<_>>(),
        ))
        .no_fixbroken(false)
        .network_thread(network_thread)
        .no_progress(no_progress)
        .sysroot(sysroot)
        .fix_dpkg_status(true)
        .protect_essential(protect_essential)
        .client(&HTTP_CLIENT)
        .yes(false)
        .remove_config(false)
        .auth_config(&auth_config)
        .build()
        .run()
}
