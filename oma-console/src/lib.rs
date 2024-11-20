//! # oma-console
//!
//! `oma-console` is a utility crate that provides console functionalities for *oma*
//!
//! It offers modules for printing stylized messages, managing pagers, displaying progress bars, and handling terminal writing utilities.
//!
//! ## Features
//!
//! - **Print**: Styled printing of messages with prefixes and automatic line wrapping.
//! - **Pager**: Terminal pager with scrolling and searching capabilities.
//!
//! ## Modules
//!
//! - `pager`: Implements terminal pager functionality using the `crossterm` and `ratatui` crates.
//! - `pb`: Provides progress bar styles using the `indicatif` crate.
//! - `writer`: Utilities for writing formatted messages to the terminal.
//! - `print`: Printing functionality with support for logging layers and color formatting.
//!

#[cfg(feature = "pager")]
pub mod pager;

#[cfg(feature = "progress_bar_style")]
pub mod pb;

#[cfg(feature = "print")]
pub mod writer;

#[cfg(feature = "print")]
pub mod print;

#[cfg(feature = "print")]
pub use print::OmaLayer;

#[cfg(feature = "print")]
pub use console;

#[cfg(feature = "progress_bar_style")]
pub use indicatif;

#[cfg(feature = "print")]
use writer::Writer;

#[cfg(feature = "print")]
pub static WRITER: std::sync::LazyLock<Writer> = std::sync::LazyLock::new(writer::Writer::default);

#[cfg(feature = "print")]
pub fn is_terminal() -> bool {
    WRITER.is_terminal()
}
