//! # oma-contents
//!
//! The `oma-contents` crate provides functionality to parse and search contents files from Debian repositories.
//!
//! ## Modules
//!
//! - `parser`: Functions for parsing lines from contents files, extracting file paths and associated packages.
//! - `searcher`: Provides functions to search through contents files, supporting various compression formats and search modes.
//!
//! ## Features
//!
//! - Supports multiple compression formats: Zstandard (`.zst`), LZ4 (`.lz4`), and Gzip (`.gz`).
//! - Provides multiple search modes:
//!   - `Provides`: Search for packages that provide a specific file.
//!   - `Files`: Search for files provided by a specific package.
//!   - `ProvidesSrc`: Search source packages that provide a specific file.
//!   - `FilesSrc`: Search for files provided by a specific source package.
//!   - `BinProvides`: Search for binary packages that provide a specific file.
//!   - `BinFiles`: Search for files provided by a specific binary package.
//! - Utilizes parallel processing for efficient searching.
//! - Supports both ripgrep-based and pure Rust search implementations.
//!

pub mod parser;
pub mod searcher;

#[derive(Debug, thiserror::Error)]
pub enum OmaContentsError {
    #[error("Contents does not exist")]
    ContentsNotExist,
    #[error("Execute ripgrep failed: {0:?}")]
    ExecuteRgFailed(std::io::Error),
    #[error("Failed to read dir or file: {0}, kind: {1}")]
    FailedToOperateDirOrFile(String, std::io::Error),
    #[error("Failed to get file {0} metadata: {1}")]
    FailedToGetFileMetadata(String, std::io::Error),
    #[error("Failed to wait ripgrep to exit: {0}")]
    FailedToWaitExit(std::io::Error),
    #[error("Contents entry missing path list: {0}")]
    ContentsEntryMissingPathList(String),
    #[error("Command not found wrong argument")]
    CnfWrongArgument,
    #[error("Ripgrep exited with error")]
    RgWithError,
    #[error(transparent)]
    LzzzErr(#[from] lzzzz::lz4f::Error),
    #[error("")]
    NoResult,
    #[error("Illegal file: {0}")]
    IllegalFile(String),
    #[error("Invaild contents")]
    InvaildContents,
}
