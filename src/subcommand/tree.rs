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
    oma_apt::{BaseDep, Package, Version},
    pkginfo::OmaDepType,
};
use spdlog::{debug, trace};

use crate::{
    CliExecuter,
    config::Config,
    error::OutputError,
    fl,
    table::oma_display_with_normal_output,
    utils::{ExitHandle, pkgnames_completions},
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
    fn execute(self, config: &Config, no_progress: bool) -> Result<ExitHandle, OutputError> {
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
    fn execute(self, _config: &Config, no_progress: bool) -> Result<ExitHandle, OutputError> {
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
            return Ok(ExitHandle::default());
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

        Ok(ExitHandle::default())
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

                        let Some(dep_installed) = dep_pkg.installed() else {
                            trace!("pkg {} is not installed, will continue", dep_pkg.name());
                            continue;
                        };

                        let key = format!("{dep_pkg}-{}", dep_installed.version());

                        if added.contains(&key) {
                            continue;
                        }

                        let pkg_rev_dep = dep_installed
                            .depends_map()
                            .values()
                            .flatten()
                            .flat_map(|x| x.iter())
                            .find(|dep| dep.name() == pkg_clone.name());

                        if pkg_rev_dep
                            .is_none_or(|pkg_rev_dep| !is_result(&pkg_installed, pkg_rev_dep))
                        {
                            continue;
                        }

                        let push = is_result(&dep_installed, dep);

                        if !push {
                            continue;
                        }

                        added.insert(key);

                        res.push(reverse_dep_tree(
                            PkgWrapper {
                                package: dep_pkg,
                                is_recommend: t == OmaDepType::Recommends,
                                comp_and_version: Some(dep_installed.version().to_string()),
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

fn is_result<'a>(pkg_installed: &Version<'a>, dep: &BaseDep<'_>) -> bool {
    debug!("{dep:#?}");

    let Some(required_ver) = dep.version() else {
        // 没有版本要求，说明要求总是符合
        return true;
    };

    let Ok(required_ver) = required_ver.parse::<debversion::Version>() else {
        return false;
    };

    let Ok(installed) = pkg_installed.version().parse::<debversion::Version>() else {
        return false;
    };

    if let Some(t) = dep.comp_type() {
        let cmp = installed.cmp(&required_ver);
        debug!("{} {pkg_installed} {cmp:?} {required_ver}", dep.name());
        return is_match_compare(t, cmp);
    }

    true
}

fn is_match_compare(t: &str, cmp: Ordering) -> bool {
    match t {
        ">" | ">>" => cmp == Ordering::Greater,
        "<" | "<<" => cmp == Ordering::Less,
        ">=" => [Ordering::Greater, Ordering::Equal].contains(&cmp),
        "<=" => [Ordering::Less, Ordering::Equal].contains(&cmp),
        "=" | "==" => Ordering::Equal == cmp,
        "!=" => Ordering::Equal != cmp,
        "" => true,
        x => unreachable!("unsupported comp type {x}"),
    }
}
