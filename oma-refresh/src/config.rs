use std::{collections::HashSet, path::Path};

use ahash::AHashMap;
use oma_apt_pkg::AptConfig;
use oma_apt_pkg::apt_sources::{
    IndexTargetTemplates, find_matching_combinations, strip_compression_ext, substitute,
};
use spdlog::debug;

use crate::{db::RefreshError, inrelease::ChecksumItem};

/// Compression formats oma can decode, in oma's preferred order (`zst` first).
/// Seeds the AOSC default `Acquire::CompressionTypes::Order`, and filters the
/// `Acquire::CompressionTypes` tree when no Order is configured (like apt
/// filters by available decompression methods).
pub(crate) const DECODABLE_COMPRESSION_FORMATS: [&str; 6] =
    ["zst", "xz", "bz2", "lzma", "gz", "lz4"];

/// IndexTarget config — stores only the enabled target keys and reads
/// properties directly from [`AptConfig`] on demand.
pub struct IndexTargetConfig<'a> {
    cfg: &'a AptConfig,
    deb_keys: Vec<String>,
    deb_src_keys: Vec<String>,
    native_arch: &'a str,
    langs: Vec<String>,
    /// Compression types to try in order, read from
    /// `Acquire::CompressionTypes::Order`. The uncompressed format (`""`) is
    /// always appended last, like apt's `pkgAcqIndex`.
    compression_order: Vec<String>,
}

impl<'a> IndexTargetConfig<'a> {
    pub fn new_from_apt_config(apt_cfg: &'a AptConfig, native_arch: &'a str) -> Self {
        let templates = IndexTargetTemplates::new(apt_cfg);
        let locales = sys_locale::get_locales();
        let langs = get_matches_language(locales);

        let mut compression_order: Vec<String> = apt_cfg
            .keys_under("Acquire::CompressionTypes::Order")
            .map(str::to_owned)
            .collect();

        // 显式配置了 `Acquire::CompressionTypes::Order` 就直接使用；为空时才
        // 遍历整个 `Acquire::CompressionTypes` 树取默认顺序（与 apt 的
        // `getCompressionTypes` 一致），且只保留 oma 能解码的格式。
        if compression_order.is_empty() {
            for ext in apt_cfg.keys_under("Acquire::CompressionTypes") {
                if ext == "Order" {
                    continue;
                }
                if !DECODABLE_COMPRESSION_FORMATS.contains(&ext) {
                    continue;
                }
                compression_order.push(ext.to_string());
            }
        }
        // apt always tries the uncompressed file last.
        compression_order.push(String::new());

        Self {
            cfg: apt_cfg,
            deb_keys: templates.get_enabled_keys("Acquire::IndexTargets::deb"),
            deb_src_keys: templates.get_enabled_keys("Acquire::IndexTargets::deb-src"),
            native_arch,
            langs,
            compression_order,
        }
    }

    #[allow(clippy::too_many_arguments)]
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
        // Group every compression variant of the same index file together;
        // each group becomes one `ChecksumDownloadEntry` whose `items` holds
        // all variants so the downloader can fall back between them.
        let mut res_map: AHashMap<String, ChecksumDownloadEntry> = AHashMap::new();
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
                    let entry = res_map.entry(name.to_string()).or_insert_with(|| {
                        to_download_entry(self.cfg, &m.config_key, &m.description)
                    });
                    if !entry.items.iter().any(|x| x.name == c.name) {
                        entry.items.push(c.to_owned());
                    }
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
                                m.component,
                                m.arch,
                                m.lang,
                                self.native_arch,
                            )
                        };
                        let entry = res_map
                            .entry(name.to_string())
                            .or_insert_with(|| to_download_entry(self.cfg, config_key, &desc));
                        if !entry.items.iter().any(|x| x.name == c.name) {
                            entry.items.push(c.to_owned());
                        }
                    }
                }
            }
        }

        let mut result = vec![];

        for (_, mut entry) in res_map {
            // Sort variants by the configured compression order
            // (`Acquire::CompressionTypes::Order`), preferred format first:
            // `Packages.zst` → `Packages.xz` → `Packages.gz`, matching apt.
            entry
                .items
                .sort_unstable_by_key(|a| self.compression_rank(&a.name));
            if entry.items[0].size == 0 {
                continue;
            }
            result.push(entry);
        }

        Ok(fallback_of_filter(result))
    }

    /// Position of `name`'s compression type in the configured download
    /// order; smaller means higher priority (tried first). Unknown types sort
    /// after everything else.
    fn compression_rank(&self, name: &str) -> usize {
        let ext = Path::new(name)
            .extension()
            .map(|x| x.to_string_lossy().into_owned())
            .unwrap_or_default();
        self.compression_order
            .iter()
            .position(|x| *x == ext)
            .unwrap_or(self.compression_order.len())
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

fn to_download_entry(cfg: &AptConfig, config_key: &str, msg: &str) -> ChecksumDownloadEntry {
    ChecksumDownloadEntry {
        items: vec![],
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
    /// Every compression variant of the index file, ordered best-first.
    /// `items[0]` is the preferred (highest-priority) compression format; the
    /// rest are fallbacks tried in order when it is unavailable, e.g.
    /// `Packages.zst` → `Packages.xz` → `Packages.gz`, matching apt's
    /// `Acquire::CompressionTypes::Order` behavior.
    pub items: Vec<ChecksumItem>,
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

#[test]
fn test_apt_config() {
    // Test that IndexTarget tree is correctly parsed from AptConfig
    let mut cfg = AptConfig::new();
    cfg.init_defaults().unwrap();
    let templates = IndexTargetTemplates::new(&cfg);
    let t = templates.get_enabled_keys("Acquire::IndexTargets::deb");
    assert!(t.iter().any(|x| x.contains("::deb::")));
    assert!(t.iter().all(|x| !x.contains("::deb-src::")));
}

#[test]
fn test_compression_rank_reads_apt_config() {
    let mut cfg = AptConfig::new();
    cfg.init_defaults().unwrap();
    for c in &["xz", "gz", "zst"] {
        cfg.set_list("Acquire::CompressionTypes::Order", c);
    }
    let config = IndexTargetConfig::new_from_apt_config(&cfg, "amd64");

    // Order read from config: xz first, then gz, then zst, uncompressed last.
    assert!(config.compression_rank("Packages.xz") < config.compression_rank("Packages.gz"));
    assert!(config.compression_rank("Packages.gz") < config.compression_rank("Packages.zst"));
    assert!(
        config.compression_rank("Packages") > config.compression_rank("Packages.zst"),
        "uncompressed must sort last"
    );
    // Unknown compression types sort after everything else.
    assert!(
        config.compression_rank("Packages.unknown") > config.compression_rank("Packages"),
        "unknown type must sort after uncompressed"
    );
}

#[test]
fn test_compression_rank_default_from_config_tree() {
    // No Order configured: the default order comes from the
    // `Acquire::CompressionTypes` tree, uncompressed last.
    let mut cfg = AptConfig::new();
    cfg.init_defaults().unwrap();
    let config = IndexTargetConfig::new_from_apt_config(&cfg, "amd64");

    for f in ["xz", "bz2", "lzma", "gz", "lz4", "zst"] {
        assert!(
            config.compression_rank(&format!("Packages.{f}")) < config.compression_rank("Packages"),
            "{f} must rank before the uncompressed fallback"
        );
    }
}

#[test]
fn test_compression_rank_partial_config_is_authoritative() {
    // An explicitly configured Order is used as-is: only the configured
    // formats rank ahead of the uncompressed fallback.
    let mut cfg = AptConfig::new();
    cfg.init_defaults().unwrap();
    cfg.set_list("Acquire::CompressionTypes::Order", "gz");
    let config = IndexTargetConfig::new_from_apt_config(&cfg, "amd64");

    assert_eq!(config.compression_rank("Packages.gz"), 0);
    assert_eq!(config.compression_rank("Packages"), 1);
    // Not-configured formats sort after the uncompressed fallback.
    assert!(config.compression_rank("Packages.xz") > config.compression_rank("Packages"));
    assert!(config.compression_rank("Packages.zst") > config.compression_rank("Packages"));
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
            items: vec![ChecksumItem {
                name: "file1".to_string(),
                size: 100,
                checksum: "abc".to_string(),
            }],
            keep_compress: false,
            msg: "msg1".to_string(),
            optional: false,
            config_key: "key1".to_string(),
            fallback_of: None,
        },
        ChecksumDownloadEntry {
            items: vec![ChecksumItem {
                name: "file2".to_string(),
                size: 100,
                checksum: "def".to_string(),
            }],
            keep_compress: false,
            msg: "msg2".to_string(),
            optional: false,
            config_key: "key2".to_string(),
            fallback_of: Some("key1".to_string()),
        },
        ChecksumDownloadEntry {
            items: vec![ChecksumItem {
                name: "file3".to_string(),
                size: 100,
                checksum: "ghi".to_string(),
            }],
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
