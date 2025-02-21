use std::io::Write;

use oma_pm::{
    apt::AptConfig,
    progress::{InstallProgressManager, get_apt_progress_string, terminal_height, terminal_width},
};

use crate::subcommand::utils::is_terminal;

pub struct OmaInstallProgressManager {
    yes: bool,
}

impl OmaInstallProgressManager {
    // The length of "Progress: [100%] ".
    const PROGRESS_STR_LEN: usize = 17;
    const BG_COLOR_RESET: &str = "\x1b[49m";
    const FG_COLOR_RESET: &str = "\x1b[39m";
    pub fn new(yes: bool) -> Self {
        Self { yes }
    }
}

impl InstallProgressManager for OmaInstallProgressManager {
    fn status_change(&self, _pkgname: &str, steps_done: u64, total_steps: u64, config: &AptConfig) {
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
        let bg_color = config.find("Dpkg::Progress-Fancy::Progress-fg", "\x1b[42m");
        let fg_color = config.find("Dpkg::Progress-Fancy::Progress-bg", "\x1b[30m");

        eprint!(
            "{bg_color}{fg_color}Progress: [{percent_str}%]{}{} ",
            Self::BG_COLOR_RESET,
            Self::FG_COLOR_RESET
        );

        // Print the progress bar.
        // We should safely be able to convert the `usize`.try_into() into the `u32`
        // needed by `get_apt_progress_string`, as usize ints only take up 8 bytes on a
        // 64-bit processor.
        eprint!(
            "{}",
            get_apt_progress_string(
                percent,
                (term_width - Self::PROGRESS_STR_LEN).try_into().unwrap()
            )
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

    fn no_interactive(&self) -> bool {
        !is_terminal() || self.yes
    }

    fn use_pty(&self) -> bool {
        is_terminal()
    }
}

pub struct NoInstallProgressManager;

impl InstallProgressManager for NoInstallProgressManager {
    fn status_change(
        &self,
        _pkgname: &str,
        _steps_done: u64,
        _total_steps: u64,
        _config: &AptConfig,
    ) {
    }

    fn no_interactive(&self) -> bool {
        true
    }

    fn use_pty(&self) -> bool {
        false
    }
}
