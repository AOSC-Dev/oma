use std::path::Path;

use anyhow::{bail, Context, Result};
use apt_sources_lists::SourceEntry;
use indexmap::IndexMap;
use log::warn;
use reqwest::blocking::Client;
use rust_apt::{
    cache::{Cache, Upgrade},
    new_cache,
    raw::progress::AptInstallProgress,
    util::{apt_lock, apt_unlock, apt_unlock_inner},
};

use crate::{
    formatter::NoProgress,
    pkgversion::PkgVersion,
    update::{
        get_sources, get_sources_dists_filename, newest_package_list, package_list,
        packages_download, update_db, APT_LIST_DISTS,
    },
};

pub struct AoscptAction {
    sources: Vec<SourceEntry>,
    db: Vec<IndexMap<String, String>>,
    client: Client,
}

impl AoscptAction {
    pub fn new() -> Result<Self> {
        let client = reqwest::blocking::ClientBuilder::new()
            .user_agent("aoscpt")
            .build()?;

        let sources = get_sources()?;
        let db_paths = get_sources_dists_filename(&sources)?;

        let db = package_list(
            db_paths
                .iter()
                .map(|x| Path::new(APT_LIST_DISTS).join(x))
                .collect(),
        )?;

        Ok(Self {
            sources,
            db,
            client,
        })
    }

    /// Update mirror database and Get all update, like apt update && apt full-upgrade
    pub fn update(&self) -> Result<()> {
        update_db(&self.sources, &self.client)?;

        let cache = new_cache!()?;
        cache.upgrade(&Upgrade::FullUpgrade)?;

        let action = apt_handler(&cache);

        let mut list = action.update.clone();
        list.extend(action.install.clone());

        let db_for_updates = newest_package_list(&self.db)?;
        packages_download(&list, &db_for_updates, &self.sources, &self.client)?;

        cache.resolve(true)?;
        apt_install(action, cache)?;


        Ok(())
    }

    pub fn install(&self, list: &[String]) -> Result<()> {
        update_db(&self.sources, &self.client)?;

        let cache = new_cache!()?;

        for i in list {
            let pkg = cache
                .get(i)
                .take()
                .context(format!("Can not get package: {}", i))?;
            if i.contains("=") {
                // Support apt install fish=3.6.0

                let mut split_arg = i.split("=");
                let name = split_arg.nth(0).context(format!("Not Support: {}", i))?;
                let version = split_arg.nth(1).context(format!("Not Support: {}", i))?;

                if PkgVersion::try_from(version).is_err() {
                    bail!("invalid version: {}", version);
                }

                let version = pkg
                    .get_version(version)
                    .context(format!("Can not get package {} version: {}", name, version))?;

                let pkg = version.parent();
                pkg.mark_install(true, true);
                pkg.protect();
            } else if i.contains("/") {
                // Support apt install fish/stable
                let mut split_arg = i.split("/");
                let name = split_arg.nth(0).context(format!("Not Support: {}", i))?;
                let branch = split_arg.nth(1).context(format!("Not Support: {}", i))?;

                let mut res = self
                    .db
                    .iter()
                    .filter(|x| x.get("Package") == Some(&name.to_string()))
                    .filter(|x| x["Filename"].split("/").nth(1) == Some(branch))
                    .collect::<Vec<_>>();

                if res.is_empty() {
                    bail!("Can not get package {} with {} branch.", name, branch);
                }

                res.sort_by(|x, y| {
                    PkgVersion::try_from(x["Version"].as_str())
                        .unwrap()
                        .cmp(&PkgVersion::try_from(y["Version"].as_str()).unwrap())
                });

                let res = res.last().unwrap()["Version"].to_string();
                let version = pkg.get_version(&res).unwrap();

                let pkg = version.parent();
                pkg.mark_install(true, true);
                pkg.protect();
            }

            pkg.mark_install(true, true);
            pkg.protect();
        }

        let action = apt_handler(&cache);
        let mut list = vec![];
        list.extend(action.install.clone());
        list.extend(action.update.clone());

        cache.resolve(true)?;
        packages_download(&list, &self.db, &self.sources, &self.client)?;
        apt_install(action, cache)?;

        Ok(())
    }

    pub fn remove(&self, list: &[String], is_purge: bool) -> Result<()> {
        let cache = new_cache!()?;

        for i in list {
            let pkg = cache.get(i).context(format!("Can not get package {}", i))?;
            pkg.mark_delete(is_purge);
        }

        let action = apt_handler(&cache);

        for i in action.autoremove {
            let pkg = cache.get(&i).unwrap();
            pkg.mark_delete(is_purge);
        }

        cache.commit(
            &mut NoProgress::new_box(),
            &mut AptInstallProgress::new_box(),
        )?;

        Ok(())
    }
}

fn apt_install(action: Action, cache: Cache) -> Result<()> {
    let autoremove = action.autoremove;

    for i in autoremove {
        let pkg = cache.get(&i).unwrap();
        // TODO: set purge configuration to user config file
        pkg.mark_delete(true);
    }

    apt_lock()?;

    cache.get_archives(&mut NoProgress::new_box())?;
    apt_unlock_inner();

    if let Err(e) = cache.do_install(&mut AptInstallProgress::new_box()) {
        warn!("{}, retrying ...", e);
        let cache = new_cache!()?;
        cache.resolve(true)?;
        cache.commit(
            &mut NoProgress::new_box(),
            &mut AptInstallProgress::new_box(),
        )?;
    }

    apt_unlock();

    Ok(())
}

struct Action {
    update: Vec<String>,
    install: Vec<String>,
    del: Vec<String>,
    reinstall: Vec<String>,
    downgrade: Vec<String>,
    autoremove: Vec<String>,
}

impl Action {
    fn new(
        update: Vec<String>,
        install: Vec<String>,
        del: Vec<String>,
        reinstall: Vec<String>,
        downgrade: Vec<String>,
        autoremove: Vec<String>,
    ) -> Self {
        Self {
            update,
            install,
            del,
            reinstall,
            downgrade,
            autoremove,
        }
    }
}

fn apt_handler(cache: &Cache) -> Action {
    let changes = cache.get_changes(true).collect::<Vec<_>>();

    let mut update: Vec<String> = vec![];
    let mut install: Vec<String> = vec![];
    let mut del: Vec<String> = vec![];
    let mut reinstall: Vec<String> = vec![];
    let mut downgrade: Vec<String> = vec![];
    let mut autoremove: Vec<String> = vec![];

    for pkg in changes {
        if pkg.marked_install() {
            install.push(pkg.name().to_string());
            // If the package is marked install then it will also
            // show up as marked upgrade, downgrade etc.
            // Check this first and continue.
            continue;
        }
        if pkg.marked_upgrade() {
            update.push(pkg.name().to_string());
        }
        if pkg.marked_delete() {
            del.push(pkg.name().to_string());
        }
        if pkg.marked_reinstall() {
            reinstall.push(pkg.name().to_string());
        }
        if pkg.marked_downgrade() {
            downgrade.push(pkg.name().to_string());
        }
        if pkg.is_auto_removable() {
            autoremove.push(pkg.name().to_string());
        }
    }

    Action::new(update, install, del, reinstall, downgrade, autoremove)
}
