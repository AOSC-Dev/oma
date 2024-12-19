use crate::{apt::AptConfig, dbus::change_status};
use oma_apt::progress::DynInstallProgress;
use tokio::runtime::Runtime;
use zbus::Connection;

pub use oma_apt::util::{get_apt_progress_string, terminal_height, terminal_width};

#[derive(Default, Debug)]
pub struct NoProgress {
    _lastline: usize,
    _pulse_interval: usize,
    _disable: bool,
}

pub(crate) struct InstallProgressArgs {
    pub config: AptConfig,
    pub tokio: Runtime,
    pub connection: Option<Connection>,
}

pub(crate) struct OmaAptInstallProgress {
    config: AptConfig,
    tokio: Runtime,
    connection: Option<Connection>,
    pm: Box<dyn InstallProgressManager>,
}

pub trait InstallProgressManager {
    fn status_change(&self, pkgname: &str, steps_done: u64, total_steps: u64, config: &AptConfig);
    fn no_interactive(&self) -> bool;
    fn use_pty(&self) -> bool;
}

impl OmaAptInstallProgress {
    pub fn new(args: InstallProgressArgs, pm: Box<dyn InstallProgressManager>) -> Self {
        let InstallProgressArgs {
            config,
            tokio,
            connection,
        } = args;

        if pm.no_interactive() {
            std::env::set_var("DEBIAN_FRONTEND", "noninteractive");
        }

        if !pm.use_pty() {
            config.set("Dpkg::Use-Pty", "false");
        }

        Self {
            config,
            tokio,
            connection,
            pm,
        }
    }
}

impl DynInstallProgress for OmaAptInstallProgress {
    fn status_changed(
        &mut self,
        pkgname: String,
        steps_done: u64,
        total_steps: u64,
        _action: String,
    ) {
        let conn = &self.connection;

        self.pm
            .status_change(&pkgname, steps_done, total_steps, &self.config);

        self.tokio.block_on(async move {
            if let Some(conn) = conn {
                change_status(conn, &format!("i {pkgname}"))
                    .await
                    .ok();
            }
        });
    }

    fn error(&mut self, _pkgname: String, _steps_done: u64, _total_steps: u64, _error: String) {}
}
