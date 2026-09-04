//! # oma-pager
//!
//! `oma-pager` is a utility crate that provides a terminal pager for *oma*
//!
//! It offers a pager with scrolling and searching capabilities, implemented with the `crossterm` and `ratatui` crates.

pub mod highlight;
mod key_binding;
pub mod oma_pager;
pub mod pager;
pub mod traits;

pub use oma_pager::OmaPager;
pub use pager::{Pager, exit_tui, prepare_create_tui};
pub use traits::{PagerExit, PagerTheme, PagerUIText};
