use std::borrow::Cow;

use crate::fl;
use anyhow::Result;
use serde::{Deserialize, Serialize};
use tracing::warn;

#[cfg(feature = "aosc")]
const DEFAULT_CONFIG: &str = include_str!("../data/config/oma.toml");

#[cfg(not(feature = "aosc"))]
const DEFAULT_CONFIG: &str = include_str!("../data/config/oma-debian.toml");

#[derive(Debug, Deserialize, Serialize)]
pub struct Config {
    pub general: Option<GeneralConfig>,
    pub network: Option<NetworkConfig>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct GeneralConfig {
    #[serde(default = "GeneralConfig::default_protect_essentials")]
    pub protect_essentials: bool,
    #[serde(default = "GeneralConfig::default_no_check_dbus")]
    pub no_check_dbus: bool,
    #[serde(default = "GeneralConfig::default_no_refresh_topics")]
    pub no_refresh_topics: bool,
    #[serde(default = "GeneralConfig::default_follow_terminal_color")]
    pub follow_terminal_color: bool,
    #[serde(default = "GeneralConfig::default_search_contents_println")]
    pub search_contents_println: bool,
    #[serde(default = "GeneralConfig::default_search_engine")]
    pub search_engine: String,
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

    pub const fn default_no_check_dbus() -> bool {
        false
    }

    pub const fn default_no_refresh_topics() -> bool {
        false
    }

    pub const fn default_follow_terminal_color() -> bool {
        false
    }

    pub const fn default_search_contents_println() -> bool {
        false
    }

    pub fn default_search_engine() -> String {
        if cfg!(feature = "aosc") {
            String::from("indicium")
        } else {
            String::from("strsim")
        }
    }
}

impl Config {
    pub fn read() -> Result<Self> {
        let s = std::fs::read_to_string("/etc/oma.toml");

        Ok(match s {
            Ok(s) => toml::from_str(&s)?,
            Err(_) => {
                warn!("{}", fl!("config-invalid"));
                toml::from_str(DEFAULT_CONFIG)?
            }
        })
    }

    pub fn protect_essentials(&self) -> bool {
        self.general
            .as_ref()
            .map(|x| x.protect_essentials)
            .unwrap_or_else(GeneralConfig::default_protect_essentials)
    }

    pub fn network_thread(&self) -> usize {
        self.network
            .as_ref()
            .map(|x| x.network_threads)
            .unwrap_or_else(NetworkConfig::default_network_thread)
    }

    pub fn no_check_dbus(&self) -> bool {
        self.general
            .as_ref()
            .map(|x| x.no_check_dbus)
            .unwrap_or_else(GeneralConfig::default_no_check_dbus)
    }

    #[cfg(feature = "aosc")]
    pub fn no_refresh_topics(&self) -> bool {
        self.general
            .as_ref()
            .map(|x| x.no_refresh_topics)
            .unwrap_or_else(GeneralConfig::default_no_refresh_topics)
    }

    pub fn follow_terminal_color(&self) -> bool {
        self.general
            .as_ref()
            .map(|x| x.follow_terminal_color)
            .unwrap_or_else(GeneralConfig::default_follow_terminal_color)
    }

    pub fn search_contents_println(&self) -> bool {
        self.general
            .as_ref()
            .map(|x| x.search_contents_println)
            .unwrap_or_else(GeneralConfig::default_search_contents_println)
    }

    pub fn search_engine(&self) -> Cow<String> {
        self.general
            .as_ref()
            .map(|x| Cow::Borrowed(&x.search_engine))
            .unwrap_or_else(|| Cow::Owned(GeneralConfig::default_search_engine()))
    }
}
