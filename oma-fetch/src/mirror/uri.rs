use std::path::PathBuf;

use reqwest::Url;

use super::error::MirrorError;

/// Where a mirror list is read from.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MirrorListLocation {
    /// A remote mirror list, fetched with the configured HTTP client.
    Http(String),
    /// A local mirror list.
    File(PathBuf),
}

impl MirrorListLocation {
    /// Whether the list itself is local. Only local lists may name `file:`
    /// mirrors.
    pub fn is_local(&self) -> bool {
        matches!(self, Self::File(_))
    }
}

/// Interpret a `mirror://` / `mirror+<transport>:` URI as the location of its
/// mirror list.
///
/// The path and trailing-slash handling follows apt's `mirror.cc`: the path
/// starts at the first `/` after the authority and runs to the end of the URI,
/// so a query or fragment is part of it. One trailing `/` is removed when the
/// path is longer than `/`. A `file:` source with a non-empty authority is
/// rejected, like apt's file method.
///
/// `mirror://host/path` is the legacy spelling of
/// `mirror+http://host/path`, matching apt.
pub fn parse_mirror_uri(uri: &str) -> Result<MirrorListLocation, MirrorError> {
    let Some(colon) = uri.find(':') else {
        return Err(MirrorError::UnsupportedScheme {
            uri: uri.to_string(),
        });
    };

    let access = &uri[..colon];
    let scheme = if access == "mirror" {
        "http"
    } else if let Some(scheme) = access.strip_prefix("mirror+") {
        scheme
    } else {
        return Err(MirrorError::UnsupportedScheme {
            uri: uri.to_string(),
        });
    };

    if !scheme.eq_ignore_ascii_case("http")
        && !scheme.eq_ignore_ascii_case("https")
        && !scheme.eq_ignore_ascii_case("file")
    {
        return Err(MirrorError::UnsupportedScheme {
            uri: uri.to_string(),
        });
    }

    let rest = &uri[colon + 1..];
    let path = apt_path(rest);

    // apt's `::URI` keeps `?query` and `#fragment` inside Path, and Path is
    // always a suffix of the URI, so removing its last byte is enough.
    let uri = if path.len() > 1 && path.ends_with('/') {
        &uri[..uri.len() - 1]
    } else {
        uri
    };

    let rest = &uri[colon + 1..];
    let path = apt_path(rest);

    if scheme.eq_ignore_ascii_case("file") {
        // apt's file method rejects any authority, including `localhost`.
        if !rest.starts_with('/') {
            return Err(MirrorError::InvalidUri {
                uri: uri.to_string(),
            });
        }

        let host = rest
            .strip_prefix("//")
            .and_then(|authority| authority.split('/').next())
            .unwrap_or_default();

        if !host.is_empty() {
            return Err(MirrorError::InvalidUri {
                uri: uri.to_string(),
            });
        }

        return file_location(uri, path);
    }

    let url = format!("{scheme}:{rest}");
    validate_http_uri(uri, &url)?;
    Ok(MirrorListLocation::Http(url))
}

/// The path part of an apt `::URI`: everything from the first `/` after the
/// authority to the end, or `/` when there is no path.
fn apt_path(rest: &str) -> &str {
    let start = if rest.starts_with("//") {
        let mut in_bracket = false;
        let bytes = rest.as_bytes();
        let mut index = 2;

        while index < bytes.len() {
            match bytes[index] {
                b'[' => in_bracket = true,
                b']' => in_bracket = false,
                b'/' if !in_bracket => break,
                _ => {}
            }
            index += 1;
        }

        index
    } else {
        rest.find('/').unwrap_or(rest.len())
    };

    if start < rest.len() {
        &rest[start..]
    } else {
        "/"
    }
}

fn file_location(uri: &str, path: &str) -> Result<MirrorListLocation, MirrorError> {
    let path = if path.starts_with("//") || path.contains(['?', '#']) {
        // apt keeps query/fragment in Path and opens them as file name
        // characters. A path starting with `//` would be parsed as an
        // authority by `Url`, so keep both raw.
        PathBuf::from(path)
    } else {
        let parsed = Url::parse(&format!("file:{path}")).map_err(|_| MirrorError::InvalidUri {
            uri: uri.to_string(),
        })?;
        parsed.to_file_path().map_err(|_| MirrorError::InvalidUri {
            uri: uri.to_string(),
        })?
    };

    if !path.is_absolute() {
        return Err(MirrorError::InvalidUri {
            uri: uri.to_string(),
        });
    }

    Ok(MirrorListLocation::File(normalize_file_path(path)))
}

fn normalize_file_path(path: PathBuf) -> PathBuf {
    path.components().as_path().to_path_buf()
}

fn validate_http_uri(uri: &str, url: &str) -> Result<(), MirrorError> {
    let parsed = Url::parse(url).map_err(|_| MirrorError::InvalidUri {
        uri: uri.to_string(),
    })?;

    if parsed.host_str().is_none() || !matches!(parsed.scheme(), "http" | "https") {
        return Err(MirrorError::InvalidUri {
            uri: uri.to_string(),
        });
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_mirror_uri_forms() {
        assert_eq!(
            parse_mirror_uri("mirror://mirrors.example.com/mirrors.txt").unwrap(),
            MirrorListLocation::Http("http://mirrors.example.com/mirrors.txt".into())
        );
        assert_eq!(
            parse_mirror_uri("mirror+http://host/mirrors.txt").unwrap(),
            MirrorListLocation::Http("http://host/mirrors.txt".into())
        );
        assert_eq!(
            parse_mirror_uri("mirror+https://host/mirrors.txt").unwrap(),
            MirrorListLocation::Http("https://host/mirrors.txt".into())
        );
        assert_eq!(
            parse_mirror_uri("mirror+file:/etc/apt/mirrors.list").unwrap(),
            MirrorListLocation::File(PathBuf::from("/etc/apt/mirrors.list"))
        );
        assert_eq!(
            parse_mirror_uri("mirror+file:///etc/apt/mirrors.list").unwrap(),
            MirrorListLocation::File(PathBuf::from("/etc/apt/mirrors.list"))
        );
        assert_eq!(
            parse_mirror_uri("mirror+file:/etc/apt/mirrors.list/").unwrap(),
            MirrorListLocation::File(PathBuf::from("/etc/apt/mirrors.list"))
        );
        assert_eq!(
            parse_mirror_uri("mirror+http://host/mirrors.txt/").unwrap(),
            MirrorListLocation::Http("http://host/mirrors.txt".into())
        );
        assert_eq!(
            parse_mirror_uri("mirror://host/mirrors.txt/").unwrap(),
            MirrorListLocation::Http("http://host/mirrors.txt".into())
        );
        assert_eq!(
            parse_mirror_uri("mirror+file:/").unwrap(),
            MirrorListLocation::File(PathBuf::from("/"))
        );
        assert_eq!(
            parse_mirror_uri("mirror+http://host/").unwrap(),
            MirrorListLocation::Http("http://host/".into())
        );
        assert_eq!(
            parse_mirror_uri("mirror+http://host").unwrap(),
            MirrorListLocation::Http("http://host".into())
        );
        // Only the path's trailing slash is removed; the rest of the URL is
        // kept as written.
        assert_eq!(
            parse_mirror_uri("mirror+http://HOST:80/mirrors.txt/").unwrap(),
            MirrorListLocation::Http("http://HOST:80/mirrors.txt".into())
        );
        // apt's ::URI treats query/fragment as part of Path, so a trailing
        // slash there is removed as well.
        assert_eq!(
            parse_mirror_uri("mirror+http://host/mirrors.txt/?cache=1/").unwrap(),
            MirrorListLocation::Http("http://host/mirrors.txt/?cache=1".into())
        );
        assert_eq!(
            parse_mirror_uri("mirror+http://host/mirrors.txt/#frag/").unwrap(),
            MirrorListLocation::Http("http://host/mirrors.txt/#frag".into())
        );
        assert_eq!(
            parse_mirror_uri("mirror+http://host/mirrors.txt/?cache=1").unwrap(),
            MirrorListLocation::Http("http://host/mirrors.txt/?cache=1".into())
        );
        assert!(parse_mirror_uri("mirror+file://localhost/etc/apt/mirrors.list").is_err());
        assert!(parse_mirror_uri("mirror+file://host/etc/apt/mirrors.list").is_err());
        assert!(parse_mirror_uri("mirror+https://").is_err());
        assert!(parse_mirror_uri("mirror://").is_err());
        assert!(parse_mirror_uri("mirror+ftp://host/list").is_err());
        assert!(parse_mirror_uri("mirror+copy:/list").is_err());
        assert!(parse_mirror_uri("mirror+file:relative").is_err());
        assert!(parse_mirror_uri("http://host/list").is_err());
    }
}
