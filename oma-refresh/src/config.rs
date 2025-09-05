use std::{borrow::Cow, collections::HashMap, path::Path};

use ahash::AHashMap;
use aho_corasick::AhoCorasick;
#[cfg(feature = "apt")]
use oma_apt_config::Config;
use oma_fetch::CompressFile;
use tracing::debug;

use crate::{db::RefreshError, inrelease::ChecksumItem};

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
    if compress_file(target) == CompressFile::Nothing {
        Cow::Borrowed(target)
    } else {
        let compress_target_without_ext = Path::new(target).with_extension("");
        let compress_target_without_ext = compress_target_without_ext.to_string_lossy().to_string();
        compress_target_without_ext.into()
    }
}

#[cfg(feature = "apt")]
fn get_index_target_tree(config: &Config, key: &str) -> Vec<HashMap<String, String>> {
    oma_apt_config::get_tree(config, key)
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

fn compress_file(name: &str) -> CompressFile {
    CompressFile::from(
        Path::new(name)
            .extension()
            .map(|x| x.to_string_lossy())
            .unwrap_or_default()
            .to_string()
            .as_str(),
    )
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
    use oma_apt_config::get_tree;

    let t = get_tree(&Config::new(), "Acquire::IndexTargets::deb");
    assert!(t.iter().any(|x| x.0.contains("::deb::")));
    assert!(t.iter().all(|x| !x.0.contains("::deb-src::")))
}
