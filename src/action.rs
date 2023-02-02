use std::path::Path;

use anyhow::Result;
use apt_sources_lists::SourceEntry;
use debcontrol::Paragraph;
use indexmap::IndexMap;
use reqwest::blocking::Client;
use rust_apt::{cache::Upgrade, new_cache};

use crate::{
    blackbox::{apt_install_calc, dpkg_run, Action, Package},
    update::{
        find_upgrade, get_sources, get_sources_dists_filename, newest_package_list, package_list,
        update_db, APT_LIST_DISTS, packages_download,
    },
};

// pub fn install(packages: &[Package], client: &Client, package_db: ) -> Result<()> {
//     let (sources, _) = update_db(client)?;
//     let package_db = package_list(&sources)?;
//     let apt_blackbox = apt_install_calc(packages)?;
//     packages_download(&apt_blackbox, packages_db, sources, client)?;

//     Ok(())
// }

pub struct AoscptAction {
    sources: Vec<SourceEntry>,
    db: Vec<IndexMap<String, String>>,
    client: Client,
}

impl AoscptAction {
    pub fn new() -> Result<Self> {
        let client = reqwest::blocking::ClientBuilder::new()
            .user_agent("aoscpt")
            .build()
            .unwrap();

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

        update.extend(install);

        let db_for_updates = newest_package_list(&self.db)?;
        packages_download(&update, &db_for_updates, &self.sources, &self.client)?;
        // dpkg_run(&apt_blackbox)?;

        Ok(())
    }

    pub fn install(&self, list: &[&str]) -> Result<()> {
        let user_action_list = list
            .iter()
            .map(|x| Package {
                name: x.to_string(),
                action: Action::Install,
            })
            .collect::<Vec<_>>();

        let apt_blackbox = apt_install_calc(&user_action_list)?;
        // packages_download(&apt_blackbox, &self.db, &self.sources, &self.client)?;
        dpkg_run(&apt_blackbox)?;

        Ok(())
    }
}
