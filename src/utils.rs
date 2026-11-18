use oma_pm::oma_apt;
use oma_repo_verify::PolicyPaths;
use std::path::{Path, PathBuf};

/// Sequoia policy configuration paths for `sysroot`, honouring the Termux
/// layout where `/usr` lives directly below the prefix.
pub fn policy_paths(sysroot: &Path) -> PolicyPaths {
    if oma_utils::is_termux() {
        PolicyPaths {
            custom: oma_utils::termux::host_path(
                Path::new("/"),
                "/etc/crypto-policies/back-ends/apt-sequoia.config",
            ),
            default: oma_utils::termux::host_path(
                Path::new("/"),
                "/usr/share/apt/default-sequoia.config",
            ),
        }
    } else {
        PolicyPaths::for_rootfs(sysroot)
    }
}

#[inline]
pub fn get_lists_dir() -> PathBuf {
    PathBuf::from(oma_apt::raw::config::find_dir(
        "Dir::State::lists".to_string(),
        "lists/".to_string(),
    ))
}

/// oma display normal message
#[macro_export]
macro_rules! msg {
    ($($arg:tt)+) => {
        use oma_console::writer::Writeln as _;
        let s = format!($($arg)+);
        oma_logger::debug!("{s}");
        $crate::WRITER.writeln("", &s).ok();
    };
}

/// oma display success message
#[macro_export]
macro_rules! success {
    ($($arg:tt)+) => {
        use oma_console::writer::Writeln as _;
        let s = format!($($arg)+);
        oma_logger::debug!("{s}");
        $crate::WRITER
            .writeln(&$crate::color::Colorize::emphasis_color("SUCCESS").bold().to_string(), &s)
            .ok();
    };
}

/// oma display due_to message
#[macro_export]
macro_rules! due_to {
    ($($arg:tt)+) => {
        use oma_console::writer::Writeln as _;
        let s = format!($($arg)+);
        oma_logger::debug!("{s}");
        $crate::WRITER
            .writeln(&$crate::color::Colorize::note_color("DUE TO").bold().to_string(), &s)
            .ok();
    };
}
