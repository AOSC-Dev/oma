use std::{borrow::Cow, path::PathBuf};

use clap::ColorChoice;
use oma_pm::apt::AptConfig;
use oma_utils::is_termux;
use once_cell::sync::OnceCell;
use reqwest::Client;
use spdlog::debug;

use crate::{
    GlobalOptions,
    args::{OhManagerAilurus, SubCmd},
    config_file::{
        BatteryTristate, ConfigFile, GeneralConfig, NetworkConfig, SearchEngine,
        TakeWakeLockTristate,
    },
    subcommand::utils::is_terminal,
};

#[derive(Debug)]
pub struct OmaConfig {
    pub dry_run: bool,
    pub debug: bool,
    pub color: ColorChoice,
    pub follow_terminal_color: bool,
    cli_no_progress: bool,
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
    no_progress: OnceCell<bool>,
    pub save_log_count: usize,
    pub user_agent: Cow<'static, str>,
    pub yn_mode: bool,
    subcmd: Option<SubCmd>,
    http_client: OnceCell<Client>,
    #[cfg(feature = "aosc")]
    http_client_blocking: OnceCell<reqwest::blocking::Client>,
    rustls_crypto_provider: OnceCell<()>,
}

impl Default for OmaConfig {
    fn default() -> Self {
        Self {
            dry_run: false,
            debug: false,
            color: ColorChoice::Auto,
            follow_terminal_color: GeneralConfig::default_follow_terminal_color(),
            cli_no_progress: false,
            no_check_dbus: GeneralConfig::default_no_check_dbus(),
            check_battery: GeneralConfig::default_check_battery(),
            take_wake_lock: GeneralConfig::default_take_wake_lock(),
            sysroot: PathBuf::from("/"),
            apt_options: Vec::new(),
            no_bell: !GeneralConfig::default_bell(),
            download_threads: NetworkConfig::default_network_thread(),
            #[cfg(feature = "aosc")]
            no_refresh_topics: GeneralConfig::default_no_refresh_topics(),
            protect_essentials: GeneralConfig::default_protect_essentials(),
            search_contents_println: GeneralConfig::default_search_contents_println(),
            search_engine: GeneralConfig::default_search_engine(),
            no_progress: OnceCell::new(),
            save_log_count: GeneralConfig::default_save_log_count(),
            user_agent: NetworkConfig::default_user_agent(),
            yn_mode: GeneralConfig::default_yn_mode(),
            subcmd: None,
            http_client: OnceCell::new(),
            rustls_crypto_provider: OnceCell::new(),
            #[cfg(feature = "aosc")]
            http_client_blocking: OnceCell::new(),
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
                #[cfg(feature = "aosc")]
                no_refresh_topics,
                follow_terminal_color,
                protect_essentials,
                search_contents_println,
                search_engine,
                yn_mode,
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
            oma_config.yn_mode = yn_mode;
        }

        if let Some(network) = network {
            oma_config.download_threads = network.network_threads;
            oma_config.user_agent = network.user_agent;
        }

        oma_config
    }

    pub fn update_from_cli(&mut self, oma: OhManagerAilurus) {
        let OhManagerAilurus { global, subcmd } = oma;

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
            user_agent,
            ..
        } = global;

        self.dry_run |= dry_run;
        self.debug |= debug;
        self.color = color;
        self.sysroot = sysroot;
        self.apt_options = apt_options;

        self.no_bell |= no_bell;
        self.follow_terminal_color |= follow_terminal_color;
        self.no_check_dbus |= no_check_dbus;

        if let Some(download_threads) = download_threads {
            self.download_threads = download_threads;
        }

        self.cli_no_progress = no_progress;

        if no_check_battery {
            self.check_battery = BatteryTristate::Ignore
        }

        if no_take_wake_lock {
            self.take_wake_lock = TakeWakeLockTristate::Warn
        }

        if let Some(user_agent) = user_agent {
            self.user_agent = user_agent.into();
        }

        self.subcmd = subcmd;
    }

    #[inline]
    pub fn no_progress(&self) -> bool {
        *self.no_progress.get_or_init(|| {
            self.cli_no_progress
                || !is_terminal()
                || self.debug
                || self.dry_run
                || std::env::var("OMA_LOG").is_ok()
                || self.color == ColorChoice::Never
        })
    }

    #[inline]
    pub fn take_subcmd(&mut self) -> Option<SubCmd> {
        self.subcmd.take()
    }

    pub fn init_apt_config(&self) {
        let apt_config = AptConfig::new();

        if !is_termux() {
            apt_config.set("Dir", &self.sysroot.to_string_lossy());
        }

        for kv in &self.apt_options {
            let (k, v) = kv.split_once('=').unwrap_or((kv.as_str(), ""));
            debug!("Set apt option: {k}={v}");
            apt_config.set(k, v);
        }
    }

    fn init_tls_config(&self) {
        self.rustls_crypto_provider.get_or_init(|| {
            #[cfg(feature = "rustls")]
            rustls::crypto::ring::default_provider()
                .install_default()
                .expect("Failed to install rustls crypto provider");
        });
    }

    pub fn http_client(&self) -> Result<&Client, reqwest::Error> {
        self.http_client.get_or_try_init(|| {
            self.init_tls_config();
            Client::builder()
                .user_agent(self.user_agent.as_ref())
                .build()
        })
    }

    #[cfg(feature = "aosc")]
    pub fn http_client_blocking(&self) -> Result<&reqwest::blocking::Client, reqwest::Error> {
        self.http_client_blocking.get_or_try_init(|| {
            self.init_tls_config();
            reqwest::blocking::Client::builder()
                .user_agent(self.user_agent.as_ref())
                .build()
        })
    }
}
