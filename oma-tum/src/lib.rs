use std::{
    fs::{self, read_dir},
    io,
    path::{Path, PathBuf},
};

use ahash::{HashMap, HashSet};
use oma_pm_operation_type::{InstallOperation, OmaOperation};
use serde::Deserialize;
use snafu::{ResultExt, Snafu};
use tracing::warn;

#[derive(Deserialize, Debug)]
pub struct TopicUpdateManifest {
    #[serde(flatten)]
    entries: HashMap<String, TopicUpdateEntry>,
}

#[inline]
const fn must_match_all_default() -> bool {
    true
}

#[derive(Deserialize, Debug)]
#[serde(tag = "type")]
pub enum TopicUpdateEntry {
    #[serde(rename = "conventional")]
    Conventional {
        security: bool,
        packages: HashMap<String, Option<String>>,
        name: HashMap<String, String>,
        caution: Option<HashMap<String, String>>,
        #[serde(default = "must_match_all_default")]
        must_match_all: bool,
    },
    #[serde(rename = "cumulative")]
    Cumulative {
        name: HashMap<String, String>,
        caution: Option<HashMap<String, String>>,
        topics: Vec<String>,
        #[serde(default)]
        security: bool,
    },
}

#[derive(Debug)]
pub enum TopicUpdateEntryRef<'a> {
    Conventional {
        security: bool,
        packages: &'a HashMap<String, Option<String>>,
        name: &'a HashMap<String, String>,
        caution: Option<&'a HashMap<String, String>>,
    },
    Cumulative {
        name: &'a HashMap<String, String>,
        caution: Option<&'a HashMap<String, String>>,
        _topics: &'a [String],
        count_packages_changed: usize,
        security: bool,
    },
}

impl TopicUpdateEntryRef<'_> {
    pub fn is_security(&self) -> bool {
        match self {
            TopicUpdateEntryRef::Conventional { security, .. } => *security,
            TopicUpdateEntryRef::Cumulative { security, .. } => *security,
        }
    }

    #[allow(dead_code)]
    pub fn count_packages(&self) -> usize {
        match self {
            TopicUpdateEntryRef::Conventional { packages, .. } => packages.len(),
            TopicUpdateEntryRef::Cumulative {
                count_packages_changed,
                ..
            } => *count_packages_changed,
        }
    }
}

impl<'a> From<&'a TopicUpdateEntry> for TopicUpdateEntryRef<'a> {
    fn from(value: &'a TopicUpdateEntry) -> Self {
        match value {
            TopicUpdateEntry::Conventional {
                security,
                packages,
                name,
                caution,
                ..
            } => TopicUpdateEntryRef::Conventional {
                security: *security,
                packages,
                name,
                caution: caution.as_ref(),
            },
            TopicUpdateEntry::Cumulative {
                name,
                caution,
                topics,
                security,
            } => TopicUpdateEntryRef::Cumulative {
                name,
                caution: caution.as_ref(),
                _topics: topics,
                count_packages_changed: 0,
                security: *security,
            },
        }
    }
}

#[derive(Debug, Snafu)]
pub enum TumError {
    #[snafu(display("Failed to read apt list dir"))]
    ReadAptListDir { source: io::Error },
    #[snafu(display("Failed to read dir entry"))]
    ReadDirEntry { source: io::Error },
    #[snafu(display("Failed to read file: {}", path.display()))]
    ReadFile { path: PathBuf, source: io::Error },
}

pub fn get_tum(sysroot: &Path) -> Result<Vec<TopicUpdateManifest>, TumError> {
    let mut entries = vec![];

    for i in read_dir(sysroot.join("var/lib/apt/lists")).context(ReadAptListDirSnafu)? {
        let i = i.context(ReadDirEntrySnafu)?;

        if i.path()
            .file_name()
            .is_some_and(|x| x.to_string_lossy().ends_with("updates.json"))
        {
            let f = fs::read(i.path()).context(ReadFileSnafu {
                path: i.path().to_path_buf(),
            })?;

            let entry = match parse_single_tum(&f) {
                Ok(entry) => entry,
                Err(e) => {
                    warn!("Parse {} got error: {}", i.path().display(), e);
                    continue;
                }
            };

            entries.push(entry);
        }
    }

    Ok(entries)
}

pub fn parse_single_tum(bytes: &[u8]) -> Result<TopicUpdateManifest, serde_json::Error> {
    serde_json::from_slice(bytes)
}

pub fn get_matches_tum<'a>(
    tum: &'a [TopicUpdateManifest],
    op: &OmaOperation,
) -> HashMap<&'a str, TopicUpdateEntryRef<'a>> {
    let mut matches = HashMap::with_hasher(ahash::RandomState::new());

    let install_map = &op
        .install
        .iter()
        .filter(|x| *x.op() != InstallOperation::Downgrade)
        .map(|x| (x.name_without_arch(), x.new_version()))
        .collect::<HashMap<_, _>>();

    let remove_map = &op.remove.iter().map(|x| (x.name())).collect::<HashSet<_>>();

    for i in tum {
        'a: for (name, entry) in &i.entries {
            if let TopicUpdateEntry::Conventional {
                must_match_all,
                packages,
                ..
            } = entry
            {
                'b: for (index, (pkg_name, version)) in packages.iter().enumerate() {
                    if !must_match_all
                        && (install_pkg_on_topic(install_map, pkg_name, version)
                            || remove_pkg_on_topic(remove_map, pkg_name, version))
                    {
                        break 'b;
                    } else if !install_pkg_on_topic(install_map, pkg_name, version)
                        && !remove_pkg_on_topic(remove_map, pkg_name, version)
                    {
                        if *must_match_all || index == packages.len() - 1 {
                            continue 'a;
                        } else {
                            continue 'b;
                        }
                    }
                }
                matches.insert(name.as_str(), TopicUpdateEntryRef::from(entry));
            }
        }
    }

    for i in tum {
        for (name, entry) in &i.entries {
            if let TopicUpdateEntry::Cumulative { topics, .. } = entry {
                if topics.iter().all(|x| matches.contains_key(x.as_str())) {
                    let mut count_packages_changed_tmp = 0;

                    for t in topics {
                        let t = matches.remove(t.as_str()).unwrap();

                        let TopicUpdateEntryRef::Conventional { packages, .. } = t else {
                            unreachable!()
                        };

                        count_packages_changed_tmp += packages.len();
                    }

                    let mut entry = TopicUpdateEntryRef::from(entry);

                    let TopicUpdateEntryRef::Cumulative {
                        count_packages_changed,
                        ..
                    } = &mut entry
                    else {
                        unreachable!()
                    };

                    *count_packages_changed = count_packages_changed_tmp;
                    matches.insert(name.as_str(), entry);
                }
            }
        }
    }

    matches
}

fn install_pkg_on_topic(
    install_map: &HashMap<&str, &str>,
    pkg_name: &str,
    tum_version: &Option<String>,
) -> bool {
    let install_ver = match install_map.get(pkg_name) {
        Some(v) => v,
        None => return false,
    };

    let tum_version = match tum_version {
        Some(v) => v,
        None => return false,
    };

    if let Some((prefix, suffix)) = install_ver.rsplit_once("~pre") {
        if is_topic_preversion(suffix) {
            return tum_version == prefix;
        } else {
            return tum_version == install_ver;
        }
    }

    tum_version == install_ver
}

fn is_topic_preversion(suffix: &str) -> bool {
    if suffix.len() < 16 {
        return false;
    }

    for (idx, c) in suffix.chars().enumerate() {
        if idx == 8 && c != 'T' {
            return false;
        } else if idx == 15 {
            if c != 'Z' {
                return false;
            }
            break;
        } else if !c.is_ascii_digit() && idx != 8 {
            return false;
        }
    }

    true
}

fn remove_pkg_on_topic(
    remove_map: &HashSet<&str>,
    pkg_name: &str,
    version: &Option<String>,
) -> bool {
    version.is_none() && remove_map.contains(pkg_name)
}

#[test]
fn test_is_topic_preversion() {
    let suffix = "20241213T090405Z";
    let res = is_topic_preversion(suffix);
    assert!(res);
}
