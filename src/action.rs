use std::process::Command;

use anyhow::Result;
use apt_sources_lists::SourceEntry;
use eight_deep_parser::{IndexMap, Item};
use reqwest::blocking::Client;

use crate::{
    blackbox::{apt_install_calc, Package, Action, dpkg_run},
    update::{get_sources, newest_package_list, package_list, packages_download, update_db, find_upgrade},
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
    db: Vec<IndexMap<String, Item>>,
    db_for_update: Vec<IndexMap<String, Item>>,
    client: Client,
}

impl AoscptAction {
    pub fn new() -> Result<Self> {
        let client = reqwest::blocking::ClientBuilder::new()
            .user_agent("aoscpt")
            .build()
            .unwrap();

        let sources = get_sources()?;
        let db_files = update_db(&sources, &client)?;
        let db = package_list(db_files)?;
        let db_for_update = newest_package_list(&db)?;

        Ok(Self {
            sources,
            db,
            db_for_update,
            client,
        })
    }

    /// Update mirror database and Get all update, like apt update && apt full-upgrade
    pub fn update(&self) -> Result<()> {
        let u = find_upgrade(&self.db_for_update)?;
        let user_action_list = u
            .iter()
            .map(|x| Package {
                name: x.package.to_string(),
                action: Action::Install,
            })
            .collect::<Vec<_>>();
    
        let apt_blackbox = apt_install_calc(&user_action_list)?;
        packages_download(&apt_blackbox, &self.db_for_update, &self.sources, &self.client)?;
        dpkg_run(&apt_blackbox)?;

        Ok(())
    }

    pub fn install(&self, list: &[&str]) -> Result<()> {
        let user_action_list = list.iter().map(|x| Package {
            name: x.to_string(),
            action: Action::Install,
        }).collect::<Vec<_>>();

        let apt_blackbox = apt_install_calc(&user_action_list)?;
        packages_download(&apt_blackbox, &self.db, &self.sources, &self.client)?;
        dpkg_run(&apt_blackbox)?;

        Ok(())
    }
}

