use crate::fl;
use serde::{Deserialize, Serialize};

use spdlog::{error, warn};

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
    #[serde(default = "GeneralConfig::default_check_battery")]
    pub check_battery: BatteryTristate,
    #[serde(default = "GeneralConfig::default_take_wake_lock")]
    pub take_wake_lock: TakeWakeLockTristate,
    #[serde(default = "GeneralConfig::default_no_refresh_topics")]
    pub no_refresh_topics: bool,
    #[serde(default = "GeneralConfig::default_follow_terminal_color")]
    pub follow_terminal_color: bool,
    #[serde(default = "GeneralConfig::default_search_contents_println")]
    pub search_contents_println: bool,
    #[serde(default = "GeneralConfig::default_bell")]
    pub bell: bool,
    #[serde(default = "GeneralConfig::default_search_engine")]
    pub search_engine: SearchEngine,
    #[serde(default = "GeneralConfig::default_save_log_count")]
    pub save_log_count: usize,
}

#[derive(Deserialize, Serialize, Debug, Clone, Copy)]
#[serde(rename_all = "lowercase")]
pub enum BatteryTristate {
    Ask,
    Warn,
    Ignore,
}

#[derive(Deserialize, Serialize, Debug, Clone, Copy)]
#[serde(rename_all = "lowercase")]
pub enum SearchEngine {
    Indicium,
    StrSim,
    Text,
}

#[derive(Deserialize, Serialize, Debug, Clone, Copy)]
#[serde(rename_all = "lowercase")]
pub enum TakeWakeLockTristate {
    Yes,
    Warn,
    Ignore,
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

    pub const fn default_bell() -> bool {
        true
    }

    pub const fn default_check_battery() -> BatteryTristate {
        BatteryTristate::Ask
    }

    pub const fn default_take_wake_lock() -> TakeWakeLockTristate {
        TakeWakeLockTristate::Yes
    }

    pub const fn default_search_engine() -> SearchEngine {
        if cfg!(feature = "aosc") {
            SearchEngine::Indicium
        } else {
            SearchEngine::StrSim
        }
    }

    pub const fn default_save_log_count() -> usize {
        10
    }
}

impl Config {
    pub fn read() -> Self {
        let Ok(s) = std::fs::read_to_string("/etc/oma.toml") else {
            warn!("{}", fl!("config-invalid"));
            return toml::from_str::<Self>(DEFAULT_CONFIG).unwrap();
        };

        toml::from_str(&s).unwrap_or_else(|e| {
            error!("Failed to read config: {e}");
            warn!("{}", fl!("config-invalid"));
            toml::from_str(DEFAULT_CONFIG).unwrap()
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

    pub fn check_battery(&self) -> BatteryTristate {
        self.general
            .as_ref()
            .map(|x| x.check_battery)
            .unwrap_or_else(GeneralConfig::default_check_battery)
    }

    pub fn take_wake_lock(&self) -> TakeWakeLockTristate {
        self.general
            .as_ref()
            .map(|x| x.take_wake_lock)
            .unwrap_or_else(GeneralConfig::default_take_wake_lock)
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

    pub fn search_engine(&self) -> SearchEngine {
        self.general
            .as_ref()
            .map(|x| x.search_engine)
            .unwrap_or_else(GeneralConfig::default_search_engine)
    }

    pub fn bell(&self) -> bool {
        self.general
            .as_ref()
            .map(|x| x.bell)
            .unwrap_or_else(GeneralConfig::default_bell)
    }

    #[allow(dead_code)]
    pub fn save_log_count(&self) -> usize {
        self.general
            .as_ref()
            .map(|x| x.save_log_count)
            .unwrap_or_else(GeneralConfig::default_save_log_count)
    }
}
