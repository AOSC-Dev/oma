use std::io::Write;

use rust_apt::{
    config::Config,
    raw::progress::{AcquireProgress, InstallProgress},
    util::{get_apt_progress_string, terminal_height, terminal_width, time_str, unit_str, NumSys},
};

use crate::{debug, warn};

// TODO: Make better structs for pkgAcquire items, workers, owners.
/// AptAcquireProgress is the default struct for the update method on the cache.
///
/// This struct mimics the output of `apt update`.
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
        _workers: Vec<rust_apt::raw::progress::Worker>,
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
            warn!("Download is not done, running apt download ...");
            println!(
                "Fetched {} in {} ({}/s)",
                unit_str(fetched_bytes, NumSys::Decimal),
                time_str(elapsed_time),
                unit_str(current_cps, NumSys::Decimal)
            );
        }
    }
}

pub struct YesInstallProgress {
    config: Config,
}

impl YesInstallProgress {
    #[allow(dead_code)]
    pub fn new(force_yes: bool) -> Self {
        let config = Config::new_clear();
        config.set("APT::Get::Assume-Yes", "true");
        config.set("Dpkg::Options::", "--force-confnew");

        if force_yes {
            warn!("Now you are using FORCE automatic mode, if this is not your intention, press Ctrl + C to stop the operation!!!!!");
            config.set("APT::Get::force-yes", "true");
        }

        Self { config }
    }

    /// Return the AptInstallProgress in a box
    /// To easily pass through to do_install
    pub fn new_box(force_yes: bool) -> Box<dyn InstallProgress> {
        Box::new(Self::new(force_yes))
    }
}

impl Default for YesInstallProgress {
    fn default() -> Self {
        Self::new(false)
    }
}

impl InstallProgress for YesInstallProgress {
    fn status_changed(
        &mut self,
        _pkgname: String,
        steps_done: u64,
        total_steps: u64,
        _action: String,
    ) {
        // Get the terminal's width and height.
        let term_height = terminal_height();
        let term_width = terminal_width();

        // Save the current cursor position.
        print!("\x1b7");

        // Go to the progress reporting line.
        print!("\x1b[{term_height};0f");
        std::io::stdout().flush().unwrap();

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

        print!("{bg_color}{fg_color}Progress: [{percent_str}%]{BG_COLOR_RESET}{FG_COLOR_RESET} ");

        // The length of "Progress: [100%] ".
        const PROGRESS_STR_LEN: usize = 17;

        // Print the progress bar.
        // We should safely be able to convert the `usize`.try_into() into the `u32`
        // needed by `get_apt_progress_string`, as usize ints only take up 8 bytes on a
        // 64-bit processor.
        print!(
            "{}",
            get_apt_progress_string(percent, (term_width - PROGRESS_STR_LEN).try_into().unwrap())
        );
        std::io::stdout().flush().unwrap();

        // If this is the last change, remove the progress reporting bar.
        // if steps_done == total_steps {
        // print!("{}", " ".repeat(term_width));
        // print!("\x1b[0;{}r", term_height);
        // }
        // Finally, go back to the previous cursor position.
        print!("\x1b8");
        std::io::stdout().flush().unwrap();
    }

    // TODO: Need to figure out when to use this.
    fn error(&mut self, _pkgname: String, _steps_done: u64, _total_steps: u64, _error: String) {}
}
