use crate::fl;
use anyhow::Result;
use serde::{Deserialize, Serialize};
use tracing::warn;

const DEFAULT_CONFIG: &str = include_str!("../data/config/oma.toml");

#[derive(Debug, Deserialize, Serialize)]
pub struct Config {
    pub general: Option<GeneralConfig>,
    pub network: Option<NetworkConfig>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct GeneralConfig {
    #[serde(default = "GeneralConfig::default_protect_essentials")]
    pub protect_essentials: bool,
    #[serde(default = "GeneralConfig::default_refresh_pure_database")]
    pub refresh_pure_database: bool,
    #[serde(default = "GeneralConfig::default_no_check_dbus")]
    pub no_check_dbus: bool,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct NetworkConfig {
    #[serde(default = "NetworkConfig::default_network_thread")]
    pub network_threads: usize,
}

impl NetworkConfig {
    pub const fn default_network_thread() -> usize {
        4
    }
}

impl GeneralConfig {
    pub const fn default_protect_essentials() -> bool {
        true
    }

    pub const fn default_refresh_pure_database() -> bool {
        false
    }

    pub const fn default_no_check_dbus() -> bool {
        false
    }
}

impl Config {
    pub fn read() -> Result<Self> {
        let s = std::fs::read_to_string("/etc/oma.toml");

        Ok(match s {
            Ok(s) => toml::from_str(&s)?,
            Err(_) => {
                warn!("{}", fl!("config-invaild"));
                toml::from_str(DEFAULT_CONFIG)?
            }
        })
    }

    pub fn network_thread(&self) -> usize {
        self.network
            .as_ref()
            .map(|x| x.network_threads)
            .unwrap_or_else(NetworkConfig::default_network_thread)
    }

    pub fn pure_db(&self) -> bool {
        self.general
            .as_ref()
            .map(|x| x.refresh_pure_database)
            .unwrap_or_else(GeneralConfig::default_refresh_pure_database)
    }

    pub fn check_dbus(&self) -> bool {
        self.general
            .as_ref()
            .map(|x| x.no_check_dbus)
            .unwrap_or_else(GeneralConfig::default_no_check_dbus)
    }
}
