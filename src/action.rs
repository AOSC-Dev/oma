use std::path::Path;

use anyhow::Result;
use apt_sources_lists::SourceEntry;
use debcontrol::Paragraph;
use indexmap::IndexMap;
use reqwest::blocking::Client;

use crate::{
    blackbox::{apt_install_calc, dpkg_run, Action, Package},
    update::{
        find_upgrade, get_sources, get_sources_dists_filename, newest_package_list, package_list,
        packages_download, update_db, APT_LIST_DISTS,
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

        let db_for_updates = newest_package_list(&self.db)?;

        let u = find_upgrade(&db_for_updates)?;
        let user_action_list = u
            .iter()
            .map(|x| Package {
                name: x.package.to_string(),
                action: Action::Install,
            })
            .collect::<Vec<_>>();

        let apt_blackbox = apt_install_calc(&user_action_list)?;
        packages_download(&apt_blackbox, &db_for_updates, &self.sources, &self.client)?;
        dpkg_run(&apt_blackbox)?;

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
        packages_download(&apt_blackbox, &self.db, &self.sources, &self.client)?;
        dpkg_run(&apt_blackbox)?;

        Ok(())
    }
}
