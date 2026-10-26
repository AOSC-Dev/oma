//! Resolving apt's `mirror://` protocol.
//!
//! A mirror source does not name a repository itself: the URI points at a
//! *mirror list* whose lines name the actual repository base URIs. apt's
//! `methods/mirror.cc` treats this as a per-request redirect — every file is
//! matched against the metadata of the request (`Architecture`, `Component`,
//! `Release`, `Suite`, `Codename`, `Language`, `Type`) and the first usable
//! mirror is used, with the remaining matches kept as fallbacks.
//!
//! Accepted URI forms (apt 1.6+):
//!
//! ```text
//! mirror+file:/etc/apt/mirrors.list   local mirror list
//! mirror+http://host/mirrors.list     remote mirror list
//! mirror+https://host/mirrors.list    remote mirror list
//! mirror://host/mirrors.list          same as mirror+http:// (legacy form)
//! ```
//!
//! A mirror list is one entry per line:
//!
//! ```text
//! # comment
//! http://mirror1.example.com/debian/       priority:1 type:index
//! http://mirror2.example.com/debian/       arch:amd64 arch:all
//! file:/srv/local/debian/                  priority:2
//! ```
//!
//! The URI and its tags are separated by a tab; tags are separated by tabs or
//! spaces. `priority` sorts mirrors (lower first, untagged last); mirrors with
//! equal priority are shuffled so that a plain list such as Ubuntu's
//! `mirrors.txt` does not always hit its first entry. The remaining tags are
//! matched against [`MirrorTarget`]:
//!
//! * `arch`       → `Architecture`
//! * `component`  → `Component`
//! * `suite`      → `Suite` *and* `Release`
//! * `codename`   → `Codename` *and* `Release`
//! * `release`    → `Release`
//! * `lang`       → `Language`
//! * `type`       → `Type` (`index` for all index files, the file extension
//!   for package files, e.g. `deb`)
//!
//! A tag that is absent from a mirror entry never filters it out; a tag whose
//! values do not contain the target value does. This is the same rule as apt.
//!
//! Security: a mirror list fetched over the network may only contain
//! `http(s)` mirrors, never `file:` (apt's CVE-2018-0501 fix). A local list
//! may contain both. Unsupported transports (`ftp`, `copy`, nested mirror
//! transports) are skipped with a debug log.
//!
//! Mirror lists are capped at [`MAX_MIRROR_LIST_SIZE`], matching apt's
//! `MaximumSize`. Compressed lists (`.gz`, `.xz`, …) are recognised but not
//! decoded yet and produce [`MirrorError::UnsupportedCompression`].
//!
//! This module does not persist or share mirror lists between calls; callers
//! should cache a [`MirrorList`] by its source URI for the duration of one
//! refresh or one download transaction, and call [`MirrorList::resolve`] once
//! per file (the target metadata differs per file).

use std::{
    collections::{HashMap, hash_map::DefaultHasher, hash_map::RandomState},
    hash::{BuildHasher, Hash, Hasher},
    io,
    path::{Path, PathBuf},
};

use oma_logger::debug;
use reqwest::{Method, Url};
use reqwest_middleware::ClientWithMiddleware;
use snafu::{ResultExt, Snafu};

use crate::send_request_with_url_and_method;

/// Maximum accepted size of a mirror list, matching apt's `MaximumSize`.
pub const MAX_MIRROR_LIST_SIZE: u64 = 1024 * 1024;

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
        validate_http_uri(uri, &url)?;
        Ok(MirrorListLocation::Http(url))
    } else if let Some(rest) = uri.strip_prefix("mirror+http://") {
        let url = format!("http://{rest}");
        validate_http_uri(uri, &url)?;
        Ok(MirrorListLocation::Http(url))
    } else if let Some(rest) = uri.strip_prefix("mirror+https://") {
        let url = format!("https://{rest}");
        validate_http_uri(uri, &url)?;
        Ok(MirrorListLocation::Http(url))
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

        Ok(MirrorListLocation::File(path))
    } else {
        Err(MirrorError::UnsupportedScheme {
            uri: uri.to_string(),
        })
    }
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

/// Metadata apt attaches to an acquire request (`Target-*`), used to select
/// mirrors from a [`MirrorList`].
///
/// Every field is optional: a missing/empty field disables that matcher. Use
/// [`TargetKind::Index`] for every index file (including `InRelease`) and
/// [`TargetKind::File`] for package files, matching apt's `Target-Type`.
#[derive(Debug, Clone, Copy, Default)]
pub struct MirrorTarget<'a> {
    /// `Target-Architecture`, e.g. `amd64` (or `all` for `binary-all`).
    pub arch: Option<&'a str>,
    /// `Target-Component`, e.g. `main`.
    pub component: Option<&'a str>,
    /// `Target-Release`: the suite as written in sources.list.
    pub release: Option<&'a str>,
    /// `Target-Suite`: the `Suite` field of the Release file (package
    /// downloads only).
    pub suite: Option<&'a str>,
    /// `Target-Codename`: the `Codename` field of the Release file (package
    /// downloads only).
    pub codename: Option<&'a str>,
    /// `Target-Language`, e.g. `zh_CN` for translation files.
    pub language: Option<&'a str>,
    /// `Target-Type`.
    pub kind: TargetKind<'a>,
}

/// The `Target-Type` of a request.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum TargetKind<'a> {
    /// Do not filter by type.
    #[default]
    Unspecified,
    /// An index file (`InRelease`, `Packages.xz`, `Translation-zh_CN.xz`, …).
    Index,
    /// A package file; the value is its extension, e.g. `deb`.
    File(&'a str),
}

impl<'a> TargetKind<'a> {
    fn apt_type(self) -> Option<&'a str> {
        match self {
            Self::Unspecified => None,
            Self::Index => Some("index"),
            Self::File(extension) => Some(extension),
        }
    }
}

/// Transport used to reach a mirror.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MirrorTransport {
    Http,
    File,
}

/// One concrete download URL produced from a mirror list.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MirrorCandidate {
    /// Base URL of the mirror (for [`MirrorList::candidates`]) or the full
    /// download URL (for [`MirrorList::resolve`]).
    pub url: String,
    pub transport: MirrorTransport,
    /// The mirror's `priority:` tag; lower is tried first, `u64::MAX` when
    /// the entry has no explicit priority.
    pub priority: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct MirrorEntry {
    uri: String,
    priority: u64,
    /// Tag names are normalized like apt's `MirrorInfo`: `arch` becomes
    /// `Architecture`, `suite`/`codename` are also recorded under `Release`,
    /// and any other name is capitalized.
    tags: HashMap<String, Vec<String>>,
}

impl MirrorEntry {
    fn matches(&self, target: &MirrorTarget<'_>) -> bool {
        tag_matches(self, "Architecture", target.arch)
            && tag_matches(self, "Component", target.component)
            && tag_matches(self, "Release", target.release)
            && tag_matches(self, "Suite", target.suite)
            && tag_matches(self, "Codename", target.codename)
            && tag_matches(self, "Language", target.language)
            && tag_matches(self, "Type", target.kind.apt_type())
    }
}

fn tag_matches(entry: &MirrorEntry, key: &str, value: Option<&str>) -> bool {
    let Some(value) = value.filter(|value| !value.is_empty()) else {
        return true;
    };

    match entry.tags.get(key) {
        // A mirror without the tag is usable for every request.
        None => true,
        Some(values) => values.iter().any(|tag| tag == value),
    }
}

/// A parsed apt mirror list, ready to be resolved against request metadata.
#[derive(Debug, Clone)]
pub struct MirrorList {
    uri: String,
    entries: Vec<MirrorEntry>,
    /// Per-list seed for the random tie-break between equal priorities.
    seed: u64,
}

impl MirrorList {
    /// Parse mirror list text.
    ///
    /// `from_network` must be `true` when the text was fetched over HTTP(S):
    /// local `file:` mirrors are then skipped, like apt does for remote
    /// lists.
    pub fn parse(uri: &str, text: &str, from_network: bool) -> Result<Self, MirrorError> {
        let raw = parse_entries(text);

        if raw.is_empty() {
            return Err(MirrorError::EmptyList {
                uri: uri.to_string(),
            });
        }

        let total = raw.len();
        let mut entries = Vec::with_capacity(total);

        for entry in raw {
            if entry_supported(&entry, from_network) {
                entries.push(entry);
            } else {
                debug!("Skipping unsupported mirror entry {} in {uri}", entry.uri);
            }
        }

        if entries.is_empty() {
            return Err(MirrorError::NoSupportedMirrors {
                uri: uri.to_string(),
            });
        }

        debug!(
            "Parsed {} mirrors from {uri} ({} entries skipped)",
            entries.len(),
            total - entries.len()
        );

        Ok(Self {
            uri: uri.to_string(),
            entries,
            seed: random_seed(),
        })
    }

    /// Fetch or read a mirror list.
    ///
    /// The list is not cached; callers should keep the returned value for the
    /// duration of one operation and reuse it for every file.
    pub async fn load(uri: &str, client: &ClientWithMiddleware) -> Result<Self, MirrorError> {
        let location = parse_mirror_uri(uri)?;

        if let Some(extension) = compressed_extension(&location) {
            return Err(MirrorError::UnsupportedCompression {
                uri: uri.to_string(),
                extension,
            });
        }

        let text = match &location {
            MirrorListLocation::Http(url) => fetch_text(uri, url, client).await?,
            MirrorListLocation::File(path) => read_text(uri, path).await?,
        };

        Self::parse(uri, &text, !location.is_local())
    }

    /// The mirror URI this list was loaded from.
    pub fn uri(&self) -> &str {
        &self.uri
    }

    /// Number of usable mirror entries.
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// Selected mirror base URLs in try order (before appending a request
    /// path). Prefer [`Self::resolve`] for download URLs.
    pub fn candidates(
        &self,
        target: &MirrorTarget<'_>,
    ) -> Result<Vec<MirrorCandidate>, MirrorError> {
        let selected = self.select(target)?;

        Ok(selected
            .into_iter()
            .filter_map(|entry| {
                transport_of(&entry.uri).map(|transport| MirrorCandidate {
                    url: entry.uri.clone(),
                    transport,
                    priority: entry.priority,
                })
            })
            .collect())
    }

    /// Expand a file URL of a mirror source into one download URL per
    /// selected mirror, in try order.
    ///
    /// `source_uri` is the mirror source as written in sources.list (the
    /// `mirror+...` URI, with or without a trailing `/`). `download_url` is
    /// the full URL that would have been used without the mirror protocol,
    /// e.g. `mirror+file:/etc/apt/mirrors.list/dists/bookworm/InRelease`.
    /// The path after `source_uri` is appended to every selected mirror.
    ///
    /// The URL is joined by string concatenation, not `Url::join`, so a
    /// base path is preserved and percent-encoding is not reinterpreted.
    pub fn resolve(
        &self,
        source_uri: &str,
        download_url: &str,
        target: &MirrorTarget<'_>,
    ) -> Result<Vec<MirrorCandidate>, MirrorError> {
        let source_base = source_uri.trim_end_matches('/');
        let suffix =
            download_url
                .strip_prefix(source_base)
                .ok_or_else(|| MirrorError::UrlMismatch {
                    source_uri: source_uri.to_string(),
                    url: download_url.to_string(),
                })?;

        // `source_base` must be a path prefix, not a random string prefix
        // (`/x` must not match `/xy`).
        if !suffix.is_empty() && !suffix.starts_with('/') {
            return Err(MirrorError::UrlMismatch {
                source_uri: source_uri.to_string(),
                url: download_url.to_string(),
            });
        }

        let path = suffix.trim_start_matches('/');
        let selected = self.select(target)?;
        let mut candidates = Vec::with_capacity(selected.len());

        for entry in selected {
            let Some(transport) = transport_of(&entry.uri) else {
                debug!("Skipping unsupported mirror entry {}", entry.uri);
                continue;
            };

            let url = if path.is_empty() {
                entry.uri.clone()
            } else {
                format!("{}/{}", entry.uri.trim_end_matches('/'), path)
            };

            candidates.push(MirrorCandidate {
                url,
                transport,
                priority: entry.priority,
            });
        }

        if candidates.is_empty() {
            return Err(MirrorError::NoSupportedMirrors {
                uri: self.uri.clone(),
            });
        }

        Ok(candidates)
    }

    fn select<'a>(
        &'a self,
        target: &MirrorTarget<'_>,
    ) -> Result<Vec<&'a MirrorEntry>, MirrorError> {
        let mut mirrors: Vec<&MirrorEntry> = self
            .entries
            .iter()
            .filter(|entry| entry.matches(target))
            .collect();

        if mirrors.is_empty() {
            debug!("No mirror in {} matches target {target:?}", self.uri);
            return Err(MirrorError::NoCandidates {
                uri: self.uri.clone(),
            });
        }

        // Lower priority first; equal priorities use the per-list seed so a
        // plain list is spread across mirrors instead of always trying the
        // first entry. The sort is stable, so duplicate URIs keep their order.
        mirrors.sort_by(|a, b| {
            a.priority
                .cmp(&b.priority)
                .then_with(|| tie_hash(self.seed, &a.uri).cmp(&tie_hash(self.seed, &b.uri)))
        });

        Ok(mirrors)
    }
}

fn parse_entries(text: &str) -> Vec<MirrorEntry> {
    let mut entries = Vec::new();

    for line in text.lines() {
        let line = line.trim();

        if line.is_empty() || line.starts_with('#') {
            continue;
        }

        let (uri, tags) = match line.split_once('\t') {
            Some((uri, tags)) => (uri.trim(), tags),
            None => (line, ""),
        };

        if uri.is_empty() {
            continue;
        }

        let mut entry = MirrorEntry {
            uri: uri.to_string(),
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
                "priority" => {
                    entry.priority = value.parse().unwrap_or_else(|_| {
                        debug!("Ignoring invalid mirror priority {value:?} for {uri}");
                        u64::MAX
                    });
                }
                "arch" => push_tag(&mut entry.tags, "Architecture", value),
                "lang" => push_tag(&mut entry.tags, "Language", value),
                "suite" => {
                    push_tag(&mut entry.tags, "Suite", value);
                    // apt records suite/codename under Release as well, so a
                    // mirror tagged either way can serve index requests
                    // (which only carry Target-Release).
                    push_tag(&mut entry.tags, "Release", value);
                }
                "codename" => {
                    push_tag(&mut entry.tags, "Codename", value);
                    push_tag(&mut entry.tags, "Release", value);
                }
                _ => {
                    let mut key = name.to_string();
                    if let Some(first) = key.get_mut(0..1) {
                        first.make_ascii_uppercase();
                    }
                    push_tag(&mut entry.tags, &key, value);
                }
            }
        }

        entries.push(entry);
    }

    entries
}

fn push_tag(tags: &mut HashMap<String, Vec<String>>, key: &str, value: &str) {
    tags.entry(key.to_string())
        .or_default()
        .push(value.to_string());
}

fn entry_supported(entry: &MirrorEntry, from_network: bool) -> bool {
    let Some((scheme, _)) = entry.uri.split_once(':') else {
        return false;
    };

    if scheme.eq_ignore_ascii_case("http") || scheme.eq_ignore_ascii_case("https") {
        true
    } else if scheme.eq_ignore_ascii_case("file") {
        // A network-fetched list must not point at local files
        // (CVE-2018-0501).
        !from_network
    } else {
        false
    }
}

fn transport_of(uri: &str) -> Option<MirrorTransport> {
    let (scheme, _) = uri.split_once(':')?;

    if scheme.eq_ignore_ascii_case("http") || scheme.eq_ignore_ascii_case("https") {
        Some(MirrorTransport::Http)
    } else if scheme.eq_ignore_ascii_case("file") {
        Some(MirrorTransport::File)
    } else {
        None
    }
}

fn random_seed() -> u64 {
    RandomState::new().build_hasher().finish()
}

fn tie_hash(seed: u64, uri: &str) -> u64 {
    let mut hasher = DefaultHasher::new();
    seed.hash(&mut hasher);
    uri.hash(&mut hasher);
    hasher.finish()
}

fn compressed_extension(location: &MirrorListLocation) -> Option<String> {
    let path = match location {
        MirrorListLocation::Http(url) => {
            let parsed = Url::parse(url).ok()?;
            Path::new(parsed.path()).to_path_buf()
        }
        MirrorListLocation::File(path) => path.clone(),
    };

    let extension = path.extension()?.to_str()?.to_ascii_lowercase();

    matches!(
        extension.as_str(),
        "gz" | "xz" | "bz2" | "zst" | "zstd" | "lz4" | "lzma"
    )
    .then_some(extension)
}

async fn fetch_text(
    uri: &str,
    url: &str,
    client: &ClientWithMiddleware,
) -> Result<String, MirrorError> {
    let mut resp = send_request_with_url_and_method(url, client, Method::GET)
        .await
        .context(FetchSnafu {
            url: url.to_string(),
        })?;

    if let Some(len) = resp.content_length()
        && len > MAX_MIRROR_LIST_SIZE
    {
        return Err(MirrorError::TooLarge {
            uri: uri.to_string(),
            limit: MAX_MIRROR_LIST_SIZE,
        });
    }

    let mut body = Vec::new();

    while let Some(chunk) = resp.chunk().await.context(ReadBodySnafu {
        url: url.to_string(),
    })? {
        if body.len() as u64 + chunk.len() as u64 > MAX_MIRROR_LIST_SIZE {
            return Err(MirrorError::TooLarge {
                uri: uri.to_string(),
                limit: MAX_MIRROR_LIST_SIZE,
            });
        }

        body.extend_from_slice(&chunk);
    }

    String::from_utf8(body).map_err(|_| MirrorError::InvalidEncoding {
        uri: uri.to_string(),
    })
}

async fn read_text(uri: &str, path: &Path) -> Result<String, MirrorError> {
    let metadata = tokio::fs::metadata(path).await.context(ReadFileSnafu {
        path: path.to_path_buf(),
    })?;

    if metadata.len() > MAX_MIRROR_LIST_SIZE {
        return Err(MirrorError::TooLarge {
            uri: uri.to_string(),
            limit: MAX_MIRROR_LIST_SIZE,
        });
    }

    let bytes = tokio::fs::read(path).await.context(ReadFileSnafu {
        path: path.to_path_buf(),
    })?;

    if bytes.len() as u64 > MAX_MIRROR_LIST_SIZE {
        return Err(MirrorError::TooLarge {
            uri: uri.to_string(),
            limit: MAX_MIRROR_LIST_SIZE,
        });
    }

    String::from_utf8(bytes).map_err(|_| MirrorError::InvalidEncoding {
        uri: uri.to_string(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    const SOURCE: &str = "mirror+file:/etc/apt/mirrors.list";
    const INRELEASE: &str = "mirror+file:/etc/apt/mirrors.list/InRelease";

    fn list(text: &str) -> MirrorList {
        MirrorList::parse(SOURCE, text, false).unwrap()
    }

    fn seeded_list(text: &str, seed: u64) -> MirrorList {
        let mut list = list(text);
        list.seed = seed;
        list
    }

    fn urls(list: &MirrorList, target: &MirrorTarget<'_>) -> Vec<String> {
        list.resolve(SOURCE, INRELEASE, target)
            .unwrap()
            .into_iter()
            .map(|candidate| candidate.url)
            .collect()
    }

    fn client() -> ClientWithMiddleware {
        rustls::crypto::ring::default_provider()
            .install_default()
            .ok();

        reqwest::ClientBuilder::new().build().unwrap().into()
    }

    fn temp_path(name: &str) -> PathBuf {
        let stamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();

        std::env::temp_dir().join(format!(
            "oma-fetch-mirror-{}-{stamp}-{name}",
            std::process::id()
        ))
    }

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
        assert!(parse_mirror_uri("mirror+https://").is_err());
        assert!(parse_mirror_uri("mirror://").is_err());
        assert!(parse_mirror_uri("mirror+ftp://host/list").is_err());
        assert!(parse_mirror_uri("mirror+copy:/list").is_err());
        assert!(parse_mirror_uri("mirror+file:relative").is_err());
        assert!(parse_mirror_uri("http://host/list").is_err());
    }

    #[test]
    fn parses_entry_tags_like_apt() {
        let list = list(
            "# comment\n\
             http://m1.example.com/debian/\tpriority:1\tarch:amd64\tsuite:bookworm\ttype:index\n\
             http://m2.example.com/debian/\tcodename:trixie\tcomponent:main\tlang:zh_CN\n\
             file:/srv/repo\n",
        );

        assert_eq!(list.len(), 3);

        let m1 = &list.entries[0];
        assert_eq!(m1.priority, 1);
        assert_eq!(m1.tags["Architecture"], vec!["amd64".to_string()]);
        assert_eq!(m1.tags["Suite"], vec!["bookworm".to_string()]);
        assert_eq!(m1.tags["Release"], vec!["bookworm".to_string()]);
        assert_eq!(m1.tags["Type"], vec!["index".to_string()]);

        let m2 = &list.entries[1];
        assert_eq!(m2.priority, u64::MAX);
        assert_eq!(m2.tags["Codename"], vec!["trixie".to_string()]);
        assert_eq!(m2.tags["Release"], vec!["trixie".to_string()]);
        assert_eq!(m2.tags["Component"], vec!["main".to_string()]);
        assert_eq!(m2.tags["Language"], vec!["zh_CN".to_string()]);

        assert_eq!(
            transport_of(&list.entries[2].uri),
            Some(MirrorTransport::File)
        );
    }

    #[test]
    fn remote_lists_reject_local_mirrors() {
        let text = "http://m1.example.com/\nfile:/srv/repo\n";
        let list = MirrorList::parse("mirror+https://host/list", text, true).unwrap();
        assert_eq!(list.len(), 1);
        assert_eq!(list.entries[0].uri, "http://m1.example.com/");

        let err = MirrorList::parse("mirror+https://host/list", "file:/srv/repo\n", true)
            .expect_err("remote list with only file mirrors must fail");
        assert!(matches!(err, MirrorError::NoSupportedMirrors { .. }));
    }

    #[test]
    fn local_lists_allow_local_mirrors() {
        let list = list("file:/srv/repo\n");
        assert_eq!(list.len(), 1);
        assert_eq!(
            transport_of(&list.entries[0].uri),
            Some(MirrorTransport::File)
        );
    }

    #[test]
    fn skips_unsupported_transports() {
        let list = list(
            "ftp://m1.example.com/\ncopy:/m2/\nmirror+http://m3/list\nhttp://ok.example.com/\n",
        );

        assert_eq!(list.len(), 1);
        assert_eq!(list.entries[0].uri, "http://ok.example.com/");
    }

    #[test]
    fn rejects_empty_lists() {
        let err = MirrorList::parse(SOURCE, "# only comments\n\n", false)
            .expect_err("an empty mirror list must be rejected");
        assert!(matches!(err, MirrorError::EmptyList { .. }));
    }

    #[test]
    fn filters_by_arch_component_language_and_type() {
        let list = seeded_list(
            "\
             http://plain.example.com/\n\
             http://amd.example.com/\tarch:amd64\n\
             http://arm.example.com/\tarch:arm64\n\
             http://index.example.com/\ttype:index\n\
             http://deb.example.com/\ttype:deb\n\
             http://main.example.com/\tcomponent:main\n\
             http://contrib.example.com/\tcomponent:contrib\n\
             http://zh.example.com/\tlang:zh_CN\n\
             http://en.example.com/\tlang:en\n",
            1,
        );

        // Package-style target: architecture + component + language + type.
        let package_target = MirrorTarget {
            arch: Some("amd64"),
            component: Some("main"),
            language: Some("zh_CN"),
            kind: TargetKind::File("deb"),
            ..Default::default()
        };
        let selected = urls(&list, &package_target);

        // Untagged mirrors serve every request.
        assert!(selected.contains(&"http://plain.example.com/InRelease".to_string()));
        // arch:amd64 matches, arch:arm64 does not.
        assert!(selected.contains(&"http://amd.example.com/InRelease".to_string()));
        assert!(!selected.contains(&"http://arm.example.com/InRelease".to_string()));
        // type:deb matches a package request, type:index does not.
        assert!(selected.contains(&"http://deb.example.com/InRelease".to_string()));
        assert!(!selected.contains(&"http://index.example.com/InRelease".to_string()));
        // component:main matches, component:contrib does not.
        assert!(selected.contains(&"http://main.example.com/InRelease".to_string()));
        assert!(!selected.contains(&"http://contrib.example.com/InRelease".to_string()));
        // lang:zh_CN matches, lang:en does not.
        assert!(selected.contains(&"http://zh.example.com/InRelease".to_string()));
        assert!(!selected.contains(&"http://en.example.com/InRelease".to_string()));

        // Index-style target: type:index and no component/language filter.
        let index_target = MirrorTarget {
            arch: Some("amd64"),
            kind: TargetKind::Index,
            ..Default::default()
        };
        let selected = urls(&list, &index_target);

        assert!(selected.contains(&"http://index.example.com/InRelease".to_string()));
        assert!(!selected.contains(&"http://deb.example.com/InRelease".to_string()));
        assert!(selected.contains(&"http://main.example.com/InRelease".to_string()));
        assert!(selected.contains(&"http://contrib.example.com/InRelease".to_string()));
        assert!(selected.contains(&"http://zh.example.com/InRelease".to_string()));
        assert!(selected.contains(&"http://en.example.com/InRelease".to_string()));

        // Unspecified type does not filter type-tagged entries.
        let unspecified = MirrorTarget {
            kind: TargetKind::Unspecified,
            ..Default::default()
        };
        let selected = urls(&list, &unspecified);
        assert!(selected.contains(&"http://index.example.com/InRelease".to_string()));
        assert!(selected.contains(&"http://deb.example.com/InRelease".to_string()));
    }

    #[test]
    fn matches_suite_and_codename_like_apt() {
        let list = seeded_list(
            "\
             http://suite.example.com/\tsuite:bookworm\n\
             http://suite-other.example.com/\tsuite:trixie\n\
             http://codename.example.com/\tcodename:bookworm\n\
             http://codename-other.example.com/\tcodename:trixie\n\
             http://release.example.com/\trelease:bookworm\n",
            1,
        );

        // Target-Release (sources.list suite) matches every spelling of the
        // release, because suite/codename tags are also recorded under
        // Release. Conflicting values are excluded.
        let release = MirrorTarget {
            release: Some("bookworm"),
            ..Default::default()
        };
        let selected = urls(&list, &release);
        assert_eq!(selected.len(), 3);
        assert!(selected.contains(&"http://suite.example.com/InRelease".to_string()));
        assert!(selected.contains(&"http://codename.example.com/InRelease".to_string()));
        assert!(selected.contains(&"http://release.example.com/InRelease".to_string()));
        assert!(!selected.contains(&"http://suite-other.example.com/InRelease".to_string()));
        assert!(!selected.contains(&"http://codename-other.example.com/InRelease".to_string()));

        // Target-Suite excludes suite:trixie, but entries without a Suite
        // tag still match (a missing tag never filters a mirror out).
        let suite = MirrorTarget {
            suite: Some("bookworm"),
            ..Default::default()
        };
        let selected = urls(&list, &suite);
        assert!(selected.contains(&"http://suite.example.com/InRelease".to_string()));
        assert!(!selected.contains(&"http://suite-other.example.com/InRelease".to_string()));
        assert!(selected.contains(&"http://codename.example.com/InRelease".to_string()));
        assert!(selected.contains(&"http://release.example.com/InRelease".to_string()));

        // Target-Codename behaves the same way for codename: tags.
        let codename = MirrorTarget {
            codename: Some("bookworm"),
            ..Default::default()
        };
        let selected = urls(&list, &codename);
        assert!(selected.contains(&"http://codename.example.com/InRelease".to_string()));
        assert!(!selected.contains(&"http://codename-other.example.com/InRelease".to_string()));
        assert!(selected.contains(&"http://suite.example.com/InRelease".to_string()));
        assert!(selected.contains(&"http://release.example.com/InRelease".to_string()));

        // A target value that no tag carries filters the entry out.
        let other = MirrorTarget {
            release: Some("unstable"),
            ..Default::default()
        };
        assert!(list.resolve(SOURCE, INRELEASE, &other).is_err());
    }

    #[test]
    fn orders_by_priority_and_keeps_untagged_last() {
        let list = seeded_list(
            "\
             http://p2.example.com/\tpriority:2\n\
             http://p1.example.com/\tpriority:1\n\
             http://p9.example.com/\tpriority:9\n\
             http://untagged.example.com/\n",
            7,
        );

        let selected = list
            .resolve(SOURCE, INRELEASE, &MirrorTarget::default())
            .unwrap();

        assert_eq!(selected[0].url, "http://p1.example.com/InRelease");
        assert_eq!(selected[1].url, "http://p2.example.com/InRelease");
        assert_eq!(selected[2].url, "http://p9.example.com/InRelease");
        assert_eq!(selected[3].url, "http://untagged.example.com/InRelease");
        assert_eq!(selected[0].priority, 1);
        assert_eq!(selected[3].priority, u64::MAX);
    }

    #[test]
    fn equal_priority_order_is_stable_within_one_list() {
        let text = "http://a.example.com/\nhttp://b.example.com/\nhttp://c.example.com/\n";

        let first = seeded_list(text, 42)
            .resolve(SOURCE, INRELEASE, &MirrorTarget::default())
            .unwrap();
        let second = seeded_list(text, 42)
            .resolve(SOURCE, INRELEASE, &MirrorTarget::default())
            .unwrap();

        assert_eq!(first, second);
        assert_eq!(first.len(), 3);
    }

    #[test]
    fn errors_when_no_mirror_matches() {
        let list = list("http://amd.example.com/\tarch:amd64\n");
        let target = MirrorTarget {
            arch: Some("arm64"),
            ..Default::default()
        };

        let err = list
            .resolve(SOURCE, INRELEASE, &target)
            .expect_err("no matching mirror must be an error");
        assert!(matches!(err, MirrorError::NoCandidates { .. }));
    }

    #[test]
    fn resolves_urls_against_mirror_bases() {
        let list = seeded_list(
            "\
             http://m1.example.com/debian/\tpriority:1\n\
             file:/srv/repo\tpriority:2\n",
            1,
        );
        let target = MirrorTarget {
            arch: Some("amd64"),
            kind: TargetKind::Index,
            ..Default::default()
        };
        let url = "mirror+file:/etc/apt/mirrors.list/dists/bookworm/main/binary-amd64/Packages.xz";

        let selected = list.resolve(SOURCE, url, &target).unwrap();

        assert_eq!(
            selected[0],
            MirrorCandidate {
                url: "http://m1.example.com/debian/dists/bookworm/main/binary-amd64/Packages.xz"
                    .into(),
                transport: MirrorTransport::Http,
                priority: 1,
            }
        );
        assert_eq!(
            selected[1],
            MirrorCandidate {
                url: "file:/srv/repo/dists/bookworm/main/binary-amd64/Packages.xz".into(),
                transport: MirrorTransport::File,
                priority: 2,
            }
        );
    }

    #[test]
    fn resolve_accepts_trailing_slashes() {
        let list = list("http://m1.example.com/debian/\n");
        let source = "mirror+file:/etc/apt/mirrors.list/";
        let url = "mirror+file:/etc/apt/mirrors.list/dists/bookworm/InRelease";

        let selected = list.resolve(source, url, &MirrorTarget::default()).unwrap();

        assert_eq!(
            selected[0].url,
            "http://m1.example.com/debian/dists/bookworm/InRelease"
        );
    }

    #[test]
    fn rejects_urls_outside_the_source() {
        let list = list("http://m1.example.com/debian/\n");

        let err = list
            .resolve(
                "mirror+file:/etc/apt/mirrors.list",
                "mirror+file:/etc/apt/mirrors.list-other/InRelease",
                &MirrorTarget::default(),
            )
            .expect_err("a non-boundary prefix must not match");
        assert!(matches!(err, MirrorError::UrlMismatch { .. }));

        let err = list
            .resolve(
                "mirror+file:/etc/apt/mirrors.list",
                "mirror+file:/another/list/InRelease",
                &MirrorTarget::default(),
            )
            .expect_err("an unrelated URL must not match");
        assert!(matches!(err, MirrorError::UrlMismatch { .. }));
    }

    #[test]
    fn candidates_returns_mirror_bases() {
        let list = list("http://m1.example.com/debian/\n");

        let candidates = list.candidates(&MirrorTarget::default()).unwrap();

        assert_eq!(candidates.len(), 1);
        assert_eq!(candidates[0].url, "http://m1.example.com/debian/");
        assert_eq!(candidates[0].transport, MirrorTransport::Http);
    }

    #[tokio::test]
    async fn loads_a_local_list() {
        let path = temp_path("mirrors.list");
        tokio::fs::write(&path, "http://m1.example.com/\nfile:/srv/repo\n")
            .await
            .unwrap();
        let uri = format!("mirror+file:{}", path.display());

        let list = MirrorList::load(&uri, &client()).await.unwrap();

        assert_eq!(list.uri(), uri);
        assert_eq!(list.len(), 2);

        let _ = tokio::fs::remove_file(&path).await;
    }

    #[tokio::test]
    async fn rejects_compressed_local_lists() {
        let path = temp_path("mirrors.list.gz");
        let uri = format!("mirror+file:{}", path.display());

        let err = MirrorList::load(&uri, &client()).await.unwrap_err();

        assert!(matches!(err, MirrorError::UnsupportedCompression { .. }));
    }

    #[tokio::test]
    async fn rejects_oversized_local_lists() {
        let path = temp_path("big.list");
        tokio::fs::write(&path, vec![b'#'; (MAX_MIRROR_LIST_SIZE + 1) as usize])
            .await
            .unwrap();
        let uri = format!("mirror+file:{}", path.display());

        let err = MirrorList::load(&uri, &client()).await.unwrap_err();

        assert!(matches!(err, MirrorError::TooLarge { .. }));

        let _ = tokio::fs::remove_file(&path).await;
    }

    #[tokio::test]
    async fn rejects_non_utf8_local_lists() {
        let path = temp_path("invalid.list");
        tokio::fs::write(&path, [0xff, 0xfe]).await.unwrap();
        let uri = format!("mirror+file:{}", path.display());

        let err = MirrorList::load(&uri, &client()).await.unwrap_err();

        assert!(matches!(err, MirrorError::InvalidEncoding { .. }));

        let _ = tokio::fs::remove_file(&path).await;
    }
}
