use std::cmp::Ordering;

use oma_apt::{
    cache::Cache,
    package::{DepType, Dependency, Package, Version},
    util::cmp_versions,
};
use tracing::debug;

use crate::{apt::OmaAptResult, pkginfo::OmaDependency};

#[derive(Debug)]
pub struct UnmetDep {
    pub package: String,
    pub unmet_dependency: WhyUnmet,
    pub specified_dependency: String,
}

#[derive(Debug)]
pub enum WhyUnmet {
    DepNotExist {
        pacakge_name: String,
        version_comp: Option<String>,
    },
    Unmet {
        dep_name: String,
        need_ver: String,
        symbol: String,
    },
    Breaks {
        break_type: String,
        dep_name: String,
        comp_ver: Option<String>,
    },
}

pub(crate) fn find_unmet_deps_with_markinstall(cache: &Cache, ver: &Version) -> Vec<UnmetDep> {
    let dep = ver.get_depends(&DepType::Depends);
    let pdep = ver.get_depends(&DepType::PreDepends);

    let mut v = vec![];

    if let Some(dep) = dep {
        let dep = OmaDependency::map_deps(dep);
        for b_dep in dep.inner() {
            for d in b_dep {
                if let Some(pkg) = cache.get(&d.name) {
                    if let Some(ver) = &d.ver {
                        if let Some(ver) = pkg.get_version(ver) {
                            find_unmet_dep_inner(&pkg, cache, &ver, &mut v);
                            continue;
                        }
                    } else if let Some(cand) = pkg.candidate() {
                        find_unmet_dep_inner(&pkg, cache, &cand, &mut v);
                        continue;
                    }
                }

                v.push(UnmetDep {
                    package: d.name.to_string(),
                    unmet_dependency: WhyUnmet::DepNotExist {
                        pacakge_name: d.name.to_string(),
                        version_comp: d.comp_ver,
                    },
                    specified_dependency: format!("{} {}", ver.parent().name(), ver.version()),
                });
            }
        }
    }

    if let Some(pdep) = pdep {
        let pdep = OmaDependency::map_deps(pdep);
        for b_dep in pdep.inner() {
            for d in b_dep {
                if let Some(pkg) = cache.get(&d.name) {
                    if let Some(ver) = &d.ver {
                        if let Some(ver) = pkg.get_version(ver) {
                            find_unmet_dep_inner(&pkg, cache, &ver, &mut v);
                            continue;
                        }
                    } else if let Some(cand) = pkg.candidate() {
                        find_unmet_dep_inner(&pkg, cache, &cand, &mut v);
                        continue;
                    }
                }

                v.push(UnmetDep {
                    package: d.name.to_string(),
                    unmet_dependency: WhyUnmet::DepNotExist {
                        pacakge_name: d.name.to_string(),
                        version_comp: d.comp_ver,
                    },
                    specified_dependency: format!("{} {}", ver.parent().name(), ver.version()),
                });
            }
        }
    }

    v
}

fn find_unmet_dep_inner(pkg: &Package, cache: &Cache, cand: &Version, v: &mut Vec<UnmetDep>) {
    let dep = pkg.rdepends_map();
    let rdep_dep = dep.get(&DepType::Depends);
    let rdep_predep = dep.get(&DepType::PreDepends);
    let rdep_breaks = dep.get(&DepType::Breaks);
    let rdep_conflicts = dep.get(&DepType::Conflicts);

    // Format dep
    if let Some(rdep_dep) = rdep_dep {
        format_deps(rdep_dep, cache, cand, v, pkg);
    }

    // Format predep
    if let Some(rdep_predep) = rdep_predep {
        format_deps(rdep_predep, cache, cand, v, pkg);
    }

    // Format breaks
    if let Some(rdep_breaks) = rdep_breaks {
        format_breaks(rdep_breaks, cache, v, pkg, cand, "Breaks");
    }

    // Format Conflicts
    if let Some(rdep_conflicts) = rdep_conflicts {
        format_breaks(rdep_conflicts, cache, v, pkg, cand, "Conflicts");
    }
}

pub(crate) fn find_unmet_deps(cache: &Cache) -> OmaAptResult<Vec<UnmetDep>> {
    let changes = cache.get_changes(true)?;
    let mut v = vec![];

    for pkg in changes {
        if let Some(cand) = pkg.candidate() {
            if !pkg.marked_delete() && !pkg.marked_purge() {
                find_unmet_dep_inner(&pkg, cache, &cand, &mut v);
            }
        }
    }

    Ok(v)
}

fn format_deps(
    rdep: &[Dependency],
    cache: &Cache,
    cand: &Version,
    v: &mut Vec<UnmetDep>,
    c: &Package,
) {
    let rdep = OmaDependency::map_deps(rdep);
    debug!("{:?}", rdep);
    for b_rdep in rdep.inner() {
        for dep in b_rdep {
            let pkg = cache.get(&dep.name);
            if let Some(pkg) = pkg {
                if pkg.is_installed() {
                    let comp = dep.comp_symbol;
                    let ver = dep.target_ver;
                    debug!("{} {comp:?} {ver:?}", pkg.name());
                    if let (Some(comp), Some(need_ver)) = (comp, ver) {
                        match comp.as_str() {
                            ">=" => {
                                // 1: 2.36-4   2: 2.36-2
                                let cmp = cmp_versions(&need_ver, cand.version()); // 要求 >= 2.36-4，但用户在安装 2.36-2
                                if cmp == Ordering::Greater {
                                    v.push(UnmetDep {
                                        package: dep.name.to_string(),
                                        unmet_dependency: WhyUnmet::Unmet {
                                            dep_name: c.name().to_owned(),
                                            need_ver,
                                            symbol: ">=".to_owned(),
                                        },
                                        specified_dependency: format!(
                                            "{} {}",
                                            c.name(),
                                            cand.version()
                                        ),
                                    })
                                }
                            }
                            ">>" => {
                                let cmp = cmp_versions(&need_ver, cand.version()); // 要求 >> 2.36-4，但用户在安装 2.36-2
                                if cmp != Ordering::Less {
                                    v.push(UnmetDep {
                                        package: dep.name.to_string(),
                                        unmet_dependency: WhyUnmet::Unmet {
                                            dep_name: c.name().to_string(),
                                            need_ver,
                                            symbol: ">>".to_string(),
                                        },
                                        specified_dependency: format!(
                                            "{} {}",
                                            c.name(),
                                            cand.version()
                                        ),
                                    })
                                }
                            }
                            ">" => {
                                let cmp = cmp_versions(&need_ver, cand.version()); // 要求 > 2.36-4，但用户在安装 2.36-2
                                if cmp != Ordering::Less {
                                    v.push(UnmetDep {
                                        package: dep.name.to_string(),
                                        unmet_dependency: WhyUnmet::Unmet {
                                            dep_name: c.name().to_string(),
                                            need_ver,
                                            symbol: ">".to_string(),
                                        },
                                        specified_dependency: format!(
                                            "{} {}",
                                            c.name(),
                                            cand.version()
                                        ),
                                    })
                                }
                            }
                            "=" => {
                                let cmp = cmp_versions(&need_ver, cand.version()); // 要求 = 2.36-4，但用户在安装 2.36-2
                                if cmp != Ordering::Equal {
                                    v.push(UnmetDep {
                                        package: dep.name.to_string(),
                                        unmet_dependency: WhyUnmet::Unmet {
                                            dep_name: c.name().to_string(),
                                            need_ver,
                                            symbol: "=".to_string(),
                                        },
                                        specified_dependency: format!(
                                            "{} {}",
                                            c.name(),
                                            cand.version()
                                        ),
                                    })
                                }
                            }
                            "<=" => {
                                // 1: 2.36-4 2: 2.36-6
                                let cmp = cmp_versions(&need_ver, cand.version()); // 要求 <= 2.36-4，但用户在安装 2.36-6
                                if cmp == Ordering::Less {
                                    v.push(UnmetDep {
                                        package: dep.name.to_string(),
                                        unmet_dependency: WhyUnmet::Unmet {
                                            dep_name: c.name().to_string(),
                                            need_ver,
                                            symbol: "<=".to_string(),
                                        },
                                        specified_dependency: format!(
                                            "{} {}",
                                            c.name(),
                                            cand.version()
                                        ),
                                    })
                                }
                            }
                            "<<" => {
                                // 1: 2.36-4 2: 2.36-6
                                let cmp = cmp_versions(&need_ver, cand.version()); // 要求 <= 2.36-4，但用户在安装 2.36-6
                                if cmp != Ordering::Greater {
                                    v.push(UnmetDep {
                                        package: dep.name.to_string(),
                                        unmet_dependency: WhyUnmet::Unmet {
                                            dep_name: c.name().to_string(),
                                            need_ver,
                                            symbol: "<<".to_string(),
                                        },
                                        specified_dependency: format!(
                                            "{} {}",
                                            c.name(),
                                            cand.version()
                                        ),
                                    })
                                }
                            }
                            "<" => {
                                // 1: 2.36-4 2: 2.36-6
                                let cmp = cmp_versions(&need_ver, cand.version()); // 要求 <= 2.36-4，但用户在安装 2.36-6
                                if cmp != Ordering::Greater {
                                    v.push(UnmetDep {
                                        package: dep.name.to_string(),
                                        unmet_dependency: WhyUnmet::Unmet {
                                            dep_name: c.name().to_string(),
                                            need_ver,
                                            symbol: "<".to_string(),
                                        },
                                        specified_dependency: format!(
                                            "{} {}",
                                            c.name(),
                                            cand.version()
                                        ),
                                    })
                                }
                            }
                            x => panic!("Unsupport symbol: {x}, pkg: {}", dep.name),
                        }
                    }
                }
            }
        }
    }
}

fn format_breaks(
    rdep_breaks: &[Dependency],
    cache: &Cache,
    v: &mut Vec<UnmetDep>,
    c: &Package,
    cand: &Version,
    typ: &str,
) {
    let rdep = OmaDependency::map_deps(rdep_breaks);
    debug!("{:?}", rdep);
    for b_rdep in rdep.inner() {
        for dep in b_rdep {
            let dep_pkg = cache.get(&dep.name);
            if let Some(dep_pkg) = dep_pkg {
                if dep.comp_ver.is_none() {
                    if dep_pkg.is_installed() {
                        v.push(UnmetDep {
                            package: dep.name,
                            unmet_dependency: WhyUnmet::Breaks {
                                break_type: typ.to_string(),
                                dep_name: dep_pkg.name().to_string(),
                                comp_ver: None,
                            },
                            specified_dependency: format!("{} {}", c.name(), cand.version()),
                        })
                    }
                } else if dep_pkg.is_installed() {
                    debug!("{} {:?} {:?}", dep_pkg.name(), dep.comp_symbol, dep.target_ver);
                    if let (Some(comp), Some(break_ver)) = (dep.comp_symbol, dep.target_ver) {
                        match comp.as_str() {
                            ">=" => {
                                // a: breaks b >= 1.0，满足要求的条件是 break_ver > cand.version
                                let cmp = cmp_versions(&break_ver, cand.version());
                                if cmp != Ordering::Greater {
                                    v.push(UnmetDep {
                                        package: dep.name,
                                        unmet_dependency: WhyUnmet::Breaks {
                                            break_type: typ.to_string(),
                                            dep_name: dep_pkg.name().to_string(),
                                            comp_ver: dep.comp_ver,
                                        },
                                        specified_dependency: format!(
                                            "{} {}",
                                            c.name(),
                                            cand.version()
                                        ),
                                    })
                                }
                            }
                            ">>" => {
                                // a: breaks b >> 1.0，满足要求的条件是 break_ver >>= cand.version
                                let cmp = cmp_versions(&break_ver, cand.version());
                                if cmp == Ordering::Less {
                                    v.push(UnmetDep {
                                        package: dep.name,
                                        unmet_dependency: WhyUnmet::Breaks {
                                            break_type: typ.to_string(),
                                            dep_name: dep_pkg.name().to_string(),
                                            comp_ver: dep.comp_ver,
                                        },
                                        specified_dependency: format!(
                                            "{} {}",
                                            c.name(),
                                            cand.version()
                                        ),
                                    })
                                }
                            }
                            ">" => {
                                // a: breaks b > 1.0，满足要求的条件是 break_ver >= cand.version
                                let cmp = cmp_versions(&break_ver, cand.version());
                                if cmp == Ordering::Less {
                                    v.push(UnmetDep {
                                        package: dep.name,
                                        unmet_dependency: WhyUnmet::Breaks {
                                            break_type: typ.to_string(),
                                            dep_name: dep_pkg.name().to_string(),
                                            comp_ver: dep.comp_ver,
                                        },
                                        specified_dependency: format!(
                                            "{} {}",
                                            c.name(),
                                            cand.version()
                                        ),
                                    })
                                }
                            }
                            "<=" => {
                                // a: breaks b <= 1.0，满足要求的条件是 break_ver < cand.version
                                let cmp = cmp_versions(&break_ver, cand.version());
                                if cmp != Ordering::Less {
                                    v.push(UnmetDep {
                                        package: dep.name,
                                        unmet_dependency: WhyUnmet::Breaks {
                                            break_type: typ.to_string(),
                                            dep_name: dep_pkg.name().to_string(),
                                            comp_ver: dep.comp_ver,
                                        },
                                        specified_dependency: format!(
                                            "{} {}",
                                            c.name(),
                                            cand.version()
                                        ),
                                    })
                                }
                            }
                            "<<" => {
                                // a: breaks b << 1.0，满足要求的条件是 break_ver <= cand.version
                                let cmp = cmp_versions(&break_ver, cand.version());
                                if cmp == Ordering::Greater {
                                    v.push(UnmetDep {
                                        package: dep.name,
                                        unmet_dependency: WhyUnmet::Breaks {
                                            break_type: typ.to_string(),
                                            dep_name: dep_pkg.name().to_string(),
                                            comp_ver: dep.comp_ver,
                                        },
                                        specified_dependency: format!(
                                            "{} {}",
                                            c.name(),
                                            cand.version()
                                        ),
                                    })
                                }
                            }
                            "<" => {
                                // a: breaks b << 1.0，满足要求的条件是 break_ver <= cand.version
                                let cmp = cmp_versions(&break_ver, cand.version());
                                if cmp == Ordering::Greater {
                                    v.push(UnmetDep {
                                        package: dep.name,
                                        unmet_dependency: WhyUnmet::Breaks {
                                            break_type: typ.to_string(),
                                            dep_name: dep_pkg.name().to_string(),
                                            comp_ver: dep.comp_ver,
                                        },
                                        specified_dependency: format!(
                                            "{} {}",
                                            c.name(),
                                            cand.version()
                                        ),
                                    })
                                }
                            }
                            x => panic!("Unsupport symbol: {x}, pkg: {}", dep.name),
                        }
                    }
                }
            }
        }
    }
}
