use std::{io::Write, sync::atomic::Ordering};

use apt_auth_config::AuthConfig;
use dashmap::DashMap;
use indicatif::{MultiProgress, ProgressBar};
use oma_apt::util::{get_apt_progress_string, terminal_height, terminal_width};
use oma_console::{
    pb::{global_progress_bar_style, progress_bar_style, spinner_style},
    writer::Writer,
};
use oma_fetch::{reqwest::ClientBuilder, DownloadProgressControl};
use oma_pm::{
    apt::{AptConfig, CommitDownloadConfig, OmaApt, OmaAptArgs, OmaAptError, SummarySort},
    matches::PackagesMatcher,
    progress::InstallProgressManager,
};
use oma_utils::dpkg::dpkg_arch;

struct MyInstallProgressManager;

impl InstallProgressManager for MyInstallProgressManager {
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

    fn no_interactive(&self) -> bool {
        false
    }

    fn use_pty(&self) -> bool {
        true
    }
}

fn main() -> Result<(), OmaAptError> {
    let oma_apt_args = OmaAptArgs::builder().yes(true).build();
    let mut apt = OmaApt::new(vec![], oma_apt_args, false, AptConfig::new())?;
    let arch = dpkg_arch("/").unwrap();

    let matcher = PackagesMatcher::builder()
        .cache(&apt.cache)
        .filter_candidate(true)
        .filter_downloadable_candidate(false)
        .select_dbg(false)
        .native_arch(&arch)
        .build();

    let pkgs = matcher.match_pkgs(["fish"])?;

    apt.install(&pkgs.0, false)?;

    let client = ClientBuilder::new().user_agent("oma").build().unwrap();

    apt.resolve(false, false)?;

    let op = apt.summary(SummarySort::Operation, |_| false, |_| false)?;

    let pm = MyProgressManager::default();

    apt.commit(
        &client,
        CommitDownloadConfig {
            network_thread: None,
            auth: &AuthConfig::system("/").unwrap(),
        },
        &pm,
        Box::new(MyInstallProgressManager),
        op,
    )?;

    Ok(())
}

struct MyProgressManager {
    mb: MultiProgress,
    pb_map: DashMap<usize, ProgressBar>,
}

impl Default for MyProgressManager {
    fn default() -> Self {
        Self {
            mb: MultiProgress::new(),
            pb_map: DashMap::new(),
        }
    }
}

impl DownloadProgressControl for MyProgressManager {
    fn checksum_mismatch_retry(&self, _index: usize, filename: &str, times: usize) {
        self.mb
            .println(format!(
                "{filename} checksum failed, retrying {times} times"
            ))
            .unwrap();
    }

    fn global_progress_set(&self, num: &std::sync::atomic::AtomicU64) {
        if let Some(pb) = self.pb_map.get(&0) {
            pb.set_position(num.load(Ordering::SeqCst));
        }
    }

    fn progress_done(&self, index: usize) {
        if let Some(pb) = self.pb_map.get(&(index + 1)) {
            pb.finish_and_clear();
        }
    }

    fn new_progress_spinner(&self, index: usize, msg: &str) {
        let (sty, inv) = spinner_style();
        let pb = self
            .mb
            .insert(index + 1, ProgressBar::new_spinner().with_style(sty));
        pb.set_message(msg.to_string());
        pb.enable_steady_tick(inv);
        self.pb_map.insert(index + 1, pb);
    }

    fn new_progress_bar(&self, index: usize, msg: &str, size: u64) {
        let writer = Writer::default();
        let sty = progress_bar_style(&writer);
        let pb = self
            .mb
            .insert(index + 1, ProgressBar::new(size).with_style(sty));
        pb.set_message(msg.to_string());
        self.pb_map.insert(index + 1, pb);
    }

    fn progress_inc(&self, index: usize, num: u64) {
        let pb = self.pb_map.get(&(index + 1)).unwrap();
        pb.inc(num);
    }

    fn progress_set(&self, index: usize, num: u64) {
        let pb = self.pb_map.get(&(index + 1)).unwrap();
        pb.set_position(num);
    }

    fn failed_to_get_source_next_url(&self, _index: usize, err: &str) {
        self.mb.println(format!("Error: {err}")).unwrap();
    }

    fn download_done(&self, _index: usize, _msg: &str) {
        return;
    }

    fn all_done(&self) {
        return;
    }

    fn new_global_progress_bar(&self, total_size: u64) {
        let writer = Writer::default();
        let sty = global_progress_bar_style(&writer);
        let pb = self
            .mb
            .insert(0, ProgressBar::new(total_size).with_style(sty));
        self.pb_map.insert(0, pb);
    }
}
