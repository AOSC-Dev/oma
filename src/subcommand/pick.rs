use dialoguer::{theme::ColorfulTheme, Select};
use oma_pm::{
    apt::{AptArgsBuilder, OmaApt, OmaAptArgsBuilder},
    pkginfo::PkgInfo,
};

use crate::fl;
use crate::{
    error::OutputError,
    history::SummaryType,
    utils::{create_async_runtime, dbus_check, root},
};
use anyhow::anyhow;

use super::utils::{normal_commit, refresh};

pub fn execute(
    pkg_str: &str,
    no_refresh: bool,
    dry_run: bool,
    network_thread: usize,
    no_progress: bool
) -> Result<i32, OutputError> {
    root()?;

    let rt = create_async_runtime()?;
    dbus_check(&rt)?;

    if !no_refresh {
        refresh(dry_run, no_progress)?;
    }

    let oma_apt_args = OmaAptArgsBuilder::default().build()?;
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
    let mut dialoguer = Select::with_theme(&theme);
    dialoguer.items(&versions_str);
    dialoguer.with_prompt(fl!("pick-tips", pkgname = pkg.name()));

    let pos = if let Some(installed) = pkg.installed() {
        versions_str
            .iter()
            .position(|x| x == installed.version())
            .unwrap_or(0)
    } else {
        0
    };

    dialoguer.default(pos);
    let sel = dialoguer.interact()?;
    let version = pkg.get_version(&versions_str[sel]).ok_or_else(|| {
        anyhow!(fl!(
            "can-not-get-pkg-version-from-database",
            name = pkg_str,
            version = versions_str[sel].clone()
        ))
    })?;

    let pkgs = vec![PkgInfo::new(&apt.cache, version.unique(), &pkg)];
    apt.install(&pkgs, false)?;

    normal_commit(
        apt,
        dry_run,
        SummaryType::Install(
            pkgs.iter()
                .map(|x| format!("{} {}", x.raw_pkg.name(), x.version_raw.version()))
                .collect(),
        ),
        AptArgsBuilder::default().build()?,
        false,
        network_thread,
        no_progress
    )?;

    Ok(0)
}
