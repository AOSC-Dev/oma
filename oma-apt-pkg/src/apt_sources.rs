use std::path::{Path, PathBuf};
use std::sync::OnceLock;

use aho_corasick::{AhoCorasick, BuildError};
use oma_apt_sources_lists::{SourceEntry, SourceLine, SourceListType, SourcesList};

use crate::AptConfig;
use crate::apt_lists::IndexSource;
use crate::filename::AptListFilename;

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

/// The configured sources parsed from `sources.list`, the driver of the
/// package database: it generates the lists files to read (forward), so
/// nothing ever looks a filename up again.
pub struct SourceLookup {
    entries: Vec<SourceEntry>,
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

            entries.extend(parsed.filter(|e| e.enabled));
        }

        Self { entries }
    }

    /// All parsed entries (enabled only).
    pub fn entries(&self) -> &[SourceEntry] {
        &self.entries
    }

    /// The (lists filename, [`IndexSource`]) pairs this source list
    /// produces, mirroring apt's cache generation which iterates the source
    /// list and reads exactly the index targets of each configured source:
    /// for every component, the `binary-<arch>` index for each configured
    /// architecture plus `binary-all`, and the flat `Packages` for flat
    /// repositories.
    ///
    /// `archs` come from [`AptConfig::architectures`](crate::AptConfig::architectures);
    /// `binary-all` is always included, like apt. The lists filename is the
    /// `URItoFileName` encoding of the index URI, so a parser reads exactly
    /// the files in this list that exist under `Dir::State::lists` — lists
    /// files that no configured source produces are never read.
    pub fn index_files(&self, archs: &[String]) -> Vec<(String, IndexSource)> {
        let cvt = AptListFilename::new();
        let mut out = Vec::new();
        for entry in &self.entries {
            let base = entry.url().trim_end_matches('/');
            let suite = entry.suite.trim_end_matches('/');
            if entry.components.is_empty() {
                // Flat repository: the index lives at the repository root.
                let uri = format!("{base}/Packages");
                if let Ok(filename) = cvt.encode(&uri) {
                    out.push((
                        filename,
                        IndexSource {
                            base_url: base.to_string(),
                            suite: suite.to_string(),
                            component: None,
                            arch: None,
                        },
                    ));
                }
                continue;
            }
            for component in &entry.components {
                for arch in archs
                    .iter()
                    .map(|a| a.as_str())
                    .chain(std::iter::once("all"))
                {
                    let uri = format!("{base}/dists/{suite}/{component}/binary-{arch}/Packages");
                    if let Ok(filename) = cvt.encode(&uri) {
                        out.push((
                            filename,
                            IndexSource {
                                base_url: base.to_string(),
                                suite: suite.to_string(),
                                component: Some(component.clone()),
                                arch: Some(arch.to_string()),
                            },
                        ));
                    }
                }
            }
        }
        out
    }
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

#[cfg(test)]
mod tests {
    use super::*;

    /// Build a `SourceLookup` from a single `.sources` (deb822) file.
    fn lookup_from(text: &str) -> SourceLookup {
        let dir = tempfile::tempdir().unwrap();
        let list = dir.path().join("test.sources");
        std::fs::write(&list, text).unwrap();
        SourceLookup::from_paths(&[list], |_| {})
    }

    #[test]
    fn test_index_files_generates_per_suite() {
        // Two entries sharing one URL but different suites. Regression for
        // a bug where matching could never reach the second entry and fell
        // back to the first — index generation must cover every configured
        // source, including ones sharing a base URL.
        let lookup = lookup_from(
            "Types: deb\n\
             URIs: https://example.com/debs\n\
             Suites: stable\n\
             Components: main\n\
             Signed-By: /dev/null\n\
             \n\
             Types: deb\n\
             URIs: https://example.com/debs\n\
             Suites: kde-6\n\
             Components: main\n\
             Signed-By: /dev/null\n",
        );

        assert_eq!(lookup.entries().len(), 2);

        let archs = vec!["amd64".to_string()];
        let files = lookup.index_files(&archs);

        // Every suite × (amd64, all) produces one lists filename, all
        // sharing the base URL but carrying their own suite/arch.
        let find = |suite: &str, arch: &str| {
            files
                .iter()
                .find(|(_, s)| s.suite == suite && s.arch.as_deref() == Some(arch))
                .map(|(f, s)| (f.as_str(), s.base_url.as_str()))
        };

        let (f, base) = find("stable", "amd64").expect("stable amd64");
        assert_eq!(
            f,
            "example.com_debs_dists_stable_main_binary-amd64_Packages"
        );
        assert_eq!(base, "https://example.com/debs");
        assert_eq!(files.len(), 4);

        // The second entry must be generated too, not skipped.
        let (f, base) = find("kde-6", "amd64").expect("kde-6 amd64");
        assert_eq!(f, "example.com_debs_dists_kde-6_main_binary-amd64_Packages");
        assert_eq!(base, "https://example.com/debs");

        // `binary-all` is always included, like apt.
        let (f, _) = find("stable", "all").expect("stable all");
        assert_eq!(f, "example.com_debs_dists_stable_main_binary-all_Packages");

        // No orphan suite is ever generated: files come only from the
        // configured sources, so removed/disabled suites leave no traces.
        assert!(files.iter().all(|(_, s)| s.suite != "removed-suite"));
    }
}
