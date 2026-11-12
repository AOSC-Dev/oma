//! Load apt sources.list entries with a process-wide cache.

use std::{
    collections::HashMap,
    io,
    path::{Path, PathBuf},
    sync::{Arc, Mutex},
    time::SystemTime,
};

use oma_apt_sources_lists::{SourceLine, SourceListType, SourcesList};
use once_cell::sync::OnceCell;

pub use oma_apt_sources_lists::SourceEntry;

#[derive(Debug)]
pub struct ConfiguredSources {
    entries: Vec<SourceEntry>,
}

impl ConfiguredSources {
    /// Parse enabled entries from the given source-list paths.
    ///
    /// `on_unknown_file` is called for files that cannot be parsed as an
    /// `apt` sources list, matching apt's `UnknownFile` handling.
    pub fn from_paths<G>(paths: &[PathBuf], mut on_unknown_file: G) -> Self
    where
        G: FnMut(&Path),
    {
        let mut entries = Vec::new();

        for path in paths {
            let Ok(list) = SourcesList::new(path) else {
                on_unknown_file(path);
                continue;
            };

            let parsed: Box<dyn Iterator<Item = SourceEntry>> = match list.entries {
                SourceListType::SourceLine(lines) => {
                    Box::new(lines.0.into_iter().filter_map(|line| {
                        if let SourceLine::Entry(entry) = line {
                            Some(entry)
                        } else {
                            None
                        }
                    }))
                }
                SourceListType::Deb822(deb822) => Box::new(deb822.entries.into_iter()),
            };

            entries.extend(parsed.filter(|entry| entry.enabled));
        }

        Self::from_entries(entries)
    }

    fn from_entries(entries: Vec<SourceEntry>) -> Self {
        Self { entries }
    }

    pub fn entries(&self) -> &[SourceEntry] {
        &self.entries
    }

    /// Return the configured suite for a repository URL.
    ///
    /// A URL may be configured with several suites. In that case the
    /// Release `Archive`/`Codename` are used to disambiguate; if they do not
    /// identify exactly one suite, no configured suite is returned.
    pub fn suite_for(
        &self,
        url: &str,
        archive: Option<&str>,
        codename: Option<&str>,
    ) -> Option<&str> {
        let url = normalize_url(url);
        let mut suites = Vec::new();

        for entry in &self.entries {
            if normalize_url(&entry.url) == url
                && !entry.suite.is_empty()
                && !suites.contains(&entry.suite.as_str())
            {
                suites.push(entry.suite.as_str());
            }
        }

        if let [suite] = suites.as_slice() {
            return Some(suite);
        }

        let mut matches = suites
            .iter()
            .copied()
            .filter(|suite| Some(*suite) == archive || Some(*suite) == codename);
        let first = matches.next()?;

        matches.next().is_none().then_some(first)
    }
}

fn normalize_url(url: &str) -> &str {
    url.trim_end_matches('/')
}

#[derive(Clone, Debug, Hash, PartialEq, Eq)]
struct SourcePaths {
    list_file: String,
    list_dir: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct SourceFingerprint {
    path: PathBuf,
    modified: SystemTime,
    len: u64,
}

struct CachedSources {
    fingerprint: Vec<SourceFingerprint>,
    sources: Arc<ConfiguredSources>,
}

static CACHE: OnceCell<Mutex<HashMap<SourcePaths, CachedSources>>> = OnceCell::new();

fn source_fingerprint(paths: &[PathBuf]) -> io::Result<Vec<SourceFingerprint>> {
    paths
        .iter()
        .map(|path| {
            let metadata = std::fs::metadata(path)?;
            Ok(SourceFingerprint {
                path: path.clone(),
                modified: metadata.modified()?,
                len: metadata.len(),
            })
        })
        .collect()
}

/// Scan the configured sources.list paths.
///
/// Like APT's `GetListOfFilesInDir`, this includes the configured
/// `sourcelist` file and every regular file under `sourceparts`.
pub fn scan_sources_list_paths(
    list_file: impl AsRef<str>,
    list_dir: impl AsRef<str>,
) -> Vec<PathBuf> {
    let mut paths = Vec::new();
    let default = Path::new(list_file.as_ref());
    let list_dir_path = Path::new(list_dir.as_ref());

    if default.exists() {
        paths.push(default.to_path_buf());
    }

    if list_dir_path.exists()
        && let Ok(dir) = std::fs::read_dir(list_dir_path)
    {
        for entry in dir.flatten() {
            let path = entry.path();
            if path.is_file() {
                paths.push(path);
            }
        }
    }

    paths
}

/// Load and cache the configured sources for a `sourcelist`/`sourceparts`
/// pair.
pub fn configured_sources(
    list_file: impl Into<String>,
    list_dir: impl Into<String>,
) -> Arc<ConfiguredSources> {
    let key = SourcePaths {
        list_file: list_file.into(),
        list_dir: list_dir.into(),
    };
    let cache = CACHE.get_or_init(|| Mutex::new(HashMap::new()));
    let mut cache = cache
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());

    let paths = scan_sources_list_paths(&key.list_file, &key.list_dir);
    let fingerprint = source_fingerprint(&paths);

    if let Ok(fingerprint) = &fingerprint
        && let Some(cached) = cache.get(&key)
        && cached.fingerprint == *fingerprint
    {
        return Arc::clone(&cached.sources);
    }

    let sources = Arc::new(ConfiguredSources::from_paths(&paths, |_| {}));

    // If any source file cannot be fingerprinted, do not cache the result:
    // the next call must try the files again instead of serving stale data.
    if let Ok(fingerprint) = fingerprint {
        cache.insert(
            key,
            CachedSources {
                fingerprint,
                sources: Arc::clone(&sources),
            },
        );
    } else {
        cache.remove(&key);
    }

    sources
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn temp_dir() -> PathBuf {
        std::env::temp_dir().join(format!(
            "oma-apt-sources-{}-{}",
            std::process::id(),
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ))
    }

    #[test]
    fn loads_caches_and_queries_entries() {
        let dir = temp_dir();
        std::fs::create_dir_all(&dir).unwrap();
        let list = dir.join("sources.list");
        std::fs::write(
            &list,
            "deb http://example.com/debian bookworm main\ndeb-src http://example.com/debian bookworm main\n",
        )
        .unwrap();

        let path = list.to_string_lossy().to_string();
        let missing_dir = dir.join("missing").to_string_lossy().to_string();
        let first = configured_sources(path.clone(), missing_dir.clone());
        assert_eq!(first.entries().len(), 2);
        assert_eq!(
            first.suite_for("http://example.com/debian", None, None),
            Some("bookworm")
        );

        let cached = configured_sources(path.clone(), missing_dir.clone());
        assert!(Arc::ptr_eq(&first, &cached));

        std::fs::write(&list, "deb http://example.com/debian trixie main\n").unwrap();
        let reloaded = configured_sources(path, missing_dir);
        assert!(!Arc::ptr_eq(&first, &reloaded));
        assert_eq!(reloaded.entries().len(), 1);
        assert_eq!(
            reloaded.suite_for("http://example.com/debian", None, None),
            Some("trixie")
        );

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn fingerprint_reports_metadata_errors() {
        let missing = temp_dir().join("missing");
        assert!(source_fingerprint(&[missing]).is_err());
    }

    #[test]
    fn disambiguates_multiple_suites_with_release_metadata() {
        let sources = ConfiguredSources::from_entries(vec![
            SourceEntry {
                enabled: true,
                source: false,
                options: vec![],
                url: "http://example.com/debian/".to_string(),
                suite: "bookworm".to_string(),
                components: vec!["main".to_string()],
                is_deb822: false,
                archs: None,
                signed_by: None,
                trusted: false,
            },
            SourceEntry {
                enabled: true,
                source: false,
                options: vec![],
                url: "http://example.com/debian".to_string(),
                suite: "trixie".to_string(),
                components: vec!["main".to_string()],
                is_deb822: false,
                archs: None,
                signed_by: None,
                trusted: false,
            },
        ]);

        assert_eq!(
            sources.suite_for(
                "http://example.com/debian/",
                Some("oldstable"),
                Some("bookworm")
            ),
            Some("bookworm")
        );
        assert_eq!(
            sources.suite_for("http://example.com/debian", Some("oldstable"), None),
            None
        );
    }
}
