//! # oma-console
//!
//! `oma-console` is a utility crate that provides console functionalities for *oma*
//!
//! It offers modules for printing stylized messages, displaying progress bars, and handling terminal writing utilities.
//!
//! ## Features
//!
//! - **Print**: Stylized message printer with support for prefixes and automatic line wrapping.
//!
//! ## Modules
//!
//! - `writer`: Implements a formatted message writer to the terminal.
//! - `print`: Implements a formatted message logger with support for different logging levels (normal, debug, error, etc.).
//!

#[cfg(feature = "print")]
pub mod writer;

#[cfg(feature = "print")]
pub mod terminal;

#[cfg(feature = "print")]
pub use console;
