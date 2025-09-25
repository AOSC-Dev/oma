use std::{
    cmp::Ordering,
    collections::HashSet,
    fmt::Display,
    io::{Write, stdout},
    path::PathBuf,
    sync::LazyLock,
};

use clap::Args;
use clap_complete::ArgValueCompleter;
use dialoguer::console::style;
use oma_pm::{
    apt::{AptConfig, OmaApt, OmaAptArgs},
    matches::{GetArchMethod, PackagesMatcher},
    oma_apt::{Package, Version},
    pkginfo::OmaDepType,
};
use tracing::{debug, trace};

use crate::{
    CliExecuter, config::Config, error::OutputError, fl, table::oma_display_with_normal_output,
    utils::pkgnames_completions,
};

use super::utils::{create_progress_spinner, handle_no_result};

use termtree::Tree as TermTree;

static DEPTH_HELP: LazyLock<String> = LazyLock::new(|| {
    fl!(
        "clap-tree-depth-help",
        memory_warn = crate::args::dangerous_color(fl!("clap-memory-warn"))
    )
});

#[derive(Debug, Args)]
pub struct Tree {
    /// Query Package(s) name
    #[arg(required = true, add = ArgValueCompleter::new(pkgnames_completions), help = fl!("clap-tree-packages-help"))]
    #[arg(help_heading = &**crate::args::ARG_HELP_HEADING_MUST)]
    packages: Vec<String>,
    /// Invert the tree direction and focus on the given package
    #[arg(short, long, help = fl!("clap-tree-reverse-help"))]
    reverse: bool,
    /// Maximum display depth of the dependency tree
    #[arg(short, long, default_value_t = 5, value_parser = clap::value_parser!(u8).range(1..=5), help = &**DEPTH_HELP)]
    depth: u8,
    /// Set sysroot target directory
    #[arg(from_global, help = fl!("clap-sysroot-help"))]
    sysroot: PathBuf,
    /// Output result to stdout, not pager
    #[arg(long, help = fl!("clap-no-pager-help"))]
    no_pager: bool,
}

#[derive(Debug, Args)]
pub struct Why {
    /// Query Package(s) name
    #[arg(required = true, add = ArgValueCompleter::new(pkgnames_completions), help = fl!("clap-tree-packages-help"))]
    #[arg(help_heading = &**crate::args::ARG_HELP_HEADING_MUST)]
    packages: Vec<String>,
    /// Maximum display depth of the dependency tree
    #[arg(short, long, default_value_t = 5, value_parser = clap::value_parser!(u8).range(1..=5), help = &**DEPTH_HELP)]
    depth: u8,
    /// Set sysroot target directory
    #[arg(from_global, help = fl!("clap-sysroot-help"))]
    sysroot: PathBuf,
    /// Output result to stdout, not pager
    #[arg(long, help = fl!("clap-no-pager-help"))]
    no_pager: bool,
}

impl From<Why> for Tree {
    fn from(value: Why) -> Self {
        let Why {
            packages,
            depth,
            sysroot,
            no_pager,
        } = value;

        Self {
            packages,
            reverse: true,
            depth,
            sysroot,
            no_pager,
        }
    }
}

impl CliExecuter for Why {
    fn execute(self, config: &Config, no_progress: bool) -> Result<i32, OutputError> {
        Tree::from(self).execute(config, no_progress)
    }
}

struct PkgWrapper<'a> {
    package: Package<'a>,
    is_recommend: bool,
    comp_and_version: Option<String>,
}

impl Display for PkgWrapper<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if self.is_recommend {
            write!(f, "{}", style("[RECOMMEND] ").blue())?;
        }

        f.write_str(&self.package.fullname(true))?;

        if let Some(comp_and_version) = &self.comp_and_version {
            write!(f, " {}", style(format!("({comp_and_version})")).yellow())?;
        } else if let Some(cand) = self.package.candidate() {
            write!(f, " {}", style(format!("({})", cand.version())).yellow())?;
        }

        Ok(())
    }
}

impl CliExecuter for Tree {
    fn execute(self, _config: &Config, no_progress: bool) -> Result<i32, OutputError> {
        let Tree {
            packages,
            reverse: invert,
            depth: limit,
            sysroot,
            no_pager,
        } = self;

        let apt = OmaApt::new(
            vec![],
            OmaAptArgs::builder().build(),
            false,
            AptConfig::new(),
        )?;

        let matcher = PackagesMatcher::builder()
            .cache(&apt.cache)
            .native_arch(GetArchMethod::SpecifySysroot(&sysroot));

        let matcher = if invert {
            let matcher = matcher.filter_candidate(false);
            matcher.build()
        } else {
            matcher.build()
        };

        let (pkgs, no_result) =
            matcher.match_pkgs_and_versions(packages.iter().map(|x| x.as_str()))?;

        handle_no_result(no_result, no_progress)?;

        let mut res = vec![];

        let pb = create_progress_spinner(no_progress || no_pager, fl!("loading-tree"));

        for p in pkgs {
            let depth = 1;
            let tree = if !invert {
                dep_tree(
                    PkgWrapper {
                        package: Package::new(&apt.cache, p.raw_pkg),
                        is_recommend: false,
                        comp_and_version: None,
                    },
                    &apt,
                    depth,
                    limit,
                )
            } else {
                if !p.version_raw.is_installed() {
                    continue;
                }
                reverse_dep_tree(
                    PkgWrapper {
                        package: Package::new(&apt.cache, p.raw_pkg),
                        is_recommend: false,
                        comp_and_version: None,
                    },
                    &apt,
                    depth,
                    limit,
                )
            };

            if no_pager {
                writeln!(stdout(), "{tree}").ok();
            } else {
                res.push(tree);
            }
        }

        if let Some(pb) = pb {
            pb.inner.finish_and_clear();
        }

        if no_pager {
            return Ok(0);
        }

        let res = res
            .into_iter()
            .map(|t| t.to_string())
            .collect::<Vec<_>>()
            .join("\n");

        let res = res.lines().collect::<Vec<_>>();

        let mut pager = oma_display_with_normal_output(false, res.len())?;
        let mut w = pager.get_writer().map_err(|e| OutputError {
            description: "Failed to get writer".to_string(),
            source: Some(Box::new(e)),
        })?;

        writeln!(w, "{}", res.join("\n")).ok();
        drop(w);

        pager.wait_for_exit().ok();

        Ok(0)
    }
}

fn dep_tree<'a>(
    pkg: PkgWrapper<'a>,
    apt: &'a OmaApt,
    depth: u8,
    limit: u8,
) -> TermTree<PkgWrapper<'a>> {
    let cand = pkg.package.candidate();

    let mut res = TermTree::new(pkg);

    let Some(cand) = cand else {
        return res;
    };

    if depth > limit {
        return res;
    }

    let deps = cand.depends_map();

    for (t, deps) in deps {
        let t = t.into();
        match t {
            OmaDepType::Depends | OmaDepType::PreDepends | OmaDepType::Recommends => {
                for dep in deps {
                    if let Some(dep) = apt.cache.get(dep.first().name()) {
                        res.push(dep_tree(
                            PkgWrapper {
                                package: dep,
                                is_recommend: t == OmaDepType::Recommends,
                                comp_and_version: None,
                            },
                            apt,
                            depth + 1,
                            limit,
                        ));
                    }
                }
            }
            _ => continue,
        }
    }

    res
}

fn reverse_dep_tree<'a>(
    pkg: PkgWrapper<'a>,
    apt: &'a OmaApt,
    depth: u8,
    limit: u8,
) -> TermTree<PkgWrapper<'a>> {
    let pkg_clone = pkg.package.clone();
    let pkg_installed = pkg_clone.installed().unwrap();
    let rdep = pkg_clone.rdepends();

    let mut res = TermTree::new(pkg);

    if depth > limit {
        return res;
    }

    for (t, deps_group) in rdep {
        let t = t.into();
        match t {
            OmaDepType::Depends | OmaDepType::PreDepends | OmaDepType::Recommends => {
                let mut added = HashSet::new();
                for deps in deps_group {
                    for dep in deps.iter() {
                        let Some(dep_pkg) = apt.cache.get(dep.name()) else {
                            trace!(
                                "dep {} does not exist on apt cache, will continue",
                                dep.name()
                            );
                            continue;
                        };

                        let Some(installed_version) = dep_pkg.installed() else {
                            trace!("pkg {} is not installed, will continue", dep_pkg.name());
                            continue;
                        };

                        let key = format!("{dep_pkg}-{}", installed_version.version());

                        if added.contains(&key) {
                            continue;
                        }

                        let pkg_rev_dep = installed_version
                            .depends_map()
                            .values()
                            .flatten()
                            .flat_map(|x| x.iter())
                            .find(|dep| dep.name() == pkg_clone.name());

                        match pkg_rev_dep {
                            Some(pkg_rev_dep) if !is_result(&pkg_installed, pkg_rev_dep, false) => {
                                continue;
                            }
                            None => continue,
                            _ => {}
                        };

                        let push = is_result(&installed_version, dep, true);

                        if !push {
                            continue;
                        }

                        added.insert(key);

                        res.push(reverse_dep_tree(
                            PkgWrapper {
                                package: dep_pkg,
                                is_recommend: t == OmaDepType::Recommends,
                                comp_and_version: Some(installed_version.version().to_string()),
                            },
                            apt,
                            depth + 1,
                            limit,
                        ));
                    }
                }
            }
            _ => continue,
        }
    }

    res
}

fn is_result<'a>(
    pkg_installed: &Version<'a>,
    dep: &oma_pm::oma_apt::BaseDep<'_>,
    is_rev: bool,
) -> bool {
    let required_ver = if is_rev {
        let ver = unsafe { dep.parent_ver() }.version().to_string();
        debug!("require dep {} ver {}", dep.name(), ver);
        ver
    } else {
        match dep.target_ver() {
            Ok(ver) => {
                debug!("require dep {} ver {}", dep.name(), ver);
                ver.to_string()
            }
            Err(_) => return true,
        }
    };

    let Ok(required_ver) = required_ver.parse::<debversion::Version>() else {
        return false;
    };

    let Ok(installed) = pkg_installed.version().parse::<debversion::Version>() else {
        return false;
    };

    let mut push = None;

    if let Some(t) = dep.comp_type() {
        let t = match t {
            ">" | ">>" => vec![Ordering::Greater],
            "<" | "<<" => vec![Ordering::Less],
            ">=" => vec![Ordering::Greater, Ordering::Equal],
            "<=" => vec![Ordering::Less, Ordering::Equal],
            "=" | "==" => vec![Ordering::Equal],
            "!=" => {
                push = Some(required_ver != installed);
                vec![]
            }
            "" => {
                push = Some(true);
                vec![]
            }
            x => unreachable!("unsupported comp type {x}"),
        };

        debug!("{} {:?} {}", dep.name(), t, required_ver);

        let cmp = installed.cmp(&required_ver);

        debug!("{} {pkg_installed} {cmp:?} {required_ver}", dep.name());

        if !t.contains(&cmp) || push.is_some_and(|x| !x) {
            return false;
        }
    }

    true
}
