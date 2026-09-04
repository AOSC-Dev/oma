//! # oma-console
//!
//! `oma-console` is a utility crate that provides console functionalities for *oma*
//!
//! It offers modules for printing stylized messages and handling terminal writing utilities.
//!
//! ## Modules
//!
//! - `writer`: Implements a formatted message writer to the terminal.
//! - `terminal`: Implements terminal writing utilities.
//!

pub mod writer;

pub mod terminal;

pub use console;
