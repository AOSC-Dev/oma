use std::collections::hash_map::{DefaultHasher, RandomState};
use std::hash::{BuildHasher, Hash, Hasher};

use oma_logger::debug;
use reqwest_middleware::ClientWithMiddleware;

use super::entry::{MirrorEntry, entry_supported, parse_entries, transport_of};
use super::error::MirrorError;
use super::load::{compressed_extension, fetch_text, read_text};
use super::target::{MirrorCandidate, MirrorTarget};
use super::uri::{MirrorListLocation, parse_mirror_uri};

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

fn random_seed() -> u64 {
    RandomState::new().build_hasher().finish()
}

fn tie_hash(seed: u64, uri: &str) -> u64 {
    let mut hasher = DefaultHasher::new();
    seed.hash(&mut hasher);
    uri.hash(&mut hasher);
    hasher.finish()
}

#[cfg(test)]
mod tests;
