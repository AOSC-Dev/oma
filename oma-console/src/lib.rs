pub mod pager;
pub mod writer;
pub mod pb;
pub use console;
pub use indicatif;

pub type Result<T> = std::result::Result<T, OmaConsoleError>;

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
    ($write:ident, $($arg:tt)+) => {
        $write.writeln("", &format!($($arg)+), false).ok();
    };
}

#[macro_export]
macro_rules! debug {
    ($write:ident, $($arg:tt)+) => {
        $write.writeln(&console::style("DEBUG").dim().to_string(), &format!($($arg)+), false).ok();
    };
}

#[macro_export]
macro_rules! success {
    ($write:ident, $($arg:tt)+) => {
        $write.writeln(&console::style("SUCCESS").green().bold().to_string(), &format!($($arg)+), false).ok();
    };
}

#[macro_export]
macro_rules! info {
    ($write:ident, $($arg:tt)+) => {
        $write.writeln(&console::style("INFO").blue().bold().to_string(), &format!($($arg)+), false).ok();
    };
}

#[macro_export]
macro_rules! warn {
    ($write:ident, $($arg:tt)+) => {
        $write.writeln(&console::style("WARNING").yellow().bold().to_string(), &format!($($arg)+), false).ok();
    };
}

#[macro_export]
macro_rules! error {
    ($write:ident, $($arg:tt)+) => {
        $write.writeln(&console::style("ERROR").red().bold().to_string(), &format!($($arg)+), false).ok();
    };
}

#[macro_export]
macro_rules! due_to {
    ($write:ident, $($arg:tt)+) => {
        $write.writeln(&console::style("DUE TO").yellow().bold().to_string(), &format!($($arg)+), false).ok();
    };
}
