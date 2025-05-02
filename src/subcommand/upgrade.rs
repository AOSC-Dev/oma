use std::fs;
use std::fs::read_dir;
use std::path::Path;

use crate::subcommand::utils::CommitChanges;
use crate::utils::pkgnames_completions;
use ahash::HashMap;
use ahash::HashSet;
use clap_complete::ArgValueCompleter;
use oma_pm::apt::InstallOperation;
use oma_pm::apt::OmaOperation;
use serde::Deserialize;
use std::path::PathBuf;
use tracing::debug;

use apt_auth_config::AuthConfig;
use clap::Args;
use oma_pm::apt::AptConfig;
use oma_pm::apt::OmaApt;
use oma_pm::apt::OmaAptArgs;
use oma_pm::apt::Upgrade as AptUpgrade;

use oma_pm::matches::GetArchMethod;
use oma_pm::matches::PackagesMatcher;

use tracing::info;
use tracing::warn;

use crate::HTTP_CLIENT;
use crate::config::Config;
use crate::error::OutputError;
use crate::fl;
use crate::utils::dbus_check;
use crate::utils::root;

use super::utils::Refresh;
use super::utils::handle_no_result;
use super::utils::lock_oma;
use super::utils::no_check_dbus_warn;
use crate::args::CliExecuter;

#[derive(Debug, Args)]
pub(crate) struct Upgrade {
    /// Do not fix apt broken status
    #[arg(long)]
    no_fixbroken: bool,
    /// Do not fix dpkg broken status
    #[arg(long)]
    no_fix_dpkg_status: bool,
    /// Install package(s) without fsync(2)
    #[arg(long)]
    force_unsafe_io: bool,
    /// Do not refresh repository metadata
    #[arg(long)]
    no_refresh: bool,
    /// Ignore repository and package dependency issues
    #[arg(long)]
    force_yes: bool,
    /// Replace configuration file(s) in the system those shipped in the package(s) to be installed (invokes `dpkg --force-confnew`)
    #[arg(long)]
    force_confnew: bool,
    #[cfg(feature = "aosc")]
    /// Do not refresh topics manifest.json file
    #[arg(long)]
    no_refresh_topics: bool,
    /// Auto remove unnecessary package(s)
    #[arg(long)]
    autoremove: bool,
    /// Remove package(s) also remove configuration file(s), like apt purge
    #[arg(long, visible_alias = "purge")]
    remove_config: bool,
    /// Bypass confirmation prompts
    #[arg(short, long)]
    yes: bool,
    #[cfg(not(feature = "aosc"))]
    /// Do not allow removal of packages during upgrade (like `apt upgrade')
    #[arg(long)]
    no_remove: bool,
    /// Package(s) to install
    #[arg(add = ArgValueCompleter::new(pkgnames_completions))]
    packages: Vec<String>,
    /// Run oma in "dry-run" mode. Useful for testing changes and operations without making changes to the system
    #[arg(from_global)]
    dry_run: bool,
    /// Run oma do not check dbus
    #[arg(from_global)]
    no_check_dbus: bool,
    /// Set sysroot target directory
    #[arg(from_global)]
    sysroot: PathBuf,
    /// Set apt options
    #[arg(from_global)]
    apt_options: Vec<String>,
    /// Setup download threads (default as 4)
    #[arg(from_global)]
    download_threads: Option<usize>,
}

impl CliExecuter for Upgrade {
    fn execute(self, config: &Config, no_progress: bool) -> Result<i32, OutputError> {
        let Upgrade {
            no_fixbroken,
            force_unsafe_io,
            no_refresh,
            force_yes,
            force_confnew,
            #[cfg(feature = "aosc")]
            no_refresh_topics,
            autoremove,
            remove_config,
            yes,
            packages,
            dry_run,
            no_check_dbus,
            sysroot,
            apt_options,
            #[cfg(not(feature = "aosc"))]
            no_remove,
            no_fix_dpkg_status,
            download_threads,
        } = self;

        if !dry_run {
            root()?;
            lock_oma()?;
        }

        let _fds = if !no_check_dbus && !config.no_check_dbus() && !dry_run {
            Some(dbus_check(yes)?)
        } else {
            no_check_dbus_warn();
            None
        };

        let apt_config = AptConfig::new();

        let auth_config = AuthConfig::system(&sysroot)?;

        if !no_refresh {
            let sysroot = sysroot.to_string_lossy();
            let builder = Refresh::builder()
                .client(&HTTP_CLIENT)
                .dry_run(dry_run)
                .no_progress(no_progress)
                .network_thread(download_threads.unwrap_or_else(|| config.network_thread()))
                .sysroot(&sysroot)
                .config(&apt_config)
                .auth_config(&auth_config);

            #[cfg(feature = "aosc")]
            let refresh = builder
                .refresh_topics(!no_refresh_topics && !config.no_refresh_topics())
                .build();

            #[cfg(not(feature = "aosc"))]
            let refresh = builder.build();

            refresh.run()?;
        }

        if yes {
            warn!("{}", fl!("automatic-mode-warn"));
        }

        let local_debs = packages
            .iter()
            .filter(|x| x.ends_with(".deb"))
            .map(|x| x.to_owned())
            .collect::<Vec<_>>();

        let pkgs_unparse = packages.iter().map(|x| x.as_str()).collect::<Vec<_>>();

        let oma_apt_args = OmaAptArgs::builder()
            .sysroot(sysroot.to_string_lossy().to_string())
            .dpkg_force_confnew(force_confnew)
            .force_yes(force_yes)
            .yes(yes)
            .another_apt_options(apt_options)
            .dpkg_force_unsafe_io(force_unsafe_io)
            .build();

        let mut apt = OmaApt::new(
            local_debs.clone(),
            oma_apt_args.clone(),
            dry_run,
            AptConfig::new(),
        )?;

        #[cfg(feature = "aosc")]
        let mode = AptUpgrade::FullUpgrade;

        #[cfg(not(feature = "aosc"))]
        let mode = if no_remove {
            AptUpgrade::Upgrade
        } else {
            AptUpgrade::FullUpgrade
        };

        debug!("Upgrade mode is using: {:?}", mode);
        apt.upgrade(mode)?;

        let matcher = PackagesMatcher::builder()
            .cache(&apt.cache)
            .filter_candidate(true)
            .filter_downloadable_candidate(false)
            .select_dbg(false)
            .native_arch(GetArchMethod::SpecifySysroot(&sysroot))
            .build();

        let (pkgs, no_result) = matcher.match_pkgs_and_versions(pkgs_unparse.clone())?;

        handle_no_result(&sysroot, no_result, no_progress)?;

        let no_marked_install = apt.install(&pkgs, false)?;

        if !no_marked_install.is_empty() {
            for (pkg, version) in no_marked_install {
                info!(
                    "{}",
                    fl!("already-installed", name = pkg, version = version)
                );
            }
        }

        CommitChanges::builder()
            .apt(apt)
            .dry_run(dry_run)
            .no_fixbroken(no_fixbroken)
            .check_update(true)
            .no_progress(no_progress)
            .sysroot(sysroot.to_string_lossy().to_string())
            .protect_essential(config.protect_essentials())
            .yes(yes)
            .remove_config(remove_config)
            .autoremove(autoremove)
            .network_thread(download_threads.unwrap_or_else(|| config.network_thread()))
            .maybe_auth_config(Some(&auth_config))
            .fix_dpkg_status(!no_fix_dpkg_status)
            .build()
            .run()
    }
}

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
enum TopicUpdateEntry {
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

pub fn get_tum(sysroot: &Path) -> Result<Vec<TopicUpdateManifest>, OutputError> {
    let mut entries = vec![];

    for i in read_dir(sysroot.join("var/lib/apt/lists")).map_err(|e| OutputError {
        description: fl!("failed-to-operate-path"),
        source: Some(Box::new(e)),
    })? {
        let i = i.map_err(|e| OutputError {
            description: fl!("failed-to-operate-path"),
            source: Some(Box::new(e)),
        })?;

        if i.path()
            .file_name()
            .is_some_and(|x| x.to_string_lossy().ends_with("updates.json"))
        {
            let f = fs::read(i.path()).map_err(|e| OutputError {
                description: fl!("failed-to-operate-path"),
                source: Some(Box::new(e)),
            })?;

            let entry: TopicUpdateManifest = match serde_json::from_slice(&f) {
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
