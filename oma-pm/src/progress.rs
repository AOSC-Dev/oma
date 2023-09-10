use std::io::Write;

use crate::apt::AptConfig;
use oma_apt::{
    raw::progress::{AcquireProgress, InstallProgress},
    util::{get_apt_progress_string, terminal_height, terminal_width, time_str, unit_str, NumSys},
};
use oma_console::{debug, is_terminal};

#[derive(Default, Debug)]
pub struct NoProgress {
    _lastline: usize,
    _pulse_interval: usize,
    _disable: bool,
}

impl NoProgress {
    /// Returns a new default progress instance.
    pub fn new() -> Self {
        Self::default()
    }

    /// Return the AptAcquireProgress in a box
    /// To easily pass through for progress
    pub fn new_box() -> Box<dyn AcquireProgress> {
        Box::new(Self::new())
    }
}

/// Do not output anything apt AcquireProgress
impl AcquireProgress for NoProgress {
    fn pulse_interval(&self) -> usize {
        0
    }

    fn hit(&mut self, _id: u32, description: String) {
        debug!("{}", description);
    }

    fn fetch(&mut self, _id: u32, description: String, _file_size: u64) {
        debug!("{}", description);
    }

    fn fail(&mut self, _id: u32, description: String, _status: u32, _error_text: String) {
        debug!("{}", description);
    }

    fn pulse(
        &mut self,
        _workers: Vec<oma_apt::raw::progress::Worker>,
        _percent: f32,
        _total_bytes: u64,
        _current_bytes: u64,
        _current_cps: u64,
    ) {
    }

    fn done(&mut self) {}

    fn start(&mut self) {}

    fn stop(
        &mut self,
        fetched_bytes: u64,
        elapsed_time: u64,
        current_cps: u64,
        _pending_errors: bool,
    ) {
        if fetched_bytes != 0 {
            debug!(
                "Fetched {} in {} ({}/s)",
                unit_str(fetched_bytes, NumSys::Decimal),
                time_str(elapsed_time),
                unit_str(current_cps, NumSys::Decimal)
            );
        }
    }
}

pub struct OmaAptInstallProgress {
    config: AptConfig,
    no_progress: bool,
}

impl OmaAptInstallProgress {
    #[allow(dead_code)]
    pub fn new(
        config: AptConfig,
        yes: bool,
        force_yes: bool,
        dpkg_force_confnew: bool,
        dpkg_force_all: bool,
        no_progress: bool,
    ) -> Self {
        if yes {
            oma_apt::raw::config::raw::config_set(
                "APT::Get::Assume-Yes".to_owned(),
                "true".to_owned(),
            );
            debug!("APT::Get::Assume-Yes is set to true");
        }

        if dpkg_force_confnew {
            let opts = config.get("Dpkg::Options::");
            if let Some(opts) = opts {
                config.set("Dpkg::Options::", &format!("{opts} --force-confnew"))
            } else {
                config.set("Dpkg::Options::", "--force-confnew")
            }

            debug!("Dpkg::Options:: is set to --force-confnew");
        } else if yes {
            let opts = config.get("Dpkg::Options::");
            if let Some(opts) = opts {
                config.set("Dpkg::Options::", &format!("{opts} --force-confold"))
            } else {
                config.set("Dpkg::Options::", "--force-confold")
            }

            debug!("Dpkg::Options:: is set to --force-confold");
        }

        if force_yes {
            // warn!("{}", fl!("force-auto-mode"));
            config.set("APT::Get::force-yes", "true");
            debug!("APT::Get::force-Yes is set to true");
        }

        if dpkg_force_all {
            // warn!("{}", fl!("dpkg-force-all-mode"));
            let opts = config.get("Dpkg::Options::");
            if let Some(opts) = opts {
                config.set("Dpkg::Options::", &format!("{opts} --force-all"))
            } else {
                config.set("Dpkg::Options::", "--force-all")
            }
            debug!("Dpkg::Options:: is set to --force-all");
        }

        if !is_terminal() || no_progress {
            std::env::set_var("DEBIAN_FRONTEND", "noninteractive");
            config.set("Dpkg::Use-Pty", "false");
        }

        Self {
            config,
            no_progress,
        }
    }

    /// Return the AptInstallProgress in a box
    /// To easily pass through to do_install
    pub fn new_box(
        config: AptConfig,
        yes: bool,
        force_yes: bool,
        dpkg_force_confnew: bool,
        dpkg_force_all: bool,
        no_progress: bool,
    ) -> Box<dyn InstallProgress> {
        Box::new(Self::new(
            config,
            yes,
            force_yes,
            dpkg_force_confnew,
            dpkg_force_all,
            no_progress,
        ))
    }
}

impl InstallProgress for OmaAptInstallProgress {
    fn status_changed(
        &mut self,
        _pkgname: String,
        steps_done: u64,
        total_steps: u64,
        _action: String,
    ) {
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
