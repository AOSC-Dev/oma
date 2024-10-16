use std::io::Write;

use crate::{apt::AptConfig, dbus::change_status};
use oma_apt::{
    progress::DynInstallProgress,
    util::{get_apt_progress_string, terminal_height, terminal_width},
};
use oma_console::is_terminal;
use tokio::runtime::Runtime;
use tracing::debug;
use zbus::Connection;

#[derive(Default, Debug)]
pub struct NoProgress {
    _lastline: usize,
    _pulse_interval: usize,
    _disable: bool,
}

pub struct InstallProgressArgs {
    pub config: AptConfig,
    // pub yes: bool,
    // pub force_yes: bool,
    // pub dpkg_force_confnew: bool,
    // pub dpkg_force_unsafe_io: bool,
    pub tokio: Runtime,
    pub connection: Option<Connection>,
}

pub struct OmaAptInstallProgress {
    config: AptConfig,
    no_progress: bool,
    tokio: Runtime,
    connection: Option<Connection>,
}

impl OmaAptInstallProgress {
    #[allow(dead_code)]
    pub fn new(args: InstallProgressArgs) -> Self {
        let InstallProgressArgs {
            config,
            tokio,
            connection,
        } = args;

        let no_progress = config.bool("Oma::NoProgress", false);

        debug!("Oma::NoProgress is: {}", no_progress);

        if !is_terminal() || no_progress {
            std::env::set_var("DEBIAN_FRONTEND", "noninteractive");
            config.set("Dpkg::Use-Pty", "false");
        }

        Self {
            config,
            no_progress,
            tokio,
            connection,
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
        self.tokio.block_on(async move {
            if let Some(conn) = conn {
                change_status(conn, &format!("Changing package {pkgname}"))
                    .await
                    .ok();
            }
        });

        if !is_terminal() || self.no_progress {
            return;
        }

        // Get the terminal's width and height.
        let term_height = terminal_height();
        let term_width = terminal_width();

        // Save the current cursor position.
        eprint!("\x1b7");

        // Go to the progress reporting line.
        eprint!("\x1b[{term_height};0f");
        // 这里（和下面的）所返回的错误都是因为无法操作终端导致的，这时程序应该直接崩溃
        // 所以下面都是 unwrap
        std::io::stderr().flush().unwrap();

        // Convert the float to a percentage string.
        let percent = steps_done as f32 / total_steps as f32;
        let mut percent_str = (percent * 100.0).round().to_string();

        let percent_padding = match percent_str.len() {
            1 => "  ",
            2 => " ",
            3 => "",
            _ => unreachable!(),
        };

        percent_str = percent_padding.to_owned() + &percent_str;

        // Get colors for progress reporting.
        // NOTE: The APT implementation confusingly has 'Progress-fg' for 'bg_color',
        // and the same the other way around.
        let bg_color = self
            .config
            .find("Dpkg::Progress-Fancy::Progress-fg", "\x1b[42m");
        let fg_color = self
            .config
            .find("Dpkg::Progress-Fancy::Progress-bg", "\x1b[30m");
        const BG_COLOR_RESET: &str = "\x1b[49m";
        const FG_COLOR_RESET: &str = "\x1b[39m";

        eprint!("{bg_color}{fg_color}Progress: [{percent_str}%]{BG_COLOR_RESET}{FG_COLOR_RESET} ");

        // The length of "Progress: [100%] ".
        const PROGRESS_STR_LEN: usize = 17;

        // Print the progress bar.
        // We should safely be able to convert the `usize`.try_into() into the `u32`
        // needed by `get_apt_progress_string`, as usize ints only take up 8 bytes on a
        // 64-bit processor.
        eprint!(
            "{}",
            get_apt_progress_string(percent, (term_width - PROGRESS_STR_LEN).try_into().unwrap())
        );
        std::io::stderr().flush().unwrap();

        // If this is the last change, remove the progress reporting bar.
        // if steps_done == total_steps {
        // print!("{}", " ".repeat(term_width));
        // print!("\x1b[0;{}r", term_height);
        // }
        // Finally, go back to the previous cursor position.
        eprint!("\x1b8");
        std::io::stderr().flush().unwrap();
    }

    fn error(&mut self, _pkgname: String, _steps_done: u64, _total_steps: u64, _error: String) {}
}
