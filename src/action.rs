use std::path::Path;

use anyhow::{bail, Result, Context};
use apt_sources_lists::SourceEntry;
use debcontrol::Paragraph;
use indexmap::IndexMap;
use log::warn;
use reqwest::blocking::Client;
use rust_apt::{
    cache::{Cache, Upgrade},
    new_cache,
    util::{apt_unlock, apt_unlock_inner}, raw::progress::AptInstallProgress,
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

        let mut list = action.update;
        list.extend(action.install);

        let db_for_updates = newest_package_list(&self.db)?;
        packages_download(&list, &db_for_updates, &self.sources, &self.client)?;

        cache.resolve(true)?;

        // 没有办法区分 apt 的下载和安装，所以只能先确保其包已经全部下载完成
        if let Err(e) = cache.commit(
            &mut NoProgress::new_box(),
            &mut AptInstallProgress::new_box(),
        ) {
            warn!("{}, retrying ...", e);
            let cache = new_cache!()?;
            cache.resolve(true)?;
            cache.commit(
                &mut NoProgress::new_box(),
                &mut AptInstallProgress::new_box(),
            )?;
        }

        Ok(())
    }

    pub fn install(&self, list: &[String]) -> Result<()> {
        update_db(&self.sources, &self.client)?;

        let cache = new_cache!()?;

        for i in list {
            let pkg = cache.get(i).take().context(format!("Can not get package: {}", i))?;
            if i.contains("=") {
                // Support apt install fish=3.6.0

                let mut split_arg = i.split("=");
                let name = split_arg.nth(0).unwrap();
                let version = split_arg.nth(1).unwrap();

                if PkgVersion::try_from(version).is_err() {
                    bail!("invalid version: {}", version);
                }

                let version = pkg.get_version(version).context(format!("Can not get package {} version: {}", name, version))?;
                let pkg = version.parent();
                pkg.mark_install(true, true);
            } else if i.contains("/") {
                // TODO: Support apt install fish/stable

                let mut split_arg = i.split("/");
                let name = split_arg.nth(0).unwrap();
                let branch = split_arg.nth(1).unwrap();

                todo!()
            } else {
                pkg.mark_install(true, true);
            }
        }

        cache.resolve(true)?;
        packages_download(&list, &self.db, &self.sources, &self.client)?;

        cache.commit(
            &mut NoProgress::new_box(),
            &mut AptInstallProgress::new_box(),
        )?;

        Ok(())
    }
}

struct Action {
    update: Vec<String>,
    install: Vec<String>,
    del: Vec<String>,
    reinstall: Vec<String>,
    downgrade: Vec<String>,
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
            del,
            reinstall,
            downgrade,
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

    Action::new(update, install, del, reinstall, downgrade)
}
