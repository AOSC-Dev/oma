pub mod pager;
pub mod pb;
pub mod writer;
use std::sync::atomic::AtomicBool;

pub use console;
pub use dialoguer;
pub use indicatif;
use once_cell::sync::Lazy;
use writer::Writer;

pub type OmaConsoleResult<T> = std::result::Result<T, OmaConsoleError>;
pub static WRITER: Lazy<Writer> = Lazy::new(writer::Writer::default);
pub static DEBUG: AtomicBool = AtomicBool::new(false);

#[derive(Debug, thiserror::Error)]
pub enum OmaConsoleError {
    #[error(transparent)]
    IoError(#[from] std::io::Error),
    #[error("No stdin")]
    StdinDoesNotExist,
}

// We will ignore write errors in the following macros, since cannot print messages is not an emergency
#[macro_export]
macro_rules! msg {
    ($($arg:tt)+) => {
        oma_console::WRITER.writeln("", &format!($($arg)+), false).ok();
    };
}

#[macro_export]
macro_rules! debug {
    ($($arg:tt)+) => {
        if oma_console::DEBUG.load(std::sync::atomic::Ordering::Relaxed) {
            oma_console::WRITER.writeln(&oma_console::console::style("DEBUG").dim().to_string(), &format!($($arg)+), false).ok();
        }
    };
}

#[macro_export]
macro_rules! success {
    ($($arg:tt)+) => {
        oma_console::WRITER.writeln(&oma_console::console::style("SUCCESS").green().bold().to_string(), &format!($($arg)+), false).ok();
    };
}

#[macro_export]
macro_rules! info {
    ($($arg:tt)+) => {
        oma_console::WRITER.writeln(&oma_console::console::style("INFO").blue().bold().to_string(), &format!($($arg)+), false).ok();
    };
}

#[macro_export]
macro_rules! warn {
    ($($arg:tt)+) => {
        oma_console::WRITER.writeln(&oma_console::console::style("WARNING").yellow().bold().to_string(), &format!($($arg)+), false).ok();
    };
}

#[macro_export]
macro_rules! error {
    ($($arg:tt)+) => {
        oma_console::WRITER.writeln(&oma_console::console::style("ERROR").red().bold().to_string(), &format!($($arg)+), false).ok();
    };
}

#[macro_export]
macro_rules! due_to {
    ($($arg:tt)+) => {
        oma_console::WRITER.writeln(&oma_console::console::style("DUE TO").yellow().bold().to_string(), &format!($($arg)+), false).ok();
    };
}
