use dialoguer::{theme::ColorfulTheme, Select};
use oma_history::SummaryType;
use oma_pm::{
    apt::{AptArgsBuilder, OmaApt, OmaAptArgsBuilder},
    pkginfo::PkgInfo,
};

use crate::{
    error::OutputError,
    utils::{create_async_runtime, dbus_check, root},
};
use crate::{fl, OmaArgs};
use anyhow::anyhow;

use super::utils::{no_check_dbus_warn, normal_commit, refresh, NormalCommitArgs};

pub fn execute(
    pkg_str: &str,
    no_refresh: bool,
    oma_args: OmaArgs,
    sysroot: String,
) -> Result<i32, OutputError> {
    root()?;

    let OmaArgs {
        dry_run,
        network_thread,
        no_progress,
        download_pure_db,
        no_check_dbus,
	review_style,
        ..
    } = oma_args;

    if !no_check_dbus {
        let rt = create_async_runtime()?;
        dbus_check(&rt, false)?;
    } else {
        no_check_dbus_warn();
    }

    if !no_refresh {
        refresh(dry_run, no_progress, download_pure_db, &sysroot)?;
    }

    let oma_apt_args = OmaAptArgsBuilder::default()
        .sysroot(sysroot.clone())
        .build()?;
    let mut apt = OmaApt::new(vec![], oma_apt_args, dry_run)?;
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
        let uri_a = versions[a].uris().next().unwrap();
        version_str_display[a] = format!("{} (from: {uri_a})", versions_str[a]);

        let uri_b = versions[b].uris().next().unwrap();
        version_str_display[b] = format!("{} (from: {uri_b})", versions_str[b]);
    }

    let theme = ColorfulTheme::default();
    let mut dialoguer = Select::with_theme(&theme)
        .items(&versions_str)
        .with_prompt(fl!("pick-tips", pkgname = pkg.name()));

    let pos = if let Some(installed) = pkg.installed() {
        versions_str
            .iter()
            .position(|x| x == installed.version())
            .unwrap_or(0)
    } else {
        0
    };

    dialoguer = dialoguer.default(pos);
    let sel = dialoguer.interact().map_err(|_| anyhow!(""))?;
    let version = pkg.get_version(&versions_str[sel]).unwrap();

    let pkgs = vec![PkgInfo::new(&version, &pkg)];
    apt.install(&pkgs, false)?;

    let args = NormalCommitArgs {
        apt,
        dry_run,
        typ: SummaryType::Install(
            pkgs.iter()
                .map(|x| format!("{} {}", x.raw_pkg.name(), x.version_raw.version()))
                .collect::<Vec<_>>(),
        ),
        apt_args: AptArgsBuilder::default().no_progress(no_progress).build()?,
        no_fixbroken: false,
        network_thread,
        no_progress,
        sysroot,
	review_style,
    };

    normal_commit(args)?;

    Ok(0)
}
