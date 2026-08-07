use std::{cmp::Ordering, collections::HashSet, path::Path};

use ahash::AHashMap;
use oma_apt_pkg::AptConfig;
use oma_apt_pkg::apt_sources::{
    IndexTargetTemplates, find_matching_combinations, strip_compression_ext, substitute,
};
use oma_fetch::CompressType;
use once_cell::sync::OnceCell;
use spdlog::debug;

use crate::{db::RefreshError, inrelease::ChecksumItem};

static COMPRESSION_ORDER: OnceCell<Vec<CompressFileWrapper>> = OnceCell::new();

#[derive(Debug, Eq, PartialEq)]
struct CompressFileWrapper {
    compress_file: CompressType,
}

impl From<&str> for CompressFileWrapper {
    fn from(value: &str) -> Self {
        match value {
            "xz" => CompressFileWrapper {
                compress_file: CompressType::Xz,
            },
            "bz2" => CompressFileWrapper {
                compress_file: CompressType::Bz2,
            },
            "lzma" => CompressFileWrapper {
                compress_file: CompressType::Lzma,
            },
            "gz" => CompressFileWrapper {
                compress_file: CompressType::Gzip,
            },
            "lz4" => CompressFileWrapper {
                compress_file: CompressType::Lz4,
            },
            "zst" => CompressFileWrapper {
                compress_file: CompressType::Zstd,
            },
            x => {
                if !x.is_ascii() {
                    debug!("{x} format is not compress format");
                }

                CompressFileWrapper {
                    compress_file: CompressType::None,
                }
            }
        }
    }
}

impl PartialOrd for CompressFileWrapper {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for CompressFileWrapper {
    fn cmp(&self, other: &Self) -> Ordering {
        let t = COMPRESSION_ORDER.get_or_init(|| {
            vec!["zst", "xz", "bz2", "lzma", "gz", "lz4", "uncompressed"]
                .into_iter()
                .map(CompressFileWrapper::from)
                .collect()
        });

        let self_pos = t.iter().position(|x| x == self).unwrap();
        let other_pos = t.iter().position(|x| x == other).unwrap();

        other_pos.cmp(&self_pos)
    }
}

impl From<CompressType> for CompressFileWrapper {
    fn from(value: CompressType) -> Self {
        Self {
            compress_file: value,
        }
    }
}

/// IndexTarget config — stores only the enabled target keys and reads
/// properties directly from [`AptConfig`] on demand.
pub struct IndexTargetConfig<'a> {
    cfg: &'a AptConfig,
    deb_keys: Vec<String>,
    deb_src_keys: Vec<String>,
    native_arch: &'a str,
    langs: Vec<String>,
}

impl<'a> IndexTargetConfig<'a> {
    pub fn new_from_apt_config(apt_cfg: &'a AptConfig, native_arch: &'a str) -> Self {
        let templates = IndexTargetTemplates::new(apt_cfg);
        let locales = sys_locale::get_locales();
        let langs = get_matches_language(locales);

        Self {
            cfg: apt_cfg,
            deb_keys: templates.get_enabled_keys("Acquire::IndexTargets::deb"),
            deb_src_keys: templates.get_enabled_keys("Acquire::IndexTargets::deb-src"),
            native_arch,
            langs,
        }
    }

    pub fn get_download_list(
        &self,
        release: &str,
        checksums: &[ChecksumItem],
        is_source: bool,
        is_flat: bool,
        archs: Vec<&str>,
        components: &[String],
        supported_archs: Option<&[&str]>,
    ) -> Result<Vec<ChecksumDownloadEntry>, RefreshError> {
        let meta_key = if is_flat { "flatMetaKey" } else { "MetaKey" };
        let mut res_map: AHashMap<String, Vec<ChecksumDownloadEntry>> = AHashMap::new();
        let tree = if is_source {
            &self.deb_src_keys
        } else {
            &self.deb_keys
        };

        let mut archs = archs;

        if !archs.contains(&"all") {
            archs.push("all");
        }

        // 与 apt 一致：如果 Release 文件声明了 `Architectures:` 字段，则跳过
        // 仓库不支持的架构（例如不提供 `binary-all` 的仓库），避免无谓地下载失败。
        if !is_source && let Some(supported_archs) = supported_archs {
            let len_before = archs.len();
            archs.retain(|a| supported_archs.contains(a));
            if archs.len() != len_before {
                debug!(
                    "Skipping architectures not supported by the repository, remaining: {archs:?}"
                );
            }
        }

        let templates = IndexTargetTemplates::new(self.cfg);

        for c in checksums {
            let name = strip_compression_ext(&c.name);

            if is_flat {
                let matches = templates
                    .resolve_targets(name, release, &archs, "", "", "", true)
                    .map_err(|e| RefreshError::InvalidUrl(e.to_string()))?;
                for m in &matches {
                    res_map
                        .entry(name.to_string())
                        .or_default()
                        .push(to_download_entry(
                            c,
                            self.cfg,
                            &m.config_key,
                            &m.description,
                        ));
                }
            } else {
                let comps: Vec<&str> = components.iter().map(|s| s.as_str()).collect();
                let langs: Vec<&str> = self.langs.iter().map(|s| s.as_str()).collect();

                for config_key in tree {
                    let template = self.cfg.get(&format!("{config_key}::{meta_key}"), "");
                    if template.is_empty() {
                        continue;
                    }
                    for m in find_matching_combinations(
                        &template,
                        release,
                        name,
                        &archs,
                        &comps,
                        &langs,
                        self.native_arch,
                    ) {
                        let desc = self.cfg.get(&format!("{config_key}::ShortDescription"), "");
                        let desc = if desc.is_empty() {
                            "Other".to_string()
                        } else {
                            substitute(
                                &desc,
                                release,
                                &m.component,
                                &m.arch,
                                &m.lang,
                                self.native_arch,
                            )
                        };
                        res_map
                            .entry(name.to_string())
                            .or_default()
                            .push(to_download_entry(c, self.cfg, config_key, &desc));
                    }
                }
            }
        }

        let mut sort_res = vec![];

        for (_, v) in &mut res_map {
            v.sort_unstable_by(|a, b| {
                compress_file(&a.item.name).cmp(&compress_file(&b.item.name))
            });
            if v[0].item.size == 0 {
                continue;
            }
            sort_res.push(v.last().unwrap().to_owned());
        }

        Ok(fallback_of_filter(sort_res))
    }
}

fn fallback_of_filter(res: Vec<ChecksumDownloadEntry>) -> Vec<ChecksumDownloadEntry> {
    if res.len() <= 1 {
        return res;
    }

    let config_keys: HashSet<_> = res.iter().map(|e| e.config_key.clone()).collect();

    res.into_iter()
        .filter(|entry| {
            if let Some(fallback_of) = &entry.fallback_of
                && config_keys.contains(fallback_of)
            {
                debug!(
                    "Skip {} because it has fallback_of pointing to existing key",
                    entry.config_key
                );
                false
            } else {
                true
            }
        })
        .collect()
}

fn to_download_entry(
    c: &ChecksumItem,
    cfg: &AptConfig,
    config_key: &str,
    msg: &str,
) -> ChecksumDownloadEntry {
    ChecksumDownloadEntry {
        item: c.to_owned(),
        keep_compress: cfg
            .get(&format!("{config_key}::KeepCompressed"), "")
            .parse::<bool>()
            .unwrap_or(false),
        msg: msg.to_string(),
        optional: match cfg.get(&format!("{config_key}::Optional"), "").as_str() {
            "0" => false,
            "1" => true,
            _ => true,
        },
        config_key: config_key
            .rsplit_once("::")
            .unwrap_or_default()
            .1
            .to_string(),
        fallback_of: {
            let v = cfg.get(&format!("{config_key}::Fallback-Of"), "");
            if v.is_empty() { None } else { Some(v) }
        },
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ChecksumDownloadEntry {
    pub item: ChecksumItem,
    pub keep_compress: bool,
    pub msg: String,
    pub optional: bool,
    pub config_key: String,
    pub fallback_of: Option<String>,
}

fn get_matches_language(locales: impl IntoIterator<Item = String>) -> Vec<String> {
    let mut langs = vec![];

    for locale in locales {
        if locale.eq_ignore_ascii_case("c") {
            langs.push("en".to_string());
            continue;
        }

        // apt 数据库使用下划线来设置 translation 文件名
        let locale = locale.replace("-", "_");

        if let Some((lang, _)) = locale.split_once("_") {
            langs.push(lang.to_lowercase());
        }

        langs.push(locale);
    }

    if langs.is_empty() {
        langs.push("en".to_string());
    }

    langs
}

fn compress_file(name: &str) -> CompressFileWrapper {
    CompressFileWrapper {
        compress_file: CompressType::from(
            Path::new(name)
                .extension()
                .map(|x| x.to_string_lossy())
                .unwrap_or_default()
                .to_string()
                .as_str(),
        ),
    }
}

#[test]
fn test_apt_config() {
    // Test that compression ordering is correct
    let mut types: Vec<CompressFileWrapper> = vec![
        CompressType::None,
        CompressType::Xz,
        CompressType::Zstd,
        CompressType::Gzip,
        CompressType::Bz2,
        CompressType::Lz4,
        CompressType::Lzma,
    ]
    .into_iter()
    .map(|x| x.into())
    .collect();

    types.sort_unstable();
    types.reverse();

    assert_eq!(
        types,
        vec![
            CompressType::Zstd,
            CompressType::Xz,
            CompressType::Bz2,
            CompressType::Lzma,
            CompressType::Gzip,
            CompressType::Lz4,
            CompressType::None,
        ]
        .into_iter()
        .map(|x| x.into())
        .collect::<Vec<CompressFileWrapper>>()
    );

    // Test that IndexTarget tree is correctly parsed from AptConfig
    let mut cfg = AptConfig::new();
    cfg.init_defaults().unwrap();
    let templates = IndexTargetTemplates::new(&cfg);
    let t = templates.get_enabled_keys("Acquire::IndexTargets::deb");
    assert!(t.iter().any(|x| x.contains("::deb::")));
    assert!(t.iter().all(|x| !x.contains("::deb-src::")));
}

#[test]
fn test_get_matches_language() {
    assert_eq!(get_matches_language(vec!["C".to_string()]), vec!["en"]);
    assert_eq!(
        get_matches_language(vec!["zh-CN".to_string()]),
        vec!["zh", "zh_CN"]
    );
    assert_eq!(
        get_matches_language(vec!["en-US".to_string()]),
        vec!["en", "en_US"]
    );
}

#[test]
fn test_fallback_of_filter() {
    let entries = vec![
        ChecksumDownloadEntry {
            item: ChecksumItem {
                name: "file1".to_string(),
                size: 100,
                checksum: "abc".to_string(),
            },
            keep_compress: false,
            msg: "msg1".to_string(),
            optional: false,
            config_key: "key1".to_string(),
            fallback_of: None,
        },
        ChecksumDownloadEntry {
            item: ChecksumItem {
                name: "file2".to_string(),
                size: 100,
                checksum: "def".to_string(),
            },
            keep_compress: false,
            msg: "msg2".to_string(),
            optional: false,
            config_key: "key2".to_string(),
            fallback_of: Some("key1".to_string()),
        },
        ChecksumDownloadEntry {
            item: ChecksumItem {
                name: "file3".to_string(),
                size: 100,
                checksum: "ghi".to_string(),
            },
            keep_compress: false,
            msg: "msg3".to_string(),
            optional: false,
            config_key: "key3".to_string(),
            fallback_of: None,
        },
    ];

    let filtered = fallback_of_filter(entries);
    assert_eq!(filtered.len(), 2);

    for i in ["key1", "key3"] {
        assert!(filtered.iter().any(|x| x.config_key == i));
    }
}
