//! # oma-logger
//!
//! Centralized logging parts for oma.
//!
//! This crate provides the logging macros used across the workspace, backed
//! by either `spdlog` (the default `spdlog` feature) or the `log` crate (the
//! `log` feature), plus the `spdlog` surface (levels, sinks, ...) so
//! `spdlog-rs` is only declared as a direct dependency here. With the
//! `formatter` feature it also provides the oma-style spdlog formatter
//! ([`OmaFormatter`]). The concrete logger setup (sinks, filters, log file
//! management) stays in the application.

#[cfg(feature = "spdlog")]
pub use spdlog::__log_impl;
#[cfg(feature = "spdlog")]
pub use spdlog::default_logger;
#[cfg(feature = "spdlog")]
pub use spdlog::error::SendToChannelError;
#[cfg(feature = "spdlog")]
pub use spdlog::sink;
#[cfg(feature = "spdlog")]
pub use spdlog::{
    Error, ErrorHandler, Level, LevelFilter, Logger, Record, StringBuf, init_log_crate_proxy,
    log_crate_proxy, set_default_logger,
};

#[cfg(feature = "formatter")]
mod formatter;

#[cfg(feature = "formatter")]
pub use formatter::OmaFormatter;

/// `spdlog`-backed logging macros: wrappers that expand straight into
/// `$crate::__log_impl!`, skipping the `normalize_forward` proc macro (whose
/// output uses an absolute `::spdlog::` path that would force every consuming
/// crate to depend on `spdlog-rs` directly). `__log_impl!` and the paths
/// referenced here are `$crate`-hygienic, so the `spdlog` crate only needs to
/// be resolvable from this crate.
///
/// Only the plain positional form (`warn!("{}", x)`) is supported; the named
/// logger / `kv:` forms are not needed by oma.
#[cfg(feature = "spdlog")]
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

#[cfg(feature = "spdlog")]
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

#[cfg(feature = "spdlog")]
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

#[cfg(feature = "spdlog")]
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

#[cfg(feature = "spdlog")]
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

#[cfg(feature = "spdlog")]
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

/// `log`-crate-backed logging macros, active when the `spdlog` feature is
/// off. The `log` crate macros are `$crate`-hygienic, so a plain re-export is
/// enough and consuming crates do not need to depend on `log` directly.
#[cfg(all(feature = "log", not(feature = "spdlog")))]
pub use log::{debug, error, info, trace, warn};
/// The `log` crate has no critical level; map it to error.
#[cfg(all(feature = "log", not(feature = "spdlog")))]
#[macro_export]
macro_rules! critical {
    ($($arg:tt)+) => {
        $crate::error!($($arg)+)
    };
}
