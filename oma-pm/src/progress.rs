use crate::dbus::change_status;
use oma_apt::{progress::DynInstallProgress, raw::config as apt_config};
use oma_utils::zbus;
use once_cell::sync::OnceCell;
use tokio::runtime::{Handle, Runtime};
use zbus::Connection;

pub use oma_apt::util::{get_apt_progress_string, terminal_height, terminal_width};

pub(crate) struct InstallProgressArgs {
    pub async_handler: OnceCell<Handle>,
    pub async_runtime: OnceCell<Runtime>,
    pub connection: OnceCell<Connection>,
}

pub(crate) struct OmaAptInstallProgress {
    rt: (OnceCell<Runtime>, OnceCell<Handle>),
    connection: OnceCell<Connection>,
    pm: Box<dyn InstallProgressManager>,
}

pub trait InstallProgressManager {
    fn status_change(&self, pkgname: &str, steps_done: u64, total_steps: u64);
    fn no_interactive(&self) -> bool;
    fn use_pty(&self) -> bool;
}

impl OmaAptInstallProgress {
    pub fn new(args: InstallProgressArgs, pm: Box<dyn InstallProgressManager>) -> Self {
        let InstallProgressArgs {
            async_handler,
            async_runtime,
            connection,
        } = args;

        if pm.no_interactive() {
            unsafe { std::env::set_var("DEBIAN_FRONTEND", "noninteractive") };
        }

        if !pm.use_pty() {
            apt_config::set("Dpkg::Use-Pty".to_string(), "false".to_string());
        }

        Self {
            rt: (async_runtime, async_handler),
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

        self.pm.status_change(&pkgname, steps_done, total_steps);

        if let Some(tokio) = self.rt.1.get()
            && let Some(conn) = conn.get()
        {
            tokio.block_on(async move {
                change_status(conn, &format!("i {pkgname}")).await.ok();
            });
        }
    }

    fn error(&mut self, _pkgname: String, _steps_done: u64, _total_steps: u64, _error: String) {}
}
