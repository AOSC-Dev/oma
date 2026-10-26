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
/// `mirror://host/path` is the legacy spelling of
/// `mirror+http://host/path`, matching apt.
pub fn parse_mirror_uri(uri: &str) -> Result<MirrorListLocation, MirrorError> {
    if let Some(rest) = uri.strip_prefix("mirror://") {
        let url = format!("http://{rest}");
        let parsed = validate_http_uri(uri, &url)?;
        Ok(MirrorListLocation::Http(normalize_http_url(&parsed, &url)))
    } else if let Some(rest) = uri.strip_prefix("mirror+http://") {
        let url = format!("http://{rest}");
        let parsed = validate_http_uri(uri, &url)?;
        Ok(MirrorListLocation::Http(normalize_http_url(&parsed, &url)))
    } else if let Some(rest) = uri.strip_prefix("mirror+https://") {
        let url = format!("https://{rest}");
        let parsed = validate_http_uri(uri, &url)?;
        Ok(MirrorListLocation::Http(normalize_http_url(&parsed, &url)))
    } else if let Some(rest) = uri.strip_prefix("mirror+file:") {
        // Require an absolute path. `url::Url` happily turns `file:relative`
        // into `/relative`, which would be a surprising interpretation of a
        // sources.list entry.
        if !rest.starts_with('/') {
            return Err(MirrorError::InvalidUri {
                uri: uri.to_string(),
            });
        }

        let parsed = Url::parse(&format!("file:{rest}")).map_err(|_| MirrorError::InvalidUri {
            uri: uri.to_string(),
        })?;
        let path = parsed.to_file_path().map_err(|_| MirrorError::InvalidUri {
            uri: uri.to_string(),
        })?;

        if !path.is_absolute() {
            return Err(MirrorError::InvalidUri {
                uri: uri.to_string(),
            });
        }

        Ok(MirrorListLocation::File(normalize_file_path(path)))
    } else {
        Err(MirrorError::UnsupportedScheme {
            uri: uri.to_string(),
        })
    }
}

/// Drop one trailing slash from the mirror list URL's path. The list is a
/// file, not a directory; apt does the same before acquiring it.
///
/// The URL is edited as text so parsing does not rewrite the host, default
/// port or dot segments; `parsed` is only used to identify the root and the
/// path's trailing slash.
fn normalize_http_url(parsed: &Url, original: &str) -> String {
    if parsed.path() == "/" || !parsed.path().ends_with('/') {
        return original.to_string();
    }

    let path_end = original.find(['?', '#']).unwrap_or(original.len());

    if !original[..path_end].ends_with('/') {
        return original.to_string();
    }

    let mut normalized = String::with_capacity(original.len() - 1);
    normalized.push_str(&original[..path_end - 1]);
    normalized.push_str(&original[path_end..]);
    normalized
}

fn normalize_file_path(path: PathBuf) -> PathBuf {
    path.components().as_path().to_path_buf()
}

fn validate_http_uri(uri: &str, url: &str) -> Result<Url, MirrorError> {
    let parsed = Url::parse(url).map_err(|_| MirrorError::InvalidUri {
        uri: uri.to_string(),
    })?;

    if parsed.host_str().is_none() || !matches!(parsed.scheme(), "http" | "https") {
        return Err(MirrorError::InvalidUri {
            uri: uri.to_string(),
        });
    }

    Ok(parsed)
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
            parse_mirror_uri("mirror+file:/").unwrap(),
            MirrorListLocation::File(PathBuf::from("/"))
        );
        assert_eq!(
            parse_mirror_uri("mirror+http://host/").unwrap(),
            MirrorListLocation::Http("http://host/".into())
        );
        // Only the path's trailing slash is removed; the rest of the URL is
        // kept as written.
        assert_eq!(
            parse_mirror_uri("mirror+http://HOST:80/mirrors.txt/").unwrap(),
            MirrorListLocation::Http("http://HOST:80/mirrors.txt".into())
        );
        assert_eq!(
            parse_mirror_uri("mirror+http://host/mirrors.txt/?cache=1/").unwrap(),
            MirrorListLocation::Http("http://host/mirrors.txt?cache=1/".into())
        );
        assert_eq!(
            parse_mirror_uri("mirror+http://host").unwrap(),
            MirrorListLocation::Http("http://host".into())
        );
        assert!(parse_mirror_uri("mirror+https://").is_err());
        assert!(parse_mirror_uri("mirror://").is_err());
        assert!(parse_mirror_uri("mirror+ftp://host/list").is_err());
        assert!(parse_mirror_uri("mirror+copy:/list").is_err());
        assert!(parse_mirror_uri("mirror+file:relative").is_err());
        assert!(parse_mirror_uri("http://host/list").is_err());
    }
}
