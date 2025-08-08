pub mod parser;
use std::{
    fs::{self, read_dir},
    io,
    path::{Path, PathBuf},
};

use ahash::{HashMap, HashSet};
use debversion::Version;
use oma_pm_operation_type::{InstallOperation, OmaOperation};
use serde::Deserialize;
use snafu::{ResultExt, Snafu};
use tracing::warn;

use crate::parser::{VersionParseError, VersionToken, parse_version_expr};

#[derive(Deserialize, Debug)]
pub struct TopicUpdateManifest {
    #[serde(flatten)]
    pub entries: HashMap<String, TopicUpdateEntry>,
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
        #[serde(default)]
        packages: HashMap<String, Option<String>>,
        #[serde(default)]
        packages_v2: HashMap<String, Option<String>>,
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
        packages_v2: &'a HashMap<String, Option<String>>,
        name: &'a HashMap<String, String>,
        caution: Option<&'a HashMap<String, String>>,
    },
    Cumulative {
        name: &'a HashMap<String, String>,
        caution: Option<&'a HashMap<String, String>>,
        topics: &'a [String],
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

    pub fn count_packages(&self) -> usize {
        match self {
            TopicUpdateEntryRef::Conventional {
                packages,
                packages_v2,
                ..
            } => {
                if !packages_v2.is_empty() {
                    packages_v2.len()
                } else {
                    packages.len()
                }
            }
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
                packages_v2,
                ..
            } => TopicUpdateEntryRef::Conventional {
                security: *security,
                packages,
                packages_v2,
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
                topics,
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
    #[snafu(display("Parse version expr got error"))]
    ParseVersionExpr { source: VersionParseError },
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
        .map(|x| (x.name_without_arch(), (x.old_version(), x.new_version())))
        .collect::<HashMap<_, _>>();

    let remove_map = &op.remove.iter().map(|x| (x.name())).collect::<HashSet<_>>();

    for i in tum {
        'a: for (name, entry) in &i.entries {
            if let TopicUpdateEntry::Conventional {
                must_match_all,
                packages,
                packages_v2,
                ..
            } = entry
            {
                if !packages_v2.is_empty() {
                    // v2
                    'b: for (index, (pkg_name, version)) in packages_v2.iter().enumerate() {
                        if !must_match_all
                            && (install_pkg_on_topic_v2(install_map, pkg_name, version)
                                .unwrap_or(false)
                                || remove_pkg_on_topic(remove_map, pkg_name, version))
                        {
                            break 'b;
                        } else if !install_pkg_on_topic_v2(install_map, pkg_name, version)
                            .unwrap_or(false)
                            && !remove_pkg_on_topic(remove_map, pkg_name, version)
                        {
                            if *must_match_all || index == packages_v2.len() - 1 {
                                continue 'a;
                            } else {
                                continue 'b;
                            }
                        }
                    }
                } else if !packages.is_empty() {
                    // v1
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
                }

                matches.insert(name.as_str(), TopicUpdateEntryRef::from(entry));
            }
        }
    }

    for i in tum {
        for (name, entry) in &i.entries {
            if let TopicUpdateEntry::Cumulative { topics, .. } = entry
                && topics.iter().all(|x| matches.contains_key(x.as_str()))
            {
                let mut count_packages_changed_tmp = 0;

                for t in topics {
                    let t = matches.remove(t.as_str()).unwrap();

                    let TopicUpdateEntryRef::Conventional {
                        packages,
                        packages_v2,
                        ..
                    } = t
                    else {
                        unreachable!()
                    };

                    if !packages_v2.is_empty() {
                        count_packages_changed_tmp += packages_v2.len();
                    } else {
                        count_packages_changed_tmp += packages.len();
                    }
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

    matches
}

pub fn collection_all_matches_security_tum_pkgs<'a>(
    matches_tum: &HashMap<&str, TopicUpdateEntryRef<'a>>,
) -> HashMap<&'a str, &'a Option<String>> {
    let mut res = HashMap::with_hasher(ahash::RandomState::new());
    for v in matches_tum.values() {
        let TopicUpdateEntryRef::Conventional {
            security,
            packages,
            packages_v2,
            ..
        } = v
        else {
            continue;
        };

        if !*security {
            continue;
        }

        if !packages_v2.is_empty() {
            res.extend(
                packages_v2
                    .iter()
                    .map(|(pkg, version)| (pkg.as_str(), version)),
            );
        } else {
            res.extend(
                packages
                    .iter()
                    .map(|(pkg, version)| (pkg.as_str(), version)),
            );
        }
    }

    res
}

fn install_pkg_on_topic(
    install_map: &HashMap<&str, (Option<&str>, &str)>,
    pkg_name: &str,
    tum_version: &Option<String>,
) -> bool {
    let Some((_, new_version)) = install_map.get(pkg_name) else {
        return false;
    };

    let Some(tum_version) = tum_version else {
        return false;
    };

    compare_version(new_version, tum_version, VersionToken::Eq)
}

fn compare_version(install_ver: &str, tum_version: &str, op: VersionToken) -> bool {
    if let Some((prefix, suffix)) = install_ver.rsplit_once("~pre")
        && is_topic_preversion(suffix)
    {
        return compare_version_inner(prefix, tum_version, op);
    }

    compare_version_inner(install_ver, tum_version, op)
}

fn compare_version_inner(another_ver: &str, tum_version: &str, op: VersionToken<'_>) -> bool {
    let another_ver: Version = another_ver.parse().unwrap();
    let tum_version: Version = tum_version.parse().unwrap();

    match op {
        VersionToken::Eq | VersionToken::EqEq => tum_version == another_ver,
        VersionToken::NotEq => tum_version != another_ver,
        VersionToken::GtEq => another_ver >= tum_version,
        VersionToken::LtEq => another_ver <= tum_version,
        VersionToken::Gt => another_ver > tum_version,
        VersionToken::Lt => another_ver < tum_version,
        _ => unreachable!(),
    }
}

fn install_pkg_on_topic_v2(
    install_map: &HashMap<&str, (Option<&str>, &str)>,
    pkg_name: &str,
    tum_version: &Option<String>,
) -> Result<bool, TumError> {
    let Some((Some(old_version), _)) = install_map.get(pkg_name) else {
        return Ok(false);
    };

    let Some(tum_version_expr) = tum_version else {
        return Ok(false);
    };

    let tokens = parse_version_expr(tum_version_expr).context(ParseVersionExprSnafu)?;

    Ok(is_right_version(tokens, old_version))
}

fn is_right_version(tokens: Vec<VersionToken<'_>>, install_ver: &str) -> bool {
    let tokens: Vec<_> = tokens
        .into_iter()
        .map(|x| {
            if let VersionToken::VersionNumber("$VER") = x {
                VersionToken::VersionNumber(install_ver)
            } else {
                x
            }
        })
        .collect();

    let mut stack = vec![];
    let mut index = 0;

    while index < tokens.len() {
        match tokens[index] {
            VersionToken::VersionNumber(install_ver) => {
                let VersionToken::VersionNumber(tum_version) = tokens[index + 1] else {
                    unreachable!()
                };

                let b = compare_version(install_ver, tum_version, tokens[index + 2]);

                stack.push(b);
                index += 3;
            }
            VersionToken::Or => {
                let b1 = stack.pop().unwrap();
                let b2 = stack.pop().unwrap();
                stack.push(b1 || b2);
                index += 1;
            }
            VersionToken::And => {
                let b1 = stack.pop().unwrap();
                let b2 = stack.pop().unwrap();
                stack.push(b1 && b2);
                index += 1;
            }
            _ => unreachable!(),
        }
    }

    assert!(stack.len() == 1);

    stack[0]
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

#[test]
fn test_is_right_version() {
    let input_expr = "(=1.2.3 || =4.5.6) && <7.8.9";
    let ver1 = "4.5.6";
    let ver2 = "1.2.3";
    let tokens = parse_version_expr(input_expr).unwrap();
    assert!(is_right_version(tokens.clone(), ver1));
    assert!(is_right_version(tokens, ver2));

    let input_expr = "<=7.8.9";
    let ver1 = "1.2.3";
    let ver2 = "7.8.9";
    let tokens = parse_version_expr(input_expr).unwrap();
    assert!(is_right_version(tokens.clone(), ver1));
    assert!(is_right_version(tokens, ver2));

    let input_expr = ">=7.8.9";
    let ver1 = "11.0.0";
    let ver2 = "7.8.9";
    let tokens = parse_version_expr(input_expr).unwrap();
    assert!(is_right_version(tokens.clone(), ver1));
    assert!(is_right_version(tokens, ver2));

    let input_expr = "=7.8.9";
    let ver1 = "7.8.9";
    let ver2 = "1.2.3";
    let tokens = parse_version_expr(input_expr).unwrap();
    assert!(is_right_version(tokens.clone(), ver1));
    assert!(!is_right_version(tokens, ver2));
}
