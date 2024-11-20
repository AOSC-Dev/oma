use std::{borrow::Cow, env, path::Path};

use ahash::AHashMap;
use oma_apt::config::Config;
use oma_fetch::CompressFile;
use smallvec::{smallvec, SmallVec};
use tracing::debug;

use crate::inrelease::ChecksumItem;

pub fn get_config(config: &Config) -> Vec<(String, String)> {
    config
        .dump()
        .lines()
        .filter_map(|x| x.split_once(|c: char| c.is_ascii_whitespace()))
        .map(|(k, v)| {
            let mut v = v.to_string();

            while v.ends_with(";") || v.ends_with("\"") {
                v.pop();
            }

            while v.starts_with("\"") {
                v.remove(0);
            }

            (k.to_string(), v)
        })
        .collect()
}

#[derive(Debug)]
pub struct ChecksumDownloadEntry {
    pub item: ChecksumItem,
    pub keep_compress: bool,
    pub msg: String,
}

pub struct FilterDownloadList<'a> {
    pub checksums: &'a [ChecksumItem],
    pub config: &'a Config,
    pub config_tree: &'a [(String, String)],
    pub archs: &'a [Cow<'a, str>],
    pub components: &'a [String],
    pub native_arch: &'a str,
    pub is_flat: bool,
    pub is_source: bool,
}

pub fn fiilter_download_list(f: FilterDownloadList) -> SmallVec<[ChecksumDownloadEntry; 32]> {
    let FilterDownloadList {
        checksums,
        config,
        config_tree,
        archs,
        components,
        native_arch,
        is_flat,
        is_source,
    } = f;

    let mut v = smallvec![];

    let mut filter_entry = vec![];

    let mut archs_contains_all = vec![];
    archs_contains_all.extend_from_slice(archs);
    archs_contains_all.push(Cow::Borrowed("all"));

    let components = if components.is_empty() {
        &["".to_string()]
    } else {
        components
    };

    let metakey = if !is_flat {
        "::MetaKey"
    } else {
        "::flatMetaKey"
    };

    for (k, v) in config_tree {
        if is_match(is_source, k, metakey) {
            for a in &archs_contains_all {
                for c in components {
                    let s = replace_arch_and_component(v, c, a, native_arch);
                    let e = k
                        .strip_prefix("APT::")
                        .unwrap_or(k)
                        .strip_suffix(metakey)
                        .unwrap();

                    let keep_compress = config.bool(&format!("{e}::KeepCompressed"), false);
                    let default_enabled = config.bool(&format!("{e}::DefaultEnabled"), true);

                    if !default_enabled {
                        debug!("{e}::DefaultEnabled is false, so continue");
                        continue;
                    }

                    debug!("{e} keep compress: {}", keep_compress);

                    let msg = if let Some(match_msg) = config.get(&format!("{e}::ShortDescription"))
                    {
                        let mut s = replace_arch_and_component(&match_msg, c, a, native_arch);

                        if let Ok(env_lang) = env::var("LANG") {
                            let langs = get_matches_language(&env_lang);

                            if !langs.is_empty() {
                                s = s.replace("$(LANGUAGE)", langs[0]);
                            }
                        }

                        s
                    } else {
                        "Other".to_string()
                    };

                    let mut list = vec![];

                    if v.contains("$(LANGUAGE)") {
                        if let Ok(env_lang) = env::var("LANG") {
                            let langs = get_matches_language(&env_lang);

                            for i in langs {
                                list.push((
                                    s.replace("$(LANGUAGE)", i),
                                    keep_compress,
                                    msg.clone(),
                                ));
                            }
                        }
                    }

                    if list.is_empty() {
                        filter_entry.push((s, keep_compress, msg.clone()));
                    } else {
                        filter_entry.extend(list);
                    }
                }
            }
        }
    }

    debug!("{:?}", filter_entry);

    let mut map: AHashMap<&str, ChecksumDownloadEntry> = AHashMap::new();

    for i in checksums {
        if let Some(x) = filter_entry.iter().find(|x| {
            let path = Path::new(&i.name);
            let path = path.with_extension("");
            let path = path.to_string_lossy();
            path == x.0
        }) {
            if let Some(y) = map.get_mut(x.0.as_str()) {
                if compress_file(&y.item.name) > compress_file(&i.name) {
                    continue;
                } else {
                    *y = ChecksumDownloadEntry {
                        item: i.clone(),
                        keep_compress: x.1,
                        msg: x.2.clone(),
                    }
                }
            } else {
                map.insert(
                    &x.0,
                    ChecksumDownloadEntry {
                        item: i.clone(),
                        keep_compress: x.1,
                        msg: x.2.clone(),
                    },
                );
            }
        }
    }

    for (_, i) in map {
        v.push(i);
    }

    debug!("{:?}", v);

    v
}

fn is_match(is_source: bool, key: &str, metakey: &str) -> bool {
    let deb = (key.starts_with("APT::Acquire::IndexTargets::deb::")
        || key.starts_with("Acquire::IndexTargets::deb::"))
        && key.ends_with(metakey);

    let deb_src = (key.starts_with("APT::Acquire::IndexTargets::deb-src::")
        || key.starts_with("Acquire::IndexTargets::deb-src::"))
        && key.ends_with(metakey);

    if is_source {
        deb || deb_src
    } else {
        deb
    }
}

fn get_matches_language(env_lang: &str) -> Vec<&str> {
    let mut langs = vec![];
    let env_lang = env_lang.split_once('.').map(|x| x.0).unwrap_or(env_lang);

    let lang = if env_lang == "C" { "en" } else { env_lang };

    langs.push(lang);

    // en_US.UTF-8 => en
    if let Some((a, _)) = lang.split_once('_') {
        langs.push(a);
    }

    langs
}

fn replace_arch_and_component(
    input: &str,
    component: &str,
    arch: &str,
    native_arch: &str,
) -> String {
    let mut output = input
        .replace("$(COMPONENT)", component)
        .replace("$(ARCHITECTURE)", arch);

    if arch == native_arch {
        output = output.replace("$(NATIVE_ARCHITECTURE)", arch);
    }

    output
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
fn test() {
    let map = get_config(&Config::new());
    dbg!(map);
}

#[test]
fn test_replace_arch_and_component() {
    let input = "$(COMPONENT)/Contents-$(ARCHITECTURE)";
    assert_eq!(
        replace_arch_and_component(input, "main", "amd64", "amd64"),
        "main/Contents-amd64"
    );

    let input = "Contents-$(ARCHITECTURE)";
    assert_eq!(
        replace_arch_and_component(input, "main", "amd64", "amd64"),
        "Contents-amd64"
    );

    let input = "$(COMPONENT)/dep11/Components-$(NATIVE_ARCHITECTURE).yml";
    assert_eq!(
        replace_arch_and_component(input, "main", "amd64", "amd64"),
        "main/dep11/Components-amd64.yml"
    );
    assert_eq!(
        replace_arch_and_component(input, "main", "amd64", "arm64"),
        "main/dep11/Components-$(NATIVE_ARCHITECTURE).yml"
    );
}

#[test]
fn test_get_matches_language() {
    assert_eq!(get_matches_language("C"), vec!["en"]);
    assert_eq!(get_matches_language("zh_CN.UTF-8"), vec!["zh_CN", "zh"]);
    assert_eq!(get_matches_language("en_US.UTF-8"), vec!["en_US", "en"]);
}
