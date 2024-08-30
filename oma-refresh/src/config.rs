use std::{cmp::Ordering, collections::VecDeque};

use ahash::AHashMap;
use oma_apt::config::Config;
use smallvec::{smallvec, SmallVec};
use tracing::debug;

use crate::inrelease::{file_is_compress, ChecksumItem};

fn get_config(config: &Config) -> Vec<(String, String)> {
    let Some(tree) = config.root_tree() else {
        return vec![];
    };

    let mut list = vec![];

    let mut stack = VecDeque::new();
    stack.push_back((tree, 0));

    let mut depth = 0;
    let mut name = "".to_string();

    while let Some((node, indent)) = stack.pop_back() {
        let mut k = None;
        let mut v = None;

        if let Some(item) = node.sibling() {
            stack.push_back((item, indent));
        }

        if let Some(item) = node.child() {
            stack.push_back((item, indent + 2));
        }

        if let Some(tag) = node.tag() {
            match indent.cmp(&depth) {
                Ordering::Less => {
                    let mut tmp = name.split("::").collect::<Vec<_>>();
                    for _ in 0..=1 {
                        tmp.pop();
                    }
                    name = tmp.join("::");
                    name.push_str("::");
                    name.push_str(&tag);
                }
                Ordering::Equal => {
                    let mut tmp = name.split("::").collect::<Vec<_>>();
                    tmp.pop();
                    name = tmp.join("::");
                    name.push_str("::");
                    name.push_str(&tag);
                }
                Ordering::Greater => {
                    name.push_str("::");
                    name.push_str(&tag);
                }
            }

            depth = indent;
            k = Some(name.strip_prefix("::").unwrap().to_string());
        }

        if let Some(value) = node.value() {
            v = Some(value);
        }

        if let Some(v) = k.zip(v) {
            list.push((v.0, v.1));
        }
    }

    list
}

pub fn fiilter_download_list(
    checksums: &SmallVec<[ChecksumItem; 32]>,
    config: &Config,
    archs: &[String],
    components: &[String],
    native_arch: &str,
) -> SmallVec<[ChecksumItem; 32]> {
    let mut v = smallvec![];
    let config_tree = get_config(config);

    let mut filter_entry = vec![];

    let mut archs_contains_all = vec![];
    archs_contains_all.extend_from_slice(archs);
    archs_contains_all.push("all".to_string());

    for (k, v) in config_tree {
        if k.starts_with("APT::Acquire::IndexTargets::deb::") && k.ends_with("::MetaKey") {
            for a in &archs_contains_all {
                for c in components {
                    let mut s = v.replace("$(COMPONENT)", c).replace("$(ARCHITECTURE)", a);
                    if a == native_arch {
                        s = s.replace("$(NATIVE_ARCHITECTURE)", a);
                    }
                    filter_entry.push(s);
                }
            }
        }
    }

    debug!("{:?}", filter_entry);

    let mut map: AHashMap<&str, ChecksumItem> = AHashMap::new();

    for i in checksums {
        if let Some(x) = filter_entry.iter().find(|x| i.name.starts_with(*x)) {
            if let Some(y) = map.get_mut(x.as_str()) {
                if file_is_compress(&y.name) {
                    continue;
                } else if file_is_compress(&i.name) {
                    *y = i.clone();
                }
            } else {
                map.insert(x, i.clone());
            }
        }
    }

    for (_, i) in map {
        v.push(i);
    }

    debug!("{:?}", v);

    v
}

#[test]
fn test() {
    let map = get_config(&Config::new());
    dbg!(map);
}
