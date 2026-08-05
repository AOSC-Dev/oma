use std::{
    io::Write,
    sync::{
        LazyLock,
        atomic::{AtomicU32, Ordering},
    },
};

use oma_pm::{
    oma_apt::raw::config as apt_config,
    progress::{InstallProgressManager, get_apt_progress_string, terminal_height, terminal_width},
};

use crate::subcommand::utils::is_terminal;

pub struct OmaInstallProgressManager {
    yes: bool,
    bg_color: String,
    fg_color: String,
}

impl OmaInstallProgressManager {
    // The length of "Progress: [100%] ".
    const PROGRESS_STR_LEN: usize = 17;
    const BG_COLOR_RESET: &str = "\x1b[49m";
    const FG_COLOR_RESET: &str = "\x1b[39m";
    pub fn new(yes: bool) -> Self {
        Self {
            yes,
            bg_color: apt_config::find(
                "Dpkg::Progress-Fancy::Progress-fg".to_string(),
                "\x1b[42m".to_string(),
            ),
            fg_color: apt_config::find(
                "Dpkg::Progress-Fancy::Progress-bg".to_string(),
                "\x1b[30m".to_string(),
            ),
        }
    }
}

impl InstallProgressManager for OmaInstallProgressManager {
    fn status_change(&self, _pkgname: &str, steps_done: u64, total_steps: u64) {
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
        let percent_1 = steps_done as f32 / total_steps as f32;
        let percent_100 = (percent_1 * 100.0).round();
        let percent_for_dpkg = 50.0 + percent_100 * 0.5;

        OSC94.set(percent_for_dpkg);

        let mut percent_str = percent_100.to_string();

        let percent_padding = match percent_str.len() {
            1 => "  ",
            2 => " ",
            3 => "",
            _ => unreachable!(),
        };

        percent_str = percent_padding.to_owned() + &percent_str;

        eprint!(
            "{}{}Progress: [{percent_str}%]{}{} ",
            self.bg_color,
            self.fg_color,
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
                percent_1,
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

/// Terminal percentage progress via OSC 94, shared by the download renderer
/// thread, the dpkg progress manager and the exit handler. The only shared
/// state is the last reported integer percent, so a plain atomic is enough:
/// updates are lock-free and redundant writes are throttled away.
pub static OSC94: LazyLock<Osc94> = LazyLock::new(Osc94::default);

/// See [`OSC94`].
#[derive(Default)]
pub struct Osc94 {
    last: AtomicU32,
}

impl Osc94 {
    /// Report `percent` (0-100) as terminal progress. The write is skipped when
    /// the rounded value equals the last one reported.
    pub fn set(&self, percent: f32) {
        let percent = percent.round() as u32;
        if self.last.swap(percent, Ordering::Relaxed) != percent {
            Self::write_set(percent);
        }
    }

    /// Clear the terminal progress. Always writes, regardless of throttling,
    /// so a later [`Self::set`] with a different value reports again.
    pub fn finish(&self) {
        self.last.store(100, Ordering::Relaxed);
        Self::write_remove();
    }

    /// Write the OSC 94 escape sequence that reports percentage progress.
    ///
    /// From https://conemu.github.io/en/AnsiEscapeCodes.html#ConEmu_specific_OSC
    /// `ESC ] 9 ; 4 ; st ; pr ST`:
    /// - st 0: remove progress
    /// - st 1: set progress value to pr (number, 0-100)
    /// - st 2: set error state, pr optional
    /// - st 3: indeterminate state, pr ignored
    /// - st 4: paused state, pr optional
    fn write_set(percent: u32) {
        eprint!("\x1b]9;4;1;{percent}\x1b\\");
    }

    fn write_remove() {
        eprint!("\x1b]9;4;0;0\x1b\\");
    }
}

pub struct NoInstallProgressManager;

impl InstallProgressManager for NoInstallProgressManager {
    fn status_change(&self, _pkgname: &str, _steps_done: u64, _total_steps: u64) {}

    fn no_interactive(&self) -> bool {
        true
    }

    fn use_pty(&self) -> bool {
        false
    }
}
