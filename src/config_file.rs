use crate::fl;
use serde::Deserialize;
use spdlog::{error, warn};

#[derive(Debug, Deserialize)]
pub struct ConfigFile {
    pub general: Option<GeneralConfig>,
    pub network: Option<NetworkConfig>,
}

impl Default for ConfigFile {
    fn default() -> Self {
        ConfigFile {
            general: Some(GeneralConfig {
                protect_essentials: GeneralConfig::default_protect_essentials(),
                no_check_dbus: GeneralConfig::default_no_check_dbus(),
                check_battery: GeneralConfig::default_check_battery(),
                take_wake_lock: GeneralConfig::default_take_wake_lock(),
                no_refresh_topics: GeneralConfig::default_no_refresh_topics(),
                follow_terminal_color: GeneralConfig::default_follow_terminal_color(),
                search_contents_println: GeneralConfig::default_search_contents_println(),
                bell: GeneralConfig::default_bell(),
                search_engine: GeneralConfig::default_search_engine(),
                save_log_count: GeneralConfig::default_save_log_count(),
            }),
            network: Some(NetworkConfig {
                network_threads: NetworkConfig::default_network_thread(),
            }),
        }
    }
}

#[allow(dead_code)]
#[derive(Debug, Deserialize)]
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

#[derive(Deserialize, Debug, Clone, Copy, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum BatteryTristate {
    Ask,
    Warn,
    Ignore,
}

#[derive(Deserialize, Debug, Clone, Copy)]
#[serde(rename_all = "lowercase")]
pub enum SearchEngine {
    Indicium,
    StrSim,
    Text,
}

#[derive(Deserialize, Debug, Clone, Copy, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum TakeWakeLockTristate {
    Yes,
    Warn,
    Ignore,
}

#[derive(Debug, Deserialize)]
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

impl ConfigFile {
    pub fn read() -> Self {
        let Ok(s) = std::fs::read_to_string("/etc/oma.toml") else {
            warn!("{}", fl!("config-invalid"));
            return Self::default();
        };

        toml::from_str(&s).unwrap_or_else(|e| {
            error!("Failed to read config: {e}");
            warn!("{}", fl!("config-invalid"));
            Self::default()
        })
    }
}
