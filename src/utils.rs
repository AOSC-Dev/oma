use oma_pm::oma_apt;
use std::path::PathBuf;

#[inline]
pub fn get_lists_dir() -> PathBuf {
    PathBuf::from(oma_apt::raw::config::find_dir(
        "Dir::State::lists".to_string(),
        "lists/".to_string(),
    ))
}

/// Return a writable cache path for APT data files.
///
/// When running as root the system APT cache (`/var/cache/apt/`) is used
/// via `apt_config::find_file`.  For non-root users, this will be found under
/// [`dirs::cache_dir`] (typically `$HOME/.cache/oma/`) instead, so that the
/// cache file can be written by an unprivileged user.
#[inline]
pub fn get_apt_cache_path(key: &str, filename: &str) -> String {
    if crate::root::is_root() {
        oma_apt::raw::config::find_file(key.to_string(), format!("var/cache/apt/{filename}"))
    } else {
        let cache_dir = dirs::cache_dir().unwrap_or_else(|| PathBuf::from("/tmp"));
        cache_dir
            .join("oma")
            .join(filename)
            .to_string_lossy()
            .to_string()
    }
}

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
