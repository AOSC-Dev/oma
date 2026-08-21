//! Resolving apt's `mirror://` protocol.
//!
//! Mirrors the behavior of apt's `methods/mirror.cc`: a `mirror://` (or
//! `mirror+http(s)://` / `mirror+file://`) URI points at a *mirror list* file
//! whose lines name the actual repository base URIs. The list is fetched over
//! the network or read from a local file, parsed, filtered and sorted, and the
//! result is the ordered set of concrete base URLs to use for downloads.
//!
//! Mirror list format (apt compatible), one entry per line:
//!
//! ```text
//! http://mirror1.example.com/debian/        priority:1 release:stable
//! http://mirror2.example.com/debian/        priority:2
//! # comments and blank lines are ignored
//! ```
//!
//! Entries are filtered by their tags (`arch:`, `release:`/`suite:`/`codename:`,
//! `type:`, ...) and ordered by `priority` (lower first; entries without an
//! explicit priority come last), like apt.

use std::{collections::HashMap, io, path::PathBuf};

use reqwest_middleware::ClientWithMiddleware;
use snafu::{ResultExt, Snafu};

use crate::{reqwest::Method, send_request_with_url_and_method};

/// Where a mirror list lives and how to obtain it.
#[derive(Debug, Clone, PartialEq, Eq)]
enum MirrorListLocation {
    /// Fetched over the network.
    Http(String),
    /// A local file.
    File(PathBuf),
}

/// One entry of a mirror list file.
#[derive(Debug, Clone)]
pub struct MirrorEntry {
    /// Base URI of the mirror (e.g. `http://mirror.example.com/debian`).
    pub url: String,
    /// Lower is preferred; mirrors without an explicit `priority:` sort last.
    pub priority: u64,
    /// Extra tags from the entry (`arch:`, `release:`, `type:`, ...).
    pub tags: HashMap<Box<str>, Vec<Box<str>>>,
}

/// The transport used to reach a mirror base URI.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MirrorSourceType {
    Http,
    File,
}

/// A mirror resolved to a concrete base URI and transport.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResolvedMirror {
    /// Base URI with any trailing `/` stripped.
    pub url: String,
    pub source_type: MirrorSourceType,
}

#[derive(Debug, Snafu)]
pub enum MirrorError {
    #[snafu(display("Unsupported mirror protocol: {uri}"))]
    UnsupportedProtocol { uri: String },
    #[snafu(display("Failed to read mirror list {path:?}: {source}"))]
    ReadFile { path: PathBuf, source: io::Error },
    #[snafu(display("Failed to fetch mirror list {url}: {source}"))]
    Fetch {
        url: String,
        source: reqwest_middleware::Error,
    },
    #[snafu(display("Failed to read mirror list body from {url}: {source}"))]
    ReadBody { url: String, source: reqwest::Error },
    #[snafu(display("Mirror list {uri} is empty"))]
    EmptyList { uri: String },
}

/// Interpret a mirror URI into the location of its mirror list.
///
/// * `mirror://host/path`    → list fetched from `http://host/path` (like apt)
/// * `mirror+http(s)://...`  → list fetched from the given URL
/// * `mirror+file:///path`   → list is the local file `/path`
fn parse_mirror_uri(uri: &str) -> Result<MirrorListLocation, MirrorError> {
    let location = if let Some(rest) = uri.strip_prefix("mirror+http:") {
        MirrorListLocation::Http(format!("http:{rest}"))
    } else if let Some(rest) = uri.strip_prefix("mirror+https:") {
        MirrorListLocation::Http(format!("https:{rest}"))
    } else if let Some(rest) = uri.strip_prefix("mirror+file:") {
        let path = rest.strip_prefix("//").unwrap_or(rest);
        MirrorListLocation::File(PathBuf::from(path))
    } else if let Some(rest) = uri.strip_prefix("mirror:") {
        // Bare `mirror://` fetches the list over http, mirroring apt.
        MirrorListLocation::Http(format!("http:{rest}"))
    } else {
        return Err(MirrorError::UnsupportedProtocol {
            uri: uri.to_string(),
        });
    };
    Ok(location)
}

/// Parse a mirror list (apt's `MirrorListFileRecieved`).
///
/// Each line is either blank/comment or `<uri>[\t<tag> ...]`. When
/// `from_network` is set — the list itself was fetched over the network —
/// local `file:` mirrors are rejected, mirroring apt's security check
/// (CVE-2018-0501).
pub fn parse_mirror_list(text: &str, from_network: bool) -> Vec<MirrorEntry> {
    let mut entries = vec![];

    for line in text.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }

        let (url, tags) = match line.find('\t') {
            Some(tab) => (&line[..tab], &line[tab + 1..]),
            None => (line, ""),
        };
        let url = url.trim();
        if url.is_empty() {
            continue;
        }

        if from_network && url.starts_with("file:") {
            continue;
        }

        let mut entry = MirrorEntry {
            url: url.to_string(),
            priority: u64::MAX,
            tags: HashMap::new(),
        };

        for tag in tags.split_whitespace() {
            let Some((name, value)) = tag.split_once(':') else {
                continue;
            };
            if name.is_empty() {
                continue;
            }
            match name {
                "priority" => entry.priority = value.parse().unwrap_or(u64::MAX),
                "arch" => entry
                    .tags
                    .entry("arch".into())
                    .or_default()
                    .push(value.into()),
                "lang" => entry
                    .tags
                    .entry("lang".into())
                    .or_default()
                    .push(value.into()),
                // `suite`/`codename` are matched against the release, like apt.
                "suite" | "codename" | "release" => entry
                    .tags
                    .entry("release".into())
                    .or_default()
                    .push(value.into()),
                other => entry
                    .tags
                    .entry(other.to_ascii_lowercase().into())
                    .or_default()
                    .push(value.into()),
            }
        }

        entries.push(entry);
    }

    entries
}

/// Filter mirrors by the request's tags and order them by priority.
///
/// A mirror is kept when each of its tags matches the request attribute
/// (`arch`, `suite`/`codename`/`release`, `type: deb`/`deb-src`); missing
/// request attributes do not filter anything out.
pub fn select_mirrors(
    mut entries: Vec<MirrorEntry>,
    arch: Option<&str>,
    suite: Option<&str>,
    is_source: bool,
) -> Vec<ResolvedMirror> {
    fn tag_matches(
        tags: &HashMap<Box<str>, Vec<Box<str>>>,
        key: &str,
        value: Option<&str>,
    ) -> bool {
        match value {
            None => true,
            Some(value) => tags
                .get(key)
                .is_none_or(|values| values.iter().any(|x| x.as_ref() == value)),
        }
    }

    entries.retain(|e| {
        tag_matches(&e.tags, "arch", arch)
            && tag_matches(&e.tags, "release", suite)
            && tag_matches(
                &e.tags,
                "type",
                Some(if is_source { "deb-src" } else { "deb" }),
            )
    });

    entries.sort_by_key(|e| e.priority);

    entries
        .into_iter()
        .map(|e| ResolvedMirror {
            url: e.url.trim_end_matches('/').to_string(),
            source_type: if e.url.starts_with("file:") {
                MirrorSourceType::File
            } else {
                MirrorSourceType::Http
            },
        })
        .collect()
}

/// Fetch (or read) and parse a mirror list into its entries, without
/// applying any per-request filtering.
///
/// The parsed list can be cached and [`select_mirrors`] applied once per
/// source — the selection depends on the request's `suite` and
/// `deb`/`deb-src` type, so a list shared by several suites or by both
/// `deb` and `deb-src` must be filtered per source, not once.
pub async fn fetch_mirror_list(
    mirror_uri: &str,
    client: &ClientWithMiddleware,
) -> Result<Vec<MirrorEntry>, MirrorError> {
    let location = parse_mirror_uri(mirror_uri)?;

    let (text, from_network) = match location {
        MirrorListLocation::Http(url) => {
            let resp = send_request_with_url_and_method(&url, client, Method::GET)
                .await
                .context(FetchSnafu { url: url.clone() })?;
            let body = resp
                .text()
                .await
                .context(ReadBodySnafu { url: url.clone() })?;
            (body, true)
        }
        MirrorListLocation::File(path) => {
            let text = tokio::fs::read_to_string(&path)
                .await
                .context(ReadFileSnafu { path })?;
            (text, false)
        }
    };

    let entries = parse_mirror_list(&text, from_network);
    if entries.is_empty() {
        return Err(MirrorError::EmptyList {
            uri: mirror_uri.to_string(),
        });
    }

    Ok(entries)
}

/// Fetch (or read) a mirror list and return the resolved mirrors for one
/// request, filtered by `arch`/`suite`/`is_source` and in priority order.
pub async fn resolve_mirrors(
    mirror_uri: &str,
    client: &ClientWithMiddleware,
    arch: Option<&str>,
    suite: Option<&str>,
    is_source: bool,
) -> Result<Vec<ResolvedMirror>, MirrorError> {
    let entries = fetch_mirror_list(mirror_uri, client).await?;
    Ok(select_mirrors(entries, arch, suite, is_source))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_mirror_uri() {
        assert_eq!(
            parse_mirror_uri("mirror://mirrors.example.com/debian/").unwrap(),
            MirrorListLocation::Http("http://mirrors.example.com/debian/".into())
        );
        assert_eq!(
            parse_mirror_uri("mirror+http://host/list").unwrap(),
            MirrorListLocation::Http("http://host/list".into())
        );
        assert_eq!(
            parse_mirror_uri("mirror+https://host/list").unwrap(),
            MirrorListLocation::Http("https://host/list".into())
        );
        assert_eq!(
            parse_mirror_uri("mirror+file:///etc/apt/mirrors.list").unwrap(),
            MirrorListLocation::File(PathBuf::from("/etc/apt/mirrors.list"))
        );
        assert_eq!(
            parse_mirror_uri("mirror+file:/etc/apt/mirrors.list").unwrap(),
            MirrorListLocation::File(PathBuf::from("/etc/apt/mirrors.list"))
        );
        assert!(parse_mirror_uri("http://plain.example.com/").is_err());
        assert!(parse_mirror_uri("mirror+ftp://host/list").is_err());
    }

    #[test]
    fn test_parse_mirror_list() {
        // Tags are separated from the URI by a tab, like apt.
        let text = "\
# comment
http://mirror1.example.com/debian/\tpriority:1\trelease:stable
http://mirror2.example.com/debian/

http://mirror3.example.com/debian/\tpriority:2\tarch:amd64
file:///local/repo\tpriority:1
";
        let entries = parse_mirror_list(text, false);
        assert_eq!(entries.len(), 4);
        assert_eq!(entries[0].url, "http://mirror1.example.com/debian/");
        assert_eq!(entries[0].priority, 1);
        assert_eq!(entries[0].tags["release"], vec![Box::from("stable")]);
        assert_eq!(entries[1].priority, u64::MAX);
        assert_eq!(entries[2].priority, 2);
        assert_eq!(entries[2].tags["arch"], vec![Box::from("amd64")]);
        assert_eq!(entries[3].url, "file:///local/repo");
    }

    #[test]
    fn test_parse_mirror_list_network_rejects_file() {
        let text = "http://mirror1.example.com/\nfile:///local/repo\n";
        let entries = parse_mirror_list(text, true);
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].url, "http://mirror1.example.com/");
    }

    #[test]
    fn test_select_mirrors() {
        let text = "\
http://m1.example.com/\tpriority:2
http://m2.example.com/\tpriority:1
http://m3.example.com/\tpriority:1\trelease:stable
http://m4.example.com/\tpriority:1\trelease:unstable
file:///local/repo\tpriority:0
";
        let entries = parse_mirror_list(text, false);

        // suite filter + priority ordering (stable within equal priority)
        let selected = select_mirrors(entries.clone(), None, Some("stable"), false);
        assert_eq!(
            selected,
            vec![
                ResolvedMirror {
                    url: "file:///local/repo".into(),
                    source_type: MirrorSourceType::File,
                },
                ResolvedMirror {
                    url: "http://m2.example.com".into(),
                    source_type: MirrorSourceType::Http,
                },
                ResolvedMirror {
                    url: "http://m3.example.com".into(),
                    source_type: MirrorSourceType::Http,
                },
                ResolvedMirror {
                    url: "http://m1.example.com".into(),
                    source_type: MirrorSourceType::Http,
                },
            ]
        );

        // arch filter
        let text = "http://m1.example.com/\tarch:arm64\nhttp://m2.example.com/\n";
        let entries = parse_mirror_list(text, false);
        let selected = select_mirrors(entries, Some("amd64"), None, false);
        assert_eq!(selected.len(), 1);
        assert_eq!(selected[0].url, "http://m2.example.com");
    }
}
