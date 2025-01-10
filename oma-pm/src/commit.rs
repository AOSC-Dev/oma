use std::path::Path;

use apt_auth_config::AuthConfig;
use chrono::Local;
use oma_apt::{
    progress::{AcquireProgress, InstallProgress},
    util::{apt_lock, apt_lock_inner, apt_unlock, apt_unlock_inner},
};
use oma_fetch::{reqwest::Client, Event, Summary};
use oma_pm_operation_type::{InstallEntry, OmaOperation};
use std::io::Write;
use tracing::debug;

use crate::{
    apt::{DownloadConfig, OmaApt, OmaAptError, OmaAptResult},
    dbus::change_status,
    download::download_pkgs,
    progress::{InstallProgressArgs, InstallProgressManager, OmaAptInstallProgress},
};

const TIME_FORMAT: &str = "%H:%M:%S on %Y-%m-%d";

pub struct CommitNetworkConfig<'a> {
    pub network_thread: Option<usize>,
    pub auth_config: &'a AuthConfig,
}

pub struct DoInstall<'a> {
    apt: OmaApt,
    client: &'a Client,
    sysroot: &'a str,
    config: CommitNetworkConfig<'a>,
}

impl<'a> DoInstall<'a> {
    pub fn new(
        apt: OmaApt,
        client: &'a Client,
        sysroot: &'a str,
        config: CommitNetworkConfig<'a>,
    ) -> Result<Self, OmaAptError> {
        Ok(Self {
            apt,
            sysroot,
            client,
            config,
        })
    }

    pub fn commit<F, Fut>(
        self,
        op: &OmaOperation,
        install_progress_manager: Box<dyn InstallProgressManager>,
        callback: F,
    ) -> OmaAptResult<()>
    where
        F: Fn(Event) -> Fut,
        Fut: std::future::Future<Output = ()>,
    {
        let summary = self.download_pkgs(&op.install, callback)?;

        if !summary.failed.is_empty() {
            return Err(OmaAptError::FailedToDownload(summary.failed.len()));
        }

        self.do_install(install_progress_manager, op)?;

        Ok(())
    }

    fn download_pkgs<F, Fut>(
        &self,
        download_pkg_list: &[InstallEntry],
        callback: F,
    ) -> OmaAptResult<Summary>
    where
        F: Fn(Event) -> Fut,
        Fut: std::future::Future<Output = ()>,
    {
        let path = self.apt.get_archive_dir();

        self.apt.tokio.block_on(async {
            if let Some(conn) = &self.apt.conn {
                change_status(conn, "Downloading").await.ok();
            }

            let config = DownloadConfig {
                network_thread: self.config.network_thread,
                download_dir: Some(path),
                auth: self.config.auth_config,
            };

            download_pkgs(self.client, download_pkg_list, config, callback).await
        })
    }

    fn do_install(
        self,
        install_progress_manager: Box<dyn InstallProgressManager>,
        op: &OmaOperation,
    ) -> OmaAptResult<()> {
        apt_lock()?;

        debug!("Try to get apt archives");

        self.apt
            .cache
            .get_archives(&mut AcquireProgress::quiet())
            .inspect_err(|e| {
                debug!("Get exception: {e}. Try to unlock apt lock");
                apt_unlock();
            })?;

        let args = InstallProgressArgs {
            config: self.apt.config,
            tokio: self.apt.tokio,
            connection: self.apt.conn,
        };

        let mut progress =
            InstallProgress::new(OmaAptInstallProgress::new(args, install_progress_manager));

        debug!("Try to unlock apt lock inner");

        apt_unlock_inner();

        debug!("Do install");

        self.apt.cache.do_install(&mut progress).inspect_err(|e| {
            debug!("do_install got except: {e}");
            apt_lock_inner().ok();
            apt_unlock();
        })?;

        debug!("Try to unlock apt lock");

        apt_unlock();

        Self::log(self.sysroot, op)?;

        Ok(())
    }

    fn log(sysroot: &'a str, op: &OmaOperation) -> OmaAptResult<()> {
        let end_time = Local::now().format(TIME_FORMAT).to_string();

        let sysroot = Path::new(sysroot);
        let history = sysroot.join("var/log/oma/history");
        let parent = history
            .parent()
            .ok_or_else(|| OmaAptError::FailedGetParentPath(history.clone()))?;

        std::fs::create_dir_all(parent)
            .map_err(|e| OmaAptError::FailedOperateDirOrFile(parent.display().to_string(), e))?;

        let mut log = std::fs::OpenOptions::new()
            .append(true)
            .create(true)
            .open(&history)
            .map_err(|e| OmaAptError::FailedOperateDirOrFile(history.display().to_string(), e))?;

        let start_time = Local::now();
        writeln!(log, "Start-Date: {start_time}").ok();

        let args = std::env::args().collect::<Vec<_>>().join(" ");

        if !args.is_empty() {
            writeln!(log, "Commandline: {args}").ok();
        }

        if let Some((user, uid)) = std::env::var("SUDO_USER")
            .ok()
            .zip(std::env::var("SUDO_UID").ok())
        {
            writeln!(log, "Requested-By: {user} ({uid})").ok();
        }

        write!(log, "{op}").ok();
        writeln!(log, "End-Date: {end_time}\n").ok();

        Ok(())
    }
}
