use std::{
    cmp::Ordering,
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
use tracing::debug;

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
            .native_arch(GetArchMethod::SpecifySysroot(&sysroot))
            .build();

        let (pkgs, no_result) =
            matcher.match_pkgs_and_versions(packages.iter().map(|x| x.as_str()))?;

        handle_no_result(&sysroot, no_result, no_progress)?;

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
    let rdep = pkg_clone.rdepends();

    let mut res = TermTree::new(pkg);

    if depth > limit {
        return res;
    }

    for (t, deps_group) in rdep {
        let t = t.into();
        match t {
            OmaDepType::Depends | OmaDepType::PreDepends | OmaDepType::Recommends => {
                for deps in deps_group {
                    for dep in deps.iter() {
                        let Some(pkg) = apt.cache.get(dep.name()) else {
                            debug!(
                                "dep {} does not exist on apt cache, will continue",
                                dep.name()
                            );
                            continue;
                        };

                        let Some(installed_version) = pkg.installed() else {
                            debug!("pkg {} is not installed, will continue", pkg.name());
                            continue;
                        };

                        let require_ver = Version::new(unsafe { dep.parent_ver() }, &apt.cache);

                        let push = if let Some(t) = dep.comp_type() {
                            let mut push = None;
                            let t = match t {
                                ">" | ">>" => vec![Ordering::Greater],
                                "<" | "<<" => vec![Ordering::Less],
                                ">=" => vec![Ordering::Greater, Ordering::Equal],
                                "<=" => vec![Ordering::Less, Ordering::Equal],
                                "=" | "==" => vec![Ordering::Equal],
                                "!=" => {
                                    push = Some(require_ver != installed_version);
                                    vec![]
                                }
                                "" => {
                                    push = Some(true);
                                    vec![]
                                }
                                x => unreachable!("unsupported comp type {x}"),
                            };

                            debug!("{} {:?} {}", dep.name(), t, require_ver);

                            let cmp = &installed_version.cmp(&require_ver);

                            debug!("{} {installed_version} {cmp:?} {require_ver}", pkg.name());

                            if push.is_none() && t.contains(cmp) {
                                push = Some(true)
                            }

                            debug!("push = {push:?}");

                            push.unwrap_or(false)
                        } else {
                            true
                        };

                        if !push {
                            continue;
                        }

                        res.push(reverse_dep_tree(
                            PkgWrapper {
                                package: pkg,
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
