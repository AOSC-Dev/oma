use oma_pm::oma_apt;
use std::path::PathBuf;

#[inline]
pub fn get_lists_dir() -> PathBuf {
    PathBuf::from(oma_apt::raw::config::find_dir(
        "Dir::State::lists".to_string(),
        "lists/".to_string(),
    ))
}

/// Read apt-configured paths for the given sysroot.
///
/// Returns `(lists_dir, dpkg_status_path, apt_cache_path, search_cache_path)`.
/// Requires apt config to already be initialized (done at startup in `main.rs`).


/// oma display normal message
#[macro_export]
macro_rules! msg {
    ($($arg:tt)+) => {
        use oma_console::writer::Writeln as _;
        let s = format!($($arg)+);
        spdlog::debug!("{s}");
        $crate::WRITER.writeln("", &s).ok();
    };
}

/// oma display success message
#[macro_export]
macro_rules! success {
    ($($arg:tt)+) => {
        use oma_console::writer::Writeln as _;
        let s = format!($($arg)+);
        spdlog::debug!("{s}");
        $crate::WRITER.writeln(&oma_console::console::style("SUCCESS").green().bold().to_string(), &s).ok();
    };
}

/// oma display due_to message
#[macro_export]
macro_rules! due_to {
    ($($arg:tt)+) => {
        use oma_console::writer::Writeln as _;
        let s = format!($($arg)+);
        spdlog::debug!("{s}");
        $crate::WRITER.writeln(&oma_console::console::style("DUE TO").yellow().bold().to_string(), &s).ok();
    };
}
