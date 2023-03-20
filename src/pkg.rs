use anyhow::{bail, Context, Result};
use console::style;
use glob_match::glob_match_with_captures;
use indicatif::HumanBytes;
use rust_apt::{
    cache::{Cache, PackageSort},
    package::{BaseDep, DepType, Dependency, Package, Version},
    raw::package::RawVersion,
    records::RecordField,
};
use std::{collections::HashMap, fmt::Write, sync::atomic::Ordering};

use crate::{cli::gen_prefix, pager::Pager, ALLOWCTRLC, info};

pub struct PkgInfo {
    pub package: String,
    pub version: String,
    pub section: Option<String>,
    pub maintainer: String,
    pub installed_size: String,
    pub dep_map: HashMap<String, String>,
    pub download_size: String,
    pub apt_manual_installed: Option<String>,
    pub apt_sources: Vec<String>,
    pub description: Option<String>,
    pub has_dbg: bool,
    pub provides: Vec<String>,
    pub deps: HashMap<String, Vec<Vec<OmaDependency>>>,
    pub version_raw: RawVersion,
    pub rdeps: HashMap<String, Vec<Vec<OmaDependency>>>,
}

impl PkgInfo {
    pub fn new(cache: &Cache, version_raw: RawVersion, pkg: &Package) -> Result<Self> {
        // 直接传入 &Version 会遇到 version.uris 生命周期问题，所以这里传入 RawVersion，然后就地创建 Version
        let version = Version::new(version_raw, pkg);
        let version_raw = version.unique();

        let section = version.section().ok().map(|x| x.to_owned());

        let maintainer = version
            .get_record(RecordField::Maintainer)
            .unwrap_or("Null <null>".to_owned());

        let installed_size = HumanBytes(version.installed_size()).to_string();

        let download_size = HumanBytes(version.size()).to_string();
        let apt_sources = version.uris().collect::<Vec<_>>();
        let description = version.description();
        let dep_map = dep_to_str_map(version.depends_map());

        let has_dbg = if let Some(pkg) = cache.get(&format!("{}-dbg", pkg.name())) {
            pkg.get_version(version.version()).is_some()
        } else {
            false
        };

        let provides = pkg
            .provides()
            .map(|x| x.name().to_string())
            .collect::<Vec<_>>();

        let deps = deps_to_map(version.depends_map());
        let rdeps = deps_to_map(&pkg.rdepends_map());

        Ok(Self {
            package: pkg.name().to_owned(),
            version: version.version().to_owned(),
            section,
            maintainer,
            installed_size,
            dep_map,
            download_size,
            apt_manual_installed: None, // TODO
            apt_sources,
            description,
            has_dbg,
            provides,
            deps,
            version_raw,
            rdeps,
        })
    }
}

fn dep_to_str_map(map: &HashMap<DepType, Vec<Dependency>>) -> HashMap<String, String> {
    let mut res = HashMap::new();
    for (k, v) in map {
        match k {
            DepType::Depends => res.insert("Depends".to_string(), dep_map_str(v)),
            DepType::PreDepends => res.insert("PreDepends".to_string(), dep_map_str(v)),
            DepType::Suggests => res.insert("Suggests".to_string(), dep_map_str(v)),
            DepType::Recommends => res.insert("Recommends".to_string(), dep_map_str(v)),
            DepType::Conflicts => res.insert("Conflicts".to_string(), dep_map_str(v)),
            DepType::Replaces => res.insert("Replaces".to_string(), dep_map_str(v)),
            DepType::Obsoletes => res.insert("Obsoletes".to_string(), dep_map_str(v)),
            DepType::Breaks => res.insert("Breaks".to_string(), dep_map_str(v)),
            DepType::Enhances => res.insert("Enhances".to_string(), dep_map_str(v)),
        };
    }

    res
}

pub fn deps_to_map(
    map: &HashMap<DepType, Vec<Dependency>>,
) -> HashMap<String, Vec<Vec<OmaDependency>>> {
    let mut res = HashMap::new();
    for (k, v) in map {
        match k {
            DepType::Depends => res.insert("Depends".to_string(), OmaDependency::map_deps(v)),
            DepType::PreDepends => res.insert("PreDepends".to_string(), OmaDependency::map_deps(v)),
            DepType::Suggests => res.insert("Suggests".to_string(), OmaDependency::map_deps(v)),
            DepType::Recommends => res.insert("Recommends".to_string(), OmaDependency::map_deps(v)),
            DepType::Conflicts => res.insert("Conflicts".to_string(), OmaDependency::map_deps(v)),
            DepType::Replaces => res.insert("Replaces".to_string(), OmaDependency::map_deps(v)),
            DepType::Obsoletes => res.insert("Obsoletes".to_string(), OmaDependency::map_deps(v)),
            DepType::Breaks => res.insert("Breaks".to_string(), OmaDependency::map_deps(v)),
            DepType::Enhances => res.insert("Enhances".to_string(), OmaDependency::map_deps(v)),
        };
    }

    res
}

pub fn query_pkgs(cache: &Cache, input: &str) -> Result<Vec<(PkgInfo, bool)>> {
    let mut res = Vec::new();
    if input.contains('=') {
        let mut split_arg = input.split('=');
        let name = split_arg.next().context(format!("Not Support: {input}"))?;
        let version_str = split_arg.collect::<String>();

        let pkg = cache
            .get(name)
            .context(format!("Can not get package {name}"))?;

        let version = pkg
            .get_version(&version_str)
            .context(format!("Can not get pkg {name} version {version_str}"))?;

        let oma_pkg = PkgInfo::new(cache, version.unique(), &pkg)?;

        res.push((oma_pkg, true));
    } else if input.ends_with(".deb") {
        bail!("search_pkg does not support read local deb package");
    } else if input.contains('/') {
        let mut split_arg = input.split('/');
        let name = split_arg.next().context(format!("Not Support: {input}"))?;
        let branch = split_arg.collect::<String>();

        let pkg = cache
            .get(name)
            .take()
            .context(format!("Can not get package: {input}"))?;

        let mut sort = vec![];

        for i in pkg.versions() {
            let item = i
                .get_record(RecordField::Filename)
                .context(format!("Can not get package {name} filename!"))?;

            if item.split('/').nth(1) == Some(&branch) {
                sort.push(i)
            }
        }

        if sort.is_empty() {
            bail!("Can not get package {} with {} branch.", name, branch);
        }

        sort.sort_by(|x, y| rust_apt::util::cmp_versions(x.version(), y.version()));

        let version = sort.last().unwrap();
        let oma_pkg = PkgInfo::new(cache, version.unique(), &pkg)?;

        res.push((oma_pkg, true));
    } else {
        let sort = PackageSort::default();
        let search_res = cache
            .packages(&sort)
            .filter(|x| glob_match_with_captures(input, x.name()).is_some());

        for pkg in search_res {
            let versions = pkg.versions();

            for ver in versions {
                let oma_pkg = PkgInfo::new(cache, ver.unique(), &pkg)?;
                if pkg.candidate() == Some(ver) {
                    res.push((oma_pkg, true));
                } else {
                    res.push((oma_pkg, false));
                }
            }
        }
    }

    Ok(res)
}

fn dep_map_str(deps: &[Dependency]) -> String {
    let mut depends_str = String::new();
    let deps = OmaDependency::map_deps(deps);

    for d in deps {
        if d.len() == 1 {
            let dep = d.first().unwrap();
            depends_str.push_str(&dep.name);
            if let Some(comp) = &dep.comp_ver {
                let _ = write!(depends_str, " ({comp})");
            }
            depends_str.push_str(", ");
        } else {
            let mut or_str = String::new();
            let total = d.len() - 1;
            for (num, base_dep) in d.iter().enumerate() {
                or_str.push_str(&base_dep.name);
                if let Some(comp) = &base_dep.comp_ver {
                    let _ = write!(or_str, " ({comp})");
                }
                if num != total {
                    or_str.push_str(" | ");
                } else {
                    or_str.push_str(", ");
                }
            }
            depends_str.push_str(&or_str);
        }
    }

    depends_str
}

#[derive(Debug)]
pub struct OmaDependency {
    pub name: String,
    pub comp_str: Option<String>,
    pub ver: Option<String>,
    pub comp_ver: Option<String>,
}

impl OmaDependency {
    fn new(dep: &BaseDep) -> Self {
        Self {
            name: dep.name(),
            comp_str: dep.comp().map(|x| x.to_string()),
            ver: dep.version().map(|x| x.to_string()),
            comp_ver: dep
                .comp()
                .and_then(|x| Some(format!("{x} {}", dep.version()?))),
        }
    }

    pub fn map_deps(deps: &[Dependency]) -> Vec<Vec<Self>> {
        let mut res = vec![];

        for dep in deps {
            if dep.is_or() {
                let mut v = vec![];
                for base_dep in &dep.base_deps {
                    v.push(Self::new(base_dep));
                }
                res.push(v);
            } else {
                let lone_dep = dep.first();
                res.push(vec![Self::new(lone_dep)]);
            }
        }

        res
    }
}

pub fn search_pkgs(cache: &Cache, input: &str) -> Result<()> {
    let sort = PackageSort::default();
    let packages = cache.packages(&sort).collect::<Vec<_>>();

    let mut res = HashMap::new();

    for pkg in packages {
        let cand = pkg.candidate().unwrap();
        if pkg.name().contains(input) && !pkg.name().contains("-dbg") {
            let oma_pkg = PkgInfo::new(cache, cand.unique(), &pkg)?;
            res.insert(
                pkg.name().to_string(),
                (oma_pkg, cand.is_installed(), pkg.is_upgradable()),
            );
        }

        if cand.description().unwrap_or("".to_owned()).contains(input)
            && !res.contains_key(pkg.name())
            && !pkg.name().contains("-dbg")
        {
            let oma_pkg = PkgInfo::new(cache, cand.unique(), &pkg)?;
            res.insert(
                pkg.name().to_string(),
                (oma_pkg, cand.is_installed(), pkg.is_upgradable()),
            );

            let providers = cand.provides().collect::<Vec<_>>();

            if !providers.is_empty() {
                for provider in providers {
                    if provider.name() == input {
                        let oma_pkg = PkgInfo::new(cache, cand.unique(), &pkg)?;
                        res.insert(
                            pkg.name().to_string(),
                            (oma_pkg, cand.is_installed(), pkg.is_upgradable()),
                        );
                    }
                }
            }
        }
    }

    let mut res = res.into_values().collect::<Vec<_>>();

    res.sort_unstable_by(|x, y| {
        let x_score = pkg_score(input, &x.0);
        let y_score = pkg_score(input, &y.0);

        let c = x_score.cmp(&y_score);

        if c == std::cmp::Ordering::Equal {
            x.0.package.cmp(&y.0.package)
        } else {
            c
        }
    });

    res.reverse();

    if res.is_empty() {
        bail!("Could not find any packages for keyword: {input}");
    }

    let height = crate::WRITER.get_height();

    let mut output = vec![];

    for (pkg, installed, upgradable) in res {
        let prefix = if installed {
            style("INSTALLED").green().to_string()
        } else if upgradable {
            style("UPGRADE").yellow().to_string()
        } else {
            style("AVAIL").dim().to_string()
        };

        let mut pkg_info_line = if pkg.section == Some("Bases".to_owned()) {
            style(&pkg.package).bold().blue().to_string()
        } else {
            style(&pkg.package).bold().to_string()
        };

        pkg_info_line.push(' ');

        if upgradable {
            let p = cache.get(&pkg.package).unwrap();
            let installed = p.installed().unwrap();
            let now_version = installed.version();
            pkg_info_line.push_str(&format!(
                "{} -> {}",
                style(now_version).yellow(),
                style(pkg.version).green()
            ));
        } else {
            pkg_info_line.push_str(&style(&pkg.version).green().to_string());
        }

        if pkg.has_dbg {
            pkg_info_line.push(' ');
            pkg_info_line.push_str(&style("(debug symbols available)").dim().to_string());
        }

        output.push((
            prefix,
            pkg_info_line,
            pkg.description.unwrap_or("".to_owned()),
        ));
    }

    if output.len() * 2 <= height.into() {
        for (prefix, line, desc) in &output {
            crate::WRITER.writeln(prefix, line)?;
            crate::WRITER.writeln("", desc)?;
        }
    } else {
        let mut pager = Pager::new(false, false)?;
        let mut out = pager.get_writer()?;

        ALLOWCTRLC.store(true, Ordering::Relaxed);

        for (prefix, line, desc) in &output {
            writeln!(out, "{}{line}", gen_prefix(prefix)).ok();
            writeln!(out, "{}{desc}", gen_prefix("")).ok();
        }

        drop(out);
        pager.wait_for_exit().ok();
    }

    Ok(())
}

fn pkg_score(input: &str, pkginfo: &PkgInfo) -> u16 {
    for i in &pkginfo.provides {
        if i == input {
            return 1000;
        }
    }

    (strsim::jaro_winkler(&pkginfo.package, input) * 1000.0) as u16
}

/// Mark package as install status
pub fn mark_install(
    cache: &Cache,
    pkg: &str,
    ver: RawVersion,
    reinstall: bool,
    is_local: bool,
) -> Result<()> {
    let pkg = cache.get(pkg).unwrap();
    let ver = Version::new(ver, &pkg);
    ver.set_candidate();

    let version = ver.version();

    if pkg.installed().as_ref() == Some(&ver) && !reinstall {
        info!("{} {version} is already installed.", pkg.name());
        return Ok(());
    } else if pkg.installed().as_ref() == Some(&ver) && reinstall {
        pkg.mark_reinstall(true);
    } else {
        pkg.mark_install(true, true);
        if !pkg.marked_install() && !pkg.marked_downgrade() && !pkg.marked_upgrade() {
            // apt 会先就地检查这个包的表面依赖是否满足要求，如果不满足则直接返回错误，而不是先交给 resolver
            bail!(
                "{} can't marked installed! maybe dependency issue?",
                if is_local {
                    ver.uris().next().unwrap_or(pkg.name().to_string())
                } else {
                    pkg.name().to_string()
                }
            );
        }
    }

    pkg.protect();

    Ok(())
}
