use std::path::PathBuf;

use clap::ColorChoice;
use once_cell::sync::OnceCell;

use crate::{
    GlobalOptions,
    config_file::{BatteryTristate, ConfigFile, GeneralConfig, SearchEngine, TakeWakeLockTristate},
    subcommand::utils::is_terminal,
};

#[derive(Debug)]
pub struct OmaConfig {
    pub dry_run: bool,
    pub debug: bool,
    pub color: ColorChoice,
    pub follow_terminal_color: bool,
    no_progress: bool,
    pub no_check_dbus: bool,
    pub check_battery: BatteryTristate,
    pub take_wake_lock: TakeWakeLockTristate,
    pub sysroot: PathBuf,
    pub apt_options: Vec<String>,
    pub no_bell: bool,
    pub download_threads: usize,
    #[cfg(feature = "aosc")]
    pub no_refresh_topics: bool,
    pub protect_essentials: bool,
    pub search_contents_println: bool,
    pub search_engine: SearchEngine,
    no_progress_oncecell: OnceCell<bool>,
    pub save_log_count: usize,
}

impl Default for OmaConfig {
    fn default() -> Self {
        Self {
            dry_run: false,
            debug: false,
            color: ColorChoice::Auto,
            follow_terminal_color: false,
            no_progress: false,
            no_check_dbus: false,
            check_battery: BatteryTristate::Ask,
            take_wake_lock: TakeWakeLockTristate::Yes,
            sysroot: PathBuf::from("/"),
            apt_options: Vec::new(),
            no_bell: false,
            download_threads: 4,
            #[cfg(feature = "aosc")]
            no_refresh_topics: false,
            protect_essentials: true,
            search_contents_println: false,
            search_engine: if cfg!(feature = "aosc") {
                SearchEngine::Indicium
            } else {
                SearchEngine::StrSim
            },
            no_progress_oncecell: OnceCell::new(),
            save_log_count: 10,
        }
    }
}

impl OmaConfig {
    pub fn from_config_file(config: ConfigFile) -> Self {
        let mut oma_config = Self::default();

        let ConfigFile { general, network } = config;

        if let Some(general) = general {
            let GeneralConfig {
                no_check_dbus,
                no_refresh_topics,
                follow_terminal_color,
                protect_essentials,
                search_contents_println,
                search_engine,
                ..
            } = general;

            oma_config.no_check_dbus = no_check_dbus;

            #[cfg(feature = "aosc")]
            {
                oma_config.no_refresh_topics = no_refresh_topics;
            }

            oma_config.follow_terminal_color = follow_terminal_color;
            oma_config.protect_essentials = protect_essentials;
            oma_config.search_contents_println = search_contents_println;
            oma_config.search_engine = search_engine;
        }

        if let Some(network) = network {
            oma_config.download_threads = network.network_threads;
        }

        oma_config
    }

    pub fn update_from_cli(&mut self, global_options: &GlobalOptions) {
        let GlobalOptions {
            dry_run,
            debug,
            color,
            follow_terminal_color,
            no_progress,
            no_check_dbus,
            no_check_battery,
            no_take_wake_lock,
            sysroot,
            apt_options,
            no_bell,
            download_threads,
            ..
        } = global_options;

        self.dry_run = *dry_run;
        self.debug = *debug;
        self.color = *color;
        self.sysroot = sysroot.clone();
        self.apt_options = apt_options.clone();
        self.no_bell = *no_bell;
        self.follow_terminal_color = *follow_terminal_color;
        self.no_check_dbus = *no_check_dbus;

        if let Some(download_threads) = download_threads {
            self.download_threads = *download_threads;
        }

        self.no_progress = *no_progress;
        self.check_battery = if *no_check_battery {
            BatteryTristate::Ignore
        } else {
            BatteryTristate::Ask
        };

        self.take_wake_lock = if *no_take_wake_lock {
            TakeWakeLockTristate::Warn
        } else {
            TakeWakeLockTristate::Yes
        };
    }

    #[inline]
    pub fn no_progress(&self) -> bool {
        *self.no_progress_oncecell.get_or_init(|| {
            self.no_progress
                || !is_terminal()
                || self.debug
                || self.dry_run
                || std::env::var("OMA_LOG").is_ok()
        })
    }
}
