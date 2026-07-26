use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::OnceLock;

use aho_corasick::{AhoCorasick, BuildError};
use oma_apt_sources_lists::{SourceEntry, SourceLine, SourceListType, SourcesLists};

use crate::AptConfig;

// ---------------------------------------------------------------------------
// SourceLookup — built from sources.list, resolves decoded URIs
// ---------------------------------------------------------------------------

/// Lookup from (host+path) → source entry, built from sources.list.
pub struct SourceLookup {
    inner: HashMap<String, SourceEntry>,
}

impl SourceLookup {
    /// Build a lookup from (host+path) → source entry by reading the apt
    /// sources list files via [`SourcesLists::new_from_paths`].
    ///
    /// Like APT's `pkgSourceList`, this collects the URL (as SITE), suite
    /// and type for each configured source.
    pub fn build(apt_cfg: &AptConfig) -> Self {
        let mut inner = HashMap::new();

        let list_file = apt_cfg.get_file("Dir::Etc::sourcelist", "etc/apt/sources.list");
        let list_dir = apt_cfg.get_dir("Dir::Etc::sourceparts", "etc/apt/sources.list.d");

        let mut paths = Vec::new();
        if std::path::Path::new(&list_file).exists() {
            paths.push(PathBuf::from(&list_file));
        }
        if let Ok(dir) = std::fs::read_dir(&list_dir) {
            for entry in dir.flatten() {
                let p = entry.path();
                let ext = p.extension().and_then(|e| e.to_str()).unwrap_or("");
                if ext == "list" || ext == "sources" {
                    paths.push(p);
                }
            }
        }

        let Ok(lists) = SourcesLists::new_from_paths(paths.iter()) else {
            return Self { inner };
        };

        for sources_list in lists.iter() {
            let entries: Vec<&SourceEntry> = match &sources_list.entries {
                SourceListType::SourceLine(lines) => lines
                    .0
                    .iter()
                    .filter_map(|l| {
                        if let SourceLine::Entry(e) = l {
                            Some(e)
                        } else {
                            None
                        }
                    })
                    .collect(),
                SourceListType::Deb822(deb822) => deb822.entries.iter().collect(),
            };

            for entry in &entries {
                let key = entry
                    .url
                    .split("://")
                    .nth(1)
                    .unwrap_or(&entry.url)
                    .trim_end_matches('/')
                    .to_string();

                inner.entry(key).or_insert_with(|| (*entry).clone());
            }
        }

        Self { inner }
    }

    /// Find which source entry a decoded URI belongs to, by checking if the
    /// URI (without scheme) starts with a known key.
    ///
    /// This approximates APT's `FindIndex` (sourcelist.cc) via URL prefix
    /// matching.
    pub fn resolve<'a>(&'a self, decoded: &'a str) -> Option<SourceMatch<'a>> {
        let host_path = decoded.split("://").nth(1).unwrap_or(decoded);
        let (key, entry) = self.inner.iter().find(|(k, _)| host_path.starts_with(*k))?;

        let rest = &host_path[key.len()..];
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
                    let comp_host_path = url.split("://").nth(1).unwrap_or(url);
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
    /// `archs`, and collect every target for which the resulting path
    /// matches `filename` via full template substitution.
    ///
    /// This is the shared core used by both `show`'s APT-Sources formatting
    /// and `oma-refresh`'s InRelease-to-download-list matching.
    pub fn resolve_targets(
        &self,
        filename: &str,
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
                    if substitute(&substituted, "", component, arch, lang, native_arch) != filename
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

/// Try all combinations of `archs × components × langs` and return every
/// set of variables for which the template matches.
///
/// Used by `oma-refresh`'s InRelease-to-download-list matching.
pub fn find_matching_combinations(
    template: &str,
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
                if substitute(template, "", comp, a, lang, native_arch) == filename {
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

// ---------------------------------------------------------------------------
// Internal helpers
// ---------------------------------------------------------------------------

/// Strip well-known compression extensions from a filename.
pub fn strip_compression_ext(name: &str) -> &str {
    for ext in &[".xz", ".bz2", ".gz", ".lzma", ".lz4", ".zst"] {
        if let Some(stripped) = name.strip_suffix(ext) {
            return stripped;
        }
    }
    name
}
