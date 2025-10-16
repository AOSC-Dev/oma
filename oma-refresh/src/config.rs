use std::{borrow::Cow, cmp::Ordering, collections::HashMap, path::Path};

use ahash::AHashMap;
use aho_corasick::AhoCorasick;
#[cfg(feature = "apt")]
use oma_apt::config::{Config, ConfigTree};
use oma_fetch::CompressFile;
use once_cell::sync::OnceCell;
use tracing::debug;

use crate::{db::RefreshError, inrelease::ChecksumItem};

static COMPRESSION_ORDER: OnceCell<Vec<CompressFileWrapper>> = OnceCell::new();

#[derive(Debug, Eq, PartialEq)]
struct CompressFileWrapper {
    compress_file: CompressFile,
}

impl From<&str> for CompressFileWrapper {
    fn from(value: &str) -> Self {
        match value {
            "xz" => CompressFileWrapper {
                compress_file: CompressFile::Xz,
            },
            "bz2" => CompressFileWrapper {
                compress_file: CompressFile::Bz2,
            },
            "lzma" => CompressFileWrapper {
                compress_file: CompressFile::Lzma,
            },
            "gz" => CompressFileWrapper {
                compress_file: CompressFile::Gzip,
            },
            "lz4" => CompressFileWrapper {
                compress_file: CompressFile::Lz4,
            },
            "zst" => CompressFileWrapper {
                compress_file: CompressFile::Zstd,
            },
            x => {
                if !x.is_ascii() {
                    debug!("{x} format is not compress format");
                }

                CompressFileWrapper {
                    compress_file: CompressFile::Nothing,
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

#[cfg(feature = "apt")]
impl Ord for CompressFileWrapper {
    fn cmp(&self, other: &Self) -> Ordering {
        let config = Config::new();
        let t = COMPRESSION_ORDER.get_or_init(|| {
            config
                .get_compression_types()
                .iter()
                .map(|t| CompressFileWrapper::from(t.as_str()))
                .collect::<Vec<_>>()
        });

        let self_pos = t.iter().position(|x| x == self).unwrap();
        let other_pos = t.iter().position(|x| x == other).unwrap();

        other_pos.cmp(&self_pos)
    }
}

#[cfg(not(feature = "apt"))]
impl Ord for CompressFileWrapper {
    fn cmp(&self, other: &Self) -> Ordering {
        let t = COMPRESSION_ORDER.get_or_init(|| {
            vec!["zst", "xz", "bz2", "lzma", "gz", "lz4", "uncompressed"]
                .into_iter()
                .map(CompressFileWrapper::from)
                .collect::<Vec<_>>()
        });

        let self_pos = t.iter().position(|x| x == self).unwrap();
        let other_pos = t.iter().position(|x| x == other).unwrap();

        other_pos.cmp(&self_pos)
    }
}

impl From<CompressFile> for CompressFileWrapper {
    fn from(value: CompressFile) -> Self {
        Self {
            compress_file: value,
        }
    }
}

#[cfg(feature = "apt")]
fn modify_result(
    tree: ConfigTree,
    res: &mut HashMap<String, HashMap<String, String>>,
    root_path: String,
) {
    use std::collections::VecDeque;
    let mut stack = VecDeque::new();
    stack.push_back((tree, root_path));

    let mut first = true;

    while let Some((node, tree_path)) = stack.pop_back() {
        // 跳过要遍历根节点的相邻节点
        if !first && let Some(entry) = node.sibling() {
            stack.push_back((entry, tree_path.clone()));
        }

        let Some(tag) = node.tag() else {
            continue;
        };

        if let Some(entry) = node.child() {
            stack.push_back((entry, format!("{tree_path}::{tag}")));
        }

        if let Some((k, v)) = node.tag().zip(node.value()) {
            res.entry(tree_path).or_default().insert(k, v);
        }

        first = false;
    }
}

#[cfg(feature = "apt")]
pub fn get_tree(config: &Config, key: &str) -> Vec<(String, HashMap<String, String>)> {
    let mut res = HashMap::new();
    let tree = config.tree(key);

    let Some(tree) = tree else {
        return vec![];
    };

    modify_result(
        tree,
        &mut res,
        key.rsplit_once("::")
            .map(|x| x.0.to_string())
            .unwrap_or_else(|| key.to_string()),
    );

    res.into_iter().collect::<Vec<_>>()
}

pub struct IndexTargetConfig<'a> {
    deb: Vec<HashMap<String, String>>,
    deb_src: Vec<HashMap<String, String>>,
    replacer: AhoCorasick,
    native_arch: &'a str,
    langs: Vec<String>,
}

impl<'a> IndexTargetConfig<'a> {
    #[cfg(feature = "apt")]
    pub fn new_from_apt_config(config: &Config, native_arch: &'a str) -> Self {
        Self::new(
            get_index_target_tree(config, "Acquire::IndexTargets::deb"),
            get_index_target_tree(config, "Acquire::IndexTargets::deb-src"),
            native_arch,
        )
    }

    pub fn new(
        deb: Vec<HashMap<String, String>>,
        deb_src: Vec<HashMap<String, String>>,
        native_arch: &'a str,
    ) -> Self {
        let locales = sys_locale::get_locales();
        let langs = get_matches_language(locales);

        Self {
            deb,
            deb_src,
            replacer: AhoCorasick::new([
                "$(ARCHITECTURE)",
                "$(COMPONENT)",
                "$(LANGUAGE)",
                "$(NATIVE_ARCHITECTURE)",
            ])
            .unwrap(),
            native_arch,
            langs,
        }
    }

    pub fn get_download_list(
        &self,
        checksums: &[ChecksumItem],
        is_source: bool,
        is_flat: bool,
        archs: Vec<&str>,
        components: &[String],
    ) -> Result<Vec<ChecksumDownloadEntry>, RefreshError> {
        let key = if is_flat { "flatMetaKey" } else { "MetaKey" };
        let mut res_map: AHashMap<String, Vec<ChecksumDownloadEntry>> = AHashMap::new();
        let tree = if is_source { &self.deb_src } else { &self.deb };

        let mut archs = archs;

        if !archs.contains(&"all") {
            archs.push("all");
        }

        for c in checksums {
            'a: for (template, config) in tree.iter().map(|x| (x.get(key), x)) {
                let Some(template) = template else {
                    debug!("{:?} config has no key: {}", config, key);
                    continue 'a;
                };

                if is_flat {
                    flat_repo_template_match(&mut res_map, c, config, template);
                } else {
                    self.normal_repo_match(&archs, components, &mut res_map, c, config, template);
                }
            }
        }

        let mut res = vec![];

        for (_, v) in &mut res_map {
            v.sort_unstable_by(|a, b| {
                compress_file(&a.item.name).cmp(&compress_file(&b.item.name))
            });
            if v[0].item.size == 0 {
                continue;
            }
            res.push(v.last().unwrap().to_owned());
        }

        Ok(res)
    }

    fn normal_repo_match(
        &self,
        archs: &[&str],
        components: &[String],
        res_map: &mut AHashMap<String, Vec<ChecksumDownloadEntry>>,
        c: &ChecksumItem,
        config: &HashMap<String, String>,
        template: &str,
    ) {
        for a in archs {
            for comp in components {
                for l in &self.langs {
                    if self.template_is_match(template, &c.name, a, comp, l) {
                        let name_without_ext = uncompress_file_name(&c.name).to_string();

                        res_map
                            .entry(name_without_ext)
                            .or_default()
                            .push(ChecksumDownloadEntry {
                                item: c.to_owned(),
                                keep_compress: config
                                    .get("KeepCompressed")
                                    .and_then(|x| x.parse::<bool>().ok())
                                    .unwrap_or(false),
                                msg: config
                                    .get("ShortDescription")
                                    .map(|x| {
                                        self.replacer
                                            .replace_all(x, &[*a, comp, l, self.native_arch])
                                    })
                                    .unwrap_or_else(|| "Other".to_string()),
                            });
                    }
                }
            }
        }
    }

    fn template_is_match(
        &self,
        template: &str,
        target: &str,
        arch: &str,
        component: &str,
        lang: &str,
    ) -> bool {
        let name = uncompress_file_name(target);

        name == self
            .replacer
            .replace_all(template, &[arch, component, lang, self.native_arch])
    }
}

fn flat_repo_template_match(
    res_map: &mut AHashMap<String, Vec<ChecksumDownloadEntry>>,
    c: &ChecksumItem,
    config: &HashMap<String, String>,
    template: &str,
) {
    let name_without_ext = uncompress_file_name(&c.name).to_string();
    if *template == name_without_ext {
        res_map
            .entry(name_without_ext)
            .or_default()
            .push(ChecksumDownloadEntry {
                item: c.to_owned(),
                keep_compress: config
                    .get("KeepCompressed")
                    .and_then(|x| x.parse::<bool>().ok())
                    .unwrap_or(false),
                msg: config
                    .get("ShortDescription")
                    .map(|x| x.to_owned())
                    .unwrap_or_else(|| "Other".to_string()),
            });
    }
}

fn uncompress_file_name(target: &str) -> Cow<'_, str> {
    if compress_file(target) == CompressFile::Nothing.into() {
        Cow::Borrowed(target)
    } else {
        let compress_target_without_ext = Path::new(target).with_extension("");
        let compress_target_without_ext = compress_target_without_ext.to_string_lossy().to_string();
        compress_target_without_ext.into()
    }
}

#[cfg(feature = "apt")]
fn get_index_target_tree(config: &Config, key: &str) -> Vec<HashMap<String, String>> {
    get_tree(config, key)
        .into_iter()
        .map(|x| x.1)
        .filter(|x| {
            x.get("DefaultEnabled")
                .and_then(|x| x.parse::<bool>().ok())
                .unwrap_or(true)
        })
        .collect::<Vec<_>>()
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ChecksumDownloadEntry {
    pub item: ChecksumItem,
    pub keep_compress: bool,
    pub msg: String,
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
        compress_file: CompressFile::from(
            Path::new(name)
                .extension()
                .map(|x| x.to_string_lossy())
                .unwrap_or_default()
                .to_string()
                .as_str(),
        ),
    }
}

#[cfg(feature = "apt")]
#[test]
fn test_compression_order() {
    use crate::test::TEST_LOCK;

    let _lock = TEST_LOCK.lock().unwrap();
    let config = Config::new();

    config.set_vector(
        "Acquire::CompressionTypes::Order",
        &vec!["zst", "xz", "bz2", "lzma", "gz", "lz4"],
    );

    let mut types = config
        .get_compression_types()
        .iter()
        .map(|t| CompressFileWrapper::from(t.as_str()))
        .collect::<Vec<_>>();

    types.sort_unstable();
    types.reverse();

    assert_eq!(
        types,
        vec![
            CompressFile::Zstd,
            CompressFile::Xz,
            CompressFile::Bz2,
            CompressFile::Lzma,
            CompressFile::Gzip,
            CompressFile::Lz4,
            CompressFile::Nothing
        ]
        .into_iter()
        .map(|x| x.into())
        .collect::<Vec<CompressFileWrapper>>()
    );
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

#[cfg(feature = "apt")]
#[test]
fn test_get_tree() {
    use crate::test::TEST_LOCK;

    let _lock = TEST_LOCK.lock().unwrap();
    let t = get_tree(&Config::new(), "Acquire::IndexTargets::deb");
    assert!(t.iter().any(|x| x.0.contains("::deb::")));
    assert!(t.iter().all(|x| !x.0.contains("::deb-src::")))
}
