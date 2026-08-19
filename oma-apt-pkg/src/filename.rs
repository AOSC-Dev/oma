//! APT lists filename encoding and decoding.
//!
//! APT stores repository index files under `/var/lib/apt/lists/` with names
//! derived from the original URI via `URItoFileName`. The encoding replaces
//! certain characters to produce a safe filesystem name:
//!
//! | Character | Encoded as | Reason |
//! |-----------|------------|--------|
//! | `_`       | `%5f`      | Escaped so it's not confused with path separators |
//! | `/`       | `_`        | Path separator becomes underscore |
//! | `+`       | `%252b`    | Plus sign (double-encoded, see below) |
//! | `~`       | `%257e`    | Tilde (double-encoded, see below) |
//! | ` `       | `%2520`    | Space (double-encoded, see below) |
//! | `@`       | `%40`      | At-sign |
//!
//! About `+`, `~` and space: apt percent-encodes the suite via
//! `pkgAcquire::URIEncode` (acquire.cc — the default `Acquire::URIEncode`
//! set is `+~ `; `+` was added as a workaround for an S3 bug,
//! LP#1003633/LP#1086997) before building the index URI, and its
//! `URItoFileName` then re-encodes the `%`, so `+` -> `%252b`, `~` ->
//! `%257e` and space -> `%2520` in list filenames. oma-refresh emits the
//! same encodings (see `test_url_encode_plus`, from commit d9287e33).
//!
//! The [`AptListFilename`] struct holds both directions, using `aho-corasick`
//! for simultaneous, order-independent replacement.

use std::borrow::Cow;

use aho_corasick::AhoCorasick;
use url::{Host, Url};

/// Errors that can occur during APT list filename encoding or decoding.
#[derive(Debug, thiserror::Error)]
pub enum FilenameError {
    /// The Aho-Corasick replacement engine failed.
    #[error("filename replacement failed: {0}")]
    Replace(#[from] std::io::Error),
    /// The output of the replacement was not valid UTF-8.
    #[error("filename output is not valid utf-8: {0}")]
    InvalidUtf8(#[from] std::string::FromUtf8Error),
}

/// Result type for [`AptListFilename`] operations.
pub type FilenameResult<T> = Result<T, FilenameError>;

/// Bidirectional APT list filename converter.
///
/// This struct wraps two [`AhoCorasick`] automata — one for encoding
/// (host+path → filename stem) and one for decoding (filename stem →
/// host+path), matching the substitution rules of APT's `URItoFileName`
/// and `oma-refresh`'s `DatabaseFilenameReplacer`.
///
/// # Examples
///
/// ```
/// # use oma_apt_pkg::AptListFilename;
/// let cvt = AptListFilename::new();
/// assert_eq!(
///     cvt.encode("mirrors.example.com/debian/dists/bookworm/main/binary-amd64").unwrap(),
///     "mirrors.example.com_debian_dists_bookworm_main_binary-amd64"
/// );
/// assert_eq!(
///     cvt.decode("mirrors.example.com_debian_dists_bookworm_main_binary-amd64").unwrap(),
///     "mirrors.example.com/debian/dists/bookworm/main/binary-amd64"
/// );
/// ```
pub struct AptListFilename {
    encoder: AhoCorasick,
    decoder: AhoCorasick,
}

impl Default for AptListFilename {
    fn default() -> Self {
        Self::new()
    }
}

impl AptListFilename {
    /// Patterns for encoding (host+path → filename stem).
    const ENCODE_PATTERNS: &[&str] = &["_", "/", "+", "~", " ", "%3a", "%3A", "@"];
    const ENCODE_REPLACE: &[&str] = &["%5f", "_", "%252b", "%257e", "%2520", ":", ":", "%40"];

    /// Patterns for decoding (filename stem → host+path).
    const DECODE_PATTERNS: &[&str] = &["%252b", "%257e", "%2520", "%40", "_", "%5f"];
    const DECODE_REPLACE: &[&str] = &["+", "~", " ", "@", "/", "_"];

    /// Build a new converter with the standard APT substitution rules.
    pub fn new() -> Self {
        Self {
            encoder: AhoCorasick::new(Self::ENCODE_PATTERNS).expect("valid encode patterns"),
            decoder: AhoCorasick::new(Self::DECODE_PATTERNS).expect("valid decode patterns"),
        }
    }

    /// Encode a URL or host+path into an APT list filename stem.
    ///
    /// If the input is a valid URL (has a scheme), the scheme is stripped
    /// and the host+path is encoded — matching APT's `URItoFileName`.
    /// Otherwise the input is treated as raw host+path directly.
    ///
    /// Input:  `https://mirrors.example.com/debian/dists/bookworm/main/binary-amd64/Packages`
    /// Output: `mirrors.example.com_debian_dists_bookworm_main_binary-amd64_Packages`
    ///
    /// Input:  `mirrors.example.com/debian/dists/bookworm/main/binary-amd64`
    /// Output: `mirrors.example.com_debian_dists_bookworm_main_binary-amd64`
    pub fn encode(&self, input: &str) -> FilenameResult<String> {
        let host_path: Cow<'_, str> = match Url::parse(input) {
            Ok(url_parsed) => {
                let host = url_parsed.host_str();

                // Don't use url_parsed.path() — it normalises "/./" to "/"
                let path = if let Some(host) = host {
                    input.split_once(host).map(|(_, p)| p).unwrap_or(input)
                } else {
                    input
                        .strip_prefix("file://")
                        .or_else(|| input.strip_prefix("file:"))
                        .unwrap_or(input)
                };

                if let Some(host) = host {
                    if let Some(Host::Ipv6(addr)) = url_parsed.host() {
                        Cow::Owned(format!("{addr}{path}"))
                    } else {
                        Cow::Owned(format!("{host}{path}"))
                    }
                } else {
                    path.into()
                }
            }
            Err(_) => {
                // Not a valid URL — treat as raw host+path
                input.into()
            }
        };

        let mut buf = Vec::new();
        self.encoder.try_stream_replace_all(
            host_path.as_bytes(),
            &mut buf,
            Self::ENCODE_REPLACE,
        )?;

        String::from_utf8(buf).map_err(FilenameError::InvalidUtf8)
    }

    /// Decode an APT list filename stem back to the original host+path.
    ///
    /// Input:  `mirrors.example.com_debian_dists_bookworm_main_binary-amd64`
    /// Output: `mirrors.example.com/debian/dists/bookworm/main/binary-amd64`
    ///
    /// Handles files created by both APT (standard `URItoFileName`) and
    /// `oma-refresh` (which double-encodes `+` as `%252b`).
    pub fn decode(&self, input: &str) -> FilenameResult<String> {
        let mut buf = Vec::new();
        self.decoder
            .try_stream_replace_all(input.as_bytes(), &mut buf, Self::DECODE_REPLACE)?;

        String::from_utf8(buf).map_err(FilenameError::InvalidUtf8)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn cvt() -> AptListFilename {
        AptListFilename::new()
    }

    // --- Ported from oma-refresh's DatabaseFilenameReplacer tests ---

    #[test]
    fn test_encode_plus_like_oma_refresh() {
        let cvt = cvt();
        let input = "repo.aosc.io/debs/dists/x264-0+git20240305/InRelease";
        assert_eq!(
            cvt.encode(input).unwrap(),
            "repo.aosc.io_debs_dists_x264-0%252bgit20240305_InRelease"
        );
    }

    #[test]
    fn test_encode_underline_like_oma_refresh() {
        let cvt = cvt();
        let input = "repo.aosc.io/debs/dists/xorg-server-21.1.13-hyperv_drm-fix/InRelease";
        assert_eq!(
            cvt.encode(input).unwrap(),
            "repo.aosc.io_debs_dists_xorg-server-21.1.13-hyperv%5fdrm-fix_InRelease"
        );
    }

    #[test]
    fn test_encode_file_protocol() {
        let cvt = cvt();
        assert_eq!(cvt.encode("/debs").unwrap(), "_debs");
    }

    #[test]
    fn test_decode_plus_roundtrip() {
        let cvt = cvt();
        let original = "repo.aosc.io/debs/dists/x264-0+git20240305/InRelease";
        assert_eq!(
            cvt.decode(&cvt.encode(original).unwrap()).unwrap(),
            original
        );
    }

    #[test]
    fn test_decode_underline_roundtrip() {
        let cvt = cvt();
        let original = "repo.aosc.io/debs/dists/xorg-server-21.1.13-hyperv_drm-fix/InRelease";
        assert_eq!(
            cvt.decode(&cvt.encode(original).unwrap()).unwrap(),
            original
        );
    }

    // --- Ported from oma-refresh: path characters ---

    #[test]
    fn test_encode_preserves_colon() {
        let cvt = cvt();
        let input = "ci.deepin.com/repo/obs/deepin:/CI:/TestingIntegration:/test-integration-pr-1537/testing/./Packages";
        assert_eq!(
            cvt.encode(input).unwrap(),
            "ci.deepin.com_repo_obs_deepin:_CI:_TestingIntegration:_test-integration-pr-1537_testing_._Packages"
        );
    }

    #[test]
    fn test_encode_preserves_dot() {
        let cvt = cvt();
        assert_eq!(
            cvt.encode("././debs/./Packages").unwrap(),
            "._._debs_._Packages"
        );
        assert_eq!(
            cvt.encode("/././debs/./Packages").unwrap(),
            "_._._debs_._Packages"
        );
    }

    // --- Original tests ---

    #[test]
    fn test_encode_basic() {
        let cvt = cvt();
        let input = "mirrors.example.com/debian/dists/bookworm/main/binary-amd64/Packages";
        let expected = "mirrors.example.com_debian_dists_bookworm_main_binary-amd64_Packages";
        assert_eq!(cvt.encode(input).unwrap(), expected);
    }

    #[test]
    fn test_decode_basic() {
        let cvt = cvt();
        let input = "mirrors.example.com_debian_dists_bookworm_main_binary-amd64";
        let expected = "mirrors.example.com/debian/dists/bookworm/main/binary-amd64";
        assert_eq!(cvt.decode(input).unwrap(), expected);
    }

    #[test]
    fn test_roundtrip() {
        let cvt = cvt();
        let original = "mirrors.example.com/debian/dists/bookworm/main/binary-amd64";
        let encoded = cvt.encode(original).unwrap();
        let decoded = cvt.decode(&encoded).unwrap();
        assert_eq!(decoded, original);
    }

    #[test]
    fn test_decode_underscore_in_path() {
        let cvt = cvt();
        let input = "repo.example.com_debs_my%5frepo_dists_stable_main_binary-amd64";
        let expected = "repo.example.com/debs/my_repo/dists/stable/main/binary-amd64";
        assert_eq!(cvt.decode(input).unwrap(), expected);
    }

    #[test]
    fn test_decode_oma_refresh_plus() {
        let cvt = cvt();
        let input = "repo.example.com_debian_dists_stable_%252bcontrib_binary-amd64";
        let expected = "repo.example.com/debian/dists/stable/+contrib/binary-amd64";
        assert_eq!(cvt.decode(input).unwrap(), expected);
    }

    #[test]
    fn test_decode_at_sign() {
        let cvt = cvt();
        let input = "repo.example.com_debs_%40special_dists_stable_main_binary-amd64";
        let expected = "repo.example.com/debs/@special/dists/stable/main/binary-amd64";
        assert_eq!(cvt.decode(input).unwrap(), expected);
    }

    #[test]
    fn test_encode_underscore_in_path() {
        let cvt = cvt();
        let input = "repo.example.com/debs/my_repo/dists/stable/main/binary-amd64";
        let encoded = cvt.encode(input).unwrap();
        assert_eq!(
            encoded,
            "repo.example.com_debs_my%5frepo_dists_stable_main_binary-amd64"
        );
        assert_eq!(cvt.decode(&encoded).unwrap(), input);
    }

    #[test]
    fn test_encode_at_sign() {
        let cvt = cvt();
        let input = "repo.example.com/debs/@special/dists/stable/main/binary-amd64";
        let encoded = cvt.encode(input).unwrap();
        assert_eq!(
            encoded,
            "repo.example.com_debs_%40special_dists_stable_main_binary-amd64"
        );
        assert_eq!(cvt.decode(&encoded).unwrap(), input);
    }

    #[test]
    fn test_encode_tilde_and_space() {
        let cvt = cvt();
        // Like apt's URIEncode default set (+~ ), ~ and space are
        // double-encoded too: ~ -> %7e -> %257e, space -> %20 -> %2520.
        let original = "repo.example.com/debs/dists/foo~bar baz/InRelease";
        let encoded = cvt.encode(original).unwrap();
        assert_eq!(
            encoded,
            "repo.example.com_debs_dists_foo%257ebar%2520baz_InRelease"
        );
        assert_eq!(cvt.decode(&encoded).unwrap(), original);
    }

    #[test]
    fn test_empty_input() {
        let cvt = cvt();
        assert_eq!(cvt.encode("").unwrap(), "");
        assert_eq!(cvt.decode("").unwrap(), "");
    }
}
