use std::{io, path::PathBuf};

use snafu::Snafu;

/// Errors while interpreting, loading or resolving a mirror list.
#[derive(Debug, Snafu)]
pub enum MirrorError {
    #[snafu(display("Unsupported mirror protocol: {uri}"))]
    UnsupportedScheme { uri: String },

    #[snafu(display("Invalid mirror URI: {uri}"))]
    InvalidUri { uri: String },

    #[snafu(display("Failed to read mirror list {}: {source}", path.display()))]
    ReadFile { path: PathBuf, source: io::Error },

    #[snafu(display("Failed to fetch mirror list {url}: {source}"))]
    Fetch {
        url: String,
        source: reqwest_middleware::Error,
    },

    #[snafu(display("Failed to read mirror list body from {url}: {source}"))]
    ReadBody { url: String, source: reqwest::Error },

    #[snafu(display("Mirror list {uri} is larger than {limit} bytes"))]
    TooLarge { uri: String, limit: u64 },

    #[snafu(display("Mirror list {uri} is empty"))]
    EmptyList { uri: String },

    #[snafu(display("Mirror list {uri} contains no mirror with a supported transport"))]
    NoSupportedMirrors { uri: String },

    #[snafu(display("Mirror list {uri} uses unsupported compression: .{extension}"))]
    UnsupportedCompression { uri: String, extension: String },

    #[snafu(display("Mirror list {uri} is not valid UTF-8"))]
    InvalidEncoding { uri: String },

    #[snafu(display("No mirror in {uri} matches the requested target"))]
    NoCandidates { uri: String },

    #[snafu(display("URL {url} does not belong to mirror source {source_uri}"))]
    UrlMismatch { source_uri: String, url: String },
}
