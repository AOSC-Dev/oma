//! # oma-logger
//!
//! Centralized logging parts for oma.
//!
//! This crate re-exports the `spdlog` surface used across the workspace
//! (macros, levels, sinks, ...), so `spdlog-rs` is only declared as a direct
//! dependency here. With the `formatter` feature it also provides the
//! oma-style spdlog formatter ([`OmaFormatter`]). The concrete logger setup
//! (sinks, filters, log file management) stays in the application.

pub use spdlog::__log_impl;
pub use spdlog::default_logger;
pub use spdlog::error::SendToChannelError;
pub use spdlog::sink;
pub use spdlog::{
    Error, ErrorHandler, Level, LevelFilter, Logger, Record, StringBuf, init_log_crate_proxy,
    log_crate_proxy, set_default_logger,
};

#[cfg(feature = "formatter")]
mod formatter;

#[cfg(feature = "formatter")]
pub use formatter::OmaFormatter;

/// Wrapper macros that expand straight into `$crate::__log_impl!`, skipping
/// the `normalize_forward` proc macro (whose output uses an absolute
/// `::spdlog::` path that would force every consuming crate to depend on
/// `spdlog-rs` directly). `__log_impl!` and the paths referenced here are
/// `$crate`-hygienic, so the `spdlog` crate only needs to be resolvable from
/// this crate.
///
/// Only the plain positional form (`warn!("{}", x)`) is supported; the named
/// logger / `kv:` forms are not needed by oma.
#[macro_export]
macro_rules! trace {
    ($($arg:tt)+) => {
        $crate::__log_impl!(
            logger: $crate::default_logger(),
            kv: {},
            $crate::Level::Trace,
            $($arg)+
        )
    };
}
#[macro_export]
macro_rules! debug {
    ($($arg:tt)+) => {
        $crate::__log_impl!(
            logger: $crate::default_logger(),
            kv: {},
            $crate::Level::Debug,
            $($arg)+
        )
    };
}
#[macro_export]
macro_rules! info {
    ($($arg:tt)+) => {
        $crate::__log_impl!(
            logger: $crate::default_logger(),
            kv: {},
            $crate::Level::Info,
            $($arg)+
        )
    };
}
#[macro_export]
macro_rules! warn {
    ($($arg:tt)+) => {
        $crate::__log_impl!(
            logger: $crate::default_logger(),
            kv: {},
            $crate::Level::Warn,
            $($arg)+
        )
    };
}
#[macro_export]
macro_rules! error {
    ($($arg:tt)+) => {
        $crate::__log_impl!(
            logger: $crate::default_logger(),
            kv: {},
            $crate::Level::Error,
            $($arg)+
        )
    };
}
#[macro_export]
macro_rules! log_error {
    ($($arg:tt)+) => {
        $crate::__log_impl!(
            logger: $crate::default_logger(),
            kv: {},
            $crate::Level::Error,
            $($arg)+
        )
    };
}
#[macro_export]
macro_rules! critical {
    ($($arg:tt)+) => {
        $crate::__log_impl!(
            logger: $crate::default_logger(),
            kv: {},
            $crate::Level::Critical,
            $($arg)+
        )
    };
}
