use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::OnceLock;

use aho_corasick::{AhoCorasick, BuildError};
use oma_apt_sources_lists::{SourceEntry, SourceLine, SourceListType, SourcesList};

use crate::AptConfig;

/// Scan the filesystem for sources list files, returning their paths.
///
/// Like APT's `GetListOfFilesInDir`, this looks for the default
/// `sourcelist` file and scans `sourceparts` directory for additional
/// regular files. Errors are silently skipped (empty vec on failure).
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

/// Lookup from (host+path) → source entries, built from sources.list.
pub struct SourceLookup {
    entries: Vec<SourceEntry>,
    lookup: HashMap<String, Vec<usize>>,
}

impl SourceLookup {
    /// Build a source lookup from the default apt sources list paths.
    ///
    /// Like APT's `pkgSourceList`, this reads `Dir::Etc::sourcelist` and
    /// `Dir::Etc::sourceparts` from config, scans for files, and collects
    /// all enabled entries.
    pub fn build(apt_cfg: &AptConfig) -> Self {
        let list_file = apt_cfg.get_file("Dir::Etc::sourcelist", "etc/apt/sources.list");
        let list_dir = apt_cfg.get_dir("Dir::Etc::sourceparts", "etc/apt/sources.list.d");
        let paths = scan_sources_list_paths(&list_file, &list_dir);
        Self::from_paths(&paths, |_: &Path| {})
    }

    /// Build from an .list and .sources file of paths.
    ///
    /// - `on_unknown_file` is called for each file whose extension is
    ///   neither `.list` nor `.sources` (matching APT's `UnknownFile`
    ///   behavior). The callback can inspect the file and decide whether
    ///   to warn (e.g. skip files matching `Dir::Ignore-Files-Silently`).
    pub fn from_paths<G>(paths: &[PathBuf], mut on_unknown_file: G) -> Self
    where
        G: FnMut(&Path),
    {
        let mut entries = Vec::new();
        let mut lookup: HashMap<String, Vec<usize>> = HashMap::new();

        for path in paths {
            let Ok(s) = SourcesList::new(path) else {
                on_unknown_file(path);
                continue;
            };

            let parsed: Box<dyn Iterator<Item = SourceEntry>> = match s.entries {
                SourceListType::SourceLine(lines) => {
                    Box::new(lines.0.into_iter().filter_map(|l| {
                        if let SourceLine::Entry(e) = l {
                            Some(e)
                        } else {
                            None
                        }
                    }))
                }
                SourceListType::Deb822(deb822) => Box::new(deb822.entries.into_iter()),
            };

            for (idx, entry) in parsed.filter(|e| e.enabled).enumerate() {
                let key = entry
                    .url()
                    .split_once("://")
                    .map_or(entry.url(), |(_, rest)| rest)
                    .trim_end_matches('/')
                    .to_string();

                lookup.entry(key).or_default().push(idx);
                entries.push(entry);
            }
        }

        Self { entries, lookup }
    }

    /// All parsed entries (enabled only).
    pub fn entries(&self) -> &[SourceEntry] {
        &self.entries
    }

    /// Find which source entry a decoded URI belongs to, by checking if the
    /// URI (without scheme) starts with a known key.
    ///
    /// This approximates APT's `FindIndex` (sourcelist.cc) via URL prefix
    /// matching.
    pub fn resolve<'a>(&'a self, decoded: &'a str) -> Option<SourceMatch<'a>> {
        let host_path = decoded.split_once("://").map_or(decoded, |(_, rest)| rest);
        let base_key = self
            .lookup
            .keys()
            .filter(|k| host_path.starts_with(*k))
            .max_by_key(|k| k.len())?;

        let &idx = self.lookup[base_key]
            .iter()
            .find(|&&idx| {
                let entry = &self.entries[idx];
                host_path.contains(&format!("/dists/{}/", entry.suite.trim_end_matches('/')))
            })
            .or_else(|| self.lookup[base_key].first())?;

        let entry = &self.entries[idx];

        let rest = &host_path[base_key.len()..];
        let is_flat = entry.suite.ends_with('/');

        // Extract the bare filename from the last path segment
        let filename = rest.rsplit('/').next().unwrap_or("");
        let filename = strip_compression_ext(filename);

        let component = if is_flat {
            None
        } else {
            // Compare against host+path (without scheme), since decoded has no
            // scheme but dist_components() returns full URLs with scheme.
            entry
                .components
                .iter()
                .zip(entry.dist_components())
                .find(|(_, url)| {
                    let comp_host_path =
                        url.split_once("://").map_or(url.as_str(), |(_, rest)| rest);
                    host_path.starts_with(comp_host_path)
                })
                .map(|(name, _)| name.as_str())
        };

        Some(SourceMatch {
            entry,
            component,
            filename,
        })
    }
}

/// The result of matching a decoded URI against a [`SourceLookup`].
#[derive(Debug)]
pub struct SourceMatch<'a> {
    /// The matched source entry.
    pub entry: &'a SourceEntry,
    /// The component name, or `None` for flat repos.
    pub component: Option<&'a str>,
    /// The bare filename (last segment of the decoded URI, stripped of
    /// compression extensions).
    pub filename: &'a str,
}

/// A resolved IndexTarget match: the target key, its Description template,
/// and the resolved architecture string.
#[derive(Debug, Clone)]
pub struct TargetResolution {
    pub config_key: String,
    pub description: String,
    pub arch: String,
}

/// Reads and queries the `Acquire::IndexTargets` configuration tree for
/// template matching and description resolution.
pub struct IndexTargetTemplates<'a> {
    cfg: &'a AptConfig,
}

impl<'a> IndexTargetTemplates<'a> {
    pub fn new(cfg: &'a AptConfig) -> Self {
        Self { cfg }
    }

    /// Return the target keys for enabled targets (those without
    /// `DefaultEnabled=false`).
    pub fn get_enabled_keys(&self, key: &str) -> Vec<String> {
        self.cfg
            .keys_under(key)
            .into_iter()
            .filter(|target| {
                let target_key = format!("{key}::{target}");
                let enabled = self.cfg.get(&format!("{target_key}::DefaultEnabled"), "");
                enabled.is_empty() || enabled.parse::<bool>().unwrap_or(true)
            })
            .map(|target| format!("{key}::{target}"))
            .collect()
    }

    /// Iterate all `Acquire::IndexTargets` entries, read `MetaKey` (or
    /// `flatMetaKey`), substitute `$(ARCHITECTURE)` with each arch from
    /// `archs`, run full template substitution on the result, and collect
    /// every target whose substituted MetaKey matches `filename`.
    ///
    /// The returned `TargetResolution` contains the raw description
    /// template (with `$(RELEASE)` etc. still unsubstituted); the caller
    /// should perform substitution via [`substitute`] when formatting.
    #[allow(clippy::too_many_arguments)]
    pub fn resolve_targets(
        &self,
        filename: &str,
        release: &str,
        archs: &[&str],
        component: &str,
        lang: &str,
        native_arch: &str,
        is_flat: bool,
    ) -> Result<Vec<TargetResolution>, crate::Error> {
        let mut results = Vec::new();
        for group in self.cfg.keys_under("Acquire::IndexTargets") {
            let group_prefix = format!("Acquire::IndexTargets::{group}");
            for target in self.cfg.keys_under(&group_prefix) {
                let config_key = format!("{group_prefix}::{target}");

                let meta_key = if is_flat {
                    self.cfg.get(&format!("{config_key}::flatMetaKey"), "")
                } else {
                    self.cfg.get(&format!("{config_key}::MetaKey"), "")
                };

                if meta_key.is_empty() {
                    continue;
                }

                for &arch in archs {
                    let substituted = meta_key.replace("$(ARCHITECTURE)", arch);
                    if substitute(&substituted, release, component, arch, lang, native_arch)
                        != filename
                    {
                        continue;
                    }

                    let desc_key = if is_flat {
                        "flatDescription"
                    } else {
                        "Description"
                    };
                    let desc = self.cfg.get(&format!("{config_key}::{desc_key}"), "");
                    if desc.is_empty() {
                        continue;
                    }

                    let actual_arch = if arch == "$(ARCHITECTURE)" {
                        let a = self.cfg.get("APT::Architecture", "");
                        if a.is_empty() {
                            return Err(crate::Error::AptSources(
                                "APT::Architecture is not set but required \
                                 for IndexTarget template resolution"
                                    .into(),
                            ));
                        }
                        a
                    } else {
                        arch.to_string()
                    };

                    results.push(TargetResolution {
                        config_key: config_key.clone(),
                        description: desc,
                        arch: actual_arch,
                    });
                }
            }
        }

        Ok(results)
    }
}

/// Substitute all APT template variables in a string.
///
/// Supports: `$(RELEASE)`, `$(COMPONENT)`, `$(ARCHITECTURE)`,
/// `$(LANGUAGE)`, `$(NATIVE_ARCHITECTURE)`.
static APT_TEMPLATE_AC: OnceLock<Result<AhoCorasick, BuildError>> = OnceLock::new();

/// Substitute all APT template variables in a string.
///
/// Supports: `$(RELEASE)`, `$(COMPONENT)`, `$(ARCHITECTURE)`,
/// `$(LANGUAGE)`, `$(NATIVE_ARCHITECTURE)`.
///
/// Falls back to chained `.replace()` if the Aho-Corasick engine fails to
/// build (should never happen with hardcoded patterns).
pub fn substitute(
    template: &str,
    release: &str,
    component: &str,
    arch: &str,
    lang: &str,
    native_arch: &str,
) -> String {
    let patterns = &[
        "$(RELEASE)",
        "$(COMPONENT)",
        "$(ARCHITECTURE)",
        "$(LANGUAGE)",
        "$(NATIVE_ARCHITECTURE)",
    ];
    let replacements = [release, component, arch, lang, native_arch];

    let ac = APT_TEMPLATE_AC.get_or_init(|| AhoCorasick::new(patterns));
    match ac {
        Ok(ac) => ac.replace_all(template, &replacements),
        Err(_) => template
            .replace("$(RELEASE)", release)
            .replace("$(COMPONENT)", component)
            .replace("$(ARCHITECTURE)", arch)
            .replace("$(LANGUAGE)", lang)
            .replace("$(NATIVE_ARCHITECTURE)", native_arch),
    }
}

/// The variables that made a template match.
#[derive(Debug, Clone)]
pub struct MatchResult {
    pub arch: String,
    pub component: String,
    pub lang: String,
}

/// Try all combinations of `release x archs × components × langs` and return every
/// set of variables for which the template matches.
pub fn find_matching_combinations(
    template: &str,
    release: &str,
    filename: &str,
    archs: &[&str],
    components: &[&str],
    langs: &[&str],
    native_arch: &str,
) -> Vec<MatchResult> {
    let mut results = Vec::new();
    for a in archs {
        for comp in components {
            for lang in langs {
                if substitute(template, release, comp, a, lang, native_arch) == filename {
                    results.push(MatchResult {
                        arch: a.to_string(),
                        component: comp.to_string(),
                        lang: lang.to_string(),
                    });
                }
            }
        }
    }

    results
}

/// Strip well-known compression extensions from a filename.
pub fn strip_compression_ext(name: &str) -> &str {
    for ext in &[".xz", ".bz2", ".gz", ".lzma", ".lz4", ".zst"] {
        if let Some(stripped) = name.strip_suffix(ext) {
            return stripped;
        }
    }

    name
}
