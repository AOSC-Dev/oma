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
