use std::path::Path;

use anyhow::{bail, Context, Result};
use apt_sources_lists::SourceEntry;
use indexmap::IndexMap;
use log::warn;
use reqwest::Client;
use rust_apt::{
    cache::{Cache, PackageSort, Upgrade},
    new_cache,
    raw::{progress::AptInstallProgress, util::raw::apt_lock_inner},
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
    pub async fn new() -> Result<Self> {
        let client = reqwest::ClientBuilder::new().user_agent("aoscpt").build()?;

        let sources = get_sources()?;

        std::fs::create_dir_all(APT_LIST_DISTS)?;

        let db_paths = get_sources_dists_filename(&sources, &client).await?;

        let db = package_list(
            db_paths
                .iter()
                .map(|x| Path::new(APT_LIST_DISTS).join(x))
                .collect(),
            &sources,
            &client,
        )
        .await?;

        Ok(Self {
            sources,
            db,
            client,
        })
    }

    /// Update mirror database and Get all update, like apt update && apt full-upgrade
    pub async fn update(&self) -> Result<()> {
        update_db(&self.sources, &self.client).await?;

        async fn update_inner(
            sources: &[SourceEntry],
            client: &Client,
            db: &[IndexMap<String, String>],
        ) -> Result<()> {
            let cache = new_cache!()?;
            cache.upgrade(&Upgrade::FullUpgrade)?;

            let action = apt_handler(&cache);

            let mut list = action.update.clone();
            list.extend(action.install);

            autoremove(&cache);

            let db_for_updates = newest_package_list(db)?;
            packages_download(&list, &db_for_updates, sources, client, None).await?;

            cache.resolve(true)?;
            apt_install(cache)?;

            Ok(())
        }

        // Retry 3 times
        let mut count = 0;
        while let Err(e) = update_inner(&self.sources, &self.client, &self.db).await {
            warn!("{e}, retrying ...");
            if count == 3 {
                return Err(e);
            }
            count += 1;
        }

        Ok(())
    }

    pub async fn install(&self, list: &[String]) -> Result<()> {
        update_db(&self.sources, &self.client).await?;

        let mut count = 0;
        while let Err(e) = self.install_inner(list).await {
            warn!("{e}, retrying ...");
            if count == 3 {
                return Err(e);
            }

            count += 1;
        }

        Ok(())
    }

    async fn install_inner(&self, list: &[String]) -> Result<()> {
        let cache = new_cache!()?;
        for i in list {
            let pkg = cache
                .get(i)
                .take()
                .context(format!("Can not get package: {i}"))?;
            if i.contains('=') {
                // Support apt install fish=3.6.0

                let mut split_arg = i.split('=');
                let name = split_arg.next().context(format!("Not Support: {i}"))?;
                let version = split_arg.nth(1).context(format!("Not Support: {i}"))?;

                if PkgVersion::try_from(version).is_err() {
                    bail!("invalid version: {}", version);
                }

                let version = pkg
                    .get_version(version)
                    .context(format!("Can not get package {name} version: {version}"))?;

                let pkg = version.parent();
                pkg.mark_install(true, true);
                pkg.protect();
            } else if i.contains('/') {
                // Support apt install fish/stable
                let mut split_arg = i.split('/');
                let name = split_arg.next().context(format!("Not Support: {i}"))?;
                let branch = split_arg.nth(1).context(format!("Not Support: {i}"))?;

                let mut res = self
                    .db
                    .iter()
                    .filter(|x| x.get("Package") == Some(&name.to_string()))
                    .filter(|x| x["Filename"].split('/').nth(1) == Some(branch))
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
        list.extend(action.update);
        autoremove(&cache);
        cache.resolve(true)?;
        // TODO: limit 参数（限制下载包并发）目前是写死的，以后将允许用户自定义并发数
        packages_download(&list, &self.db, &self.sources, &self.client, None).await?;
        apt_install(cache)?;

        Ok(())
    }

    pub fn remove(&self, list: &[String], is_purge: bool) -> Result<()> {
        let cache = new_cache!()?;

        for i in list {
            let pkg = cache.get(i).context(format!("Can not get package {i}"))?;
            pkg.mark_delete(is_purge);
        }

        autoremove(&cache);

        cache.commit(
            &mut NoProgress::new_box(),
            &mut AptInstallProgress::new_box(),
        )?;

        Ok(())
    }

    pub async fn refresh(&self) -> Result<()> {
        update_db(&self.sources, &self.client).await
    }
}

fn autoremove(cache: &Cache) {
    let sort = PackageSort::default();
    let mut autoremove = vec![];

    for pkg in cache.packages(&sort) {
        if pkg.is_auto_removable() {
            autoremove.push(pkg.name().to_string());
            pkg.mark_delete(true);
        }
    }

    dbg!(autoremove);
}

fn apt_install(cache: Cache) -> Result<()> {
    apt_lock()?;
    cache.get_archives(&mut NoProgress::new_box())?;
    apt_unlock_inner();

    if let Err(e) = cache.do_install(&mut AptInstallProgress::new_box()) {
        apt_lock_inner()?;
        apt_unlock();
        return Err(e.into());
    }

    apt_unlock();

    Ok(())
}

#[derive(Debug)]
struct Action {
    update: Vec<String>,
    install: Vec<String>,
    _del: Vec<String>,
    _reinstall: Vec<String>,
    _downgrade: Vec<String>,
}

impl Action {
    fn new(
        update: Vec<String>,
        install: Vec<String>,
        del: Vec<String>,
        reinstall: Vec<String>,
        downgrade: Vec<String>,
    ) -> Self {
        Self {
            update,
            install,
            _del: del,
            _reinstall: reinstall,
            _downgrade: downgrade,
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
    }

    let action = Action::new(update, install, del, reinstall, downgrade);

    dbg!(&action);

    action
}
