pub mod pager;
pub mod pb;
pub mod writer;
pub use console;
pub use dialoguer;
pub use indicatif;
use once_cell::sync::Lazy;
use writer::Writer;

pub type Result<T> = std::result::Result<T, OmaConsoleError>;
pub static WRITER: Lazy<Writer> = Lazy::new(writer::Writer::default);

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
        use oma_console::WRITER as MSG_WRITER;
        MSG_WRITER.writeln("", &format!($($arg)+), false).ok();
    };
}

#[macro_export]
macro_rules! debug {
    ($($arg:tt)+) => {
        use oma_console::WRITER as DEBUG_WRITER;
        DEBUG_WRITER.writeln(&console::style("DEBUG").dim().to_string(), &format!($($arg)+), false).ok();
    };
}

#[macro_export]
macro_rules! success {
    ($($arg:tt)+) => {
        oma_console::WRITER.writeln(&console::style("SUCCESS").green().bold().to_string(), &format!($($arg)+), false).ok();
    };
}

#[macro_export]
macro_rules! info {
    ($($arg:tt)+) => {
        oma_console::WRITER.writeln(&console::style("INFO").blue().bold().to_string(), &format!($($arg)+), false).ok();
    };
}

#[macro_export]
macro_rules! warn {
    ($($arg:tt)+) => {
        oma_console::WRITER.writeln(&console::style("WARNING").yellow().bold().to_string(), &format!($($arg)+), false).ok();
    };
}

#[macro_export]
macro_rules! error {
    ($($arg:tt)+) => {
        oma_console::WRITER.writeln(&console::style("ERROR").red().bold().to_string(), &format!($($arg)+), false).ok();
    };
}

#[macro_export]
macro_rules! due_to {
    ($($arg:tt)+) => {
        use oma_console::WRITER as DUE_TO_WRITER;
        DUE_TO_WRITER.writeln(&console::style("DUE TO").yellow().bold().to_string(), &format!($($arg)+), false).ok();
    };
}
