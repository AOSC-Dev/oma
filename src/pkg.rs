use anyhow::{bail, Context, Result};
use console::style;
use dialoguer::{theme::ColorfulTheme, Confirm, Input};
use glob_match::glob_match_with_captures;
use indicatif::{HumanBytes, ProgressBar};
use indicium::simple::{Indexable, SearchIndex};
use rust_apt::{
    cache::{Cache, PackageSort},
    package::{BaseDep, DepType, Dependency, Package, Version},
    raw::package::{RawPackage, RawVersion},
    records::RecordField,
};
use std::{
    collections::HashMap,
    fmt::{Display, Write},
    path::Path,
    sync::atomic::Ordering,
};

use crate::{
    cli::gen_prefix, fl, formatter::find_unmet_deps_with_markinstall, info, pager::Pager,
    utils::apt_style_url, ALLOWCTRLC, CONFIG,
};

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
    pub raw_pkg: RawPackage,
    pub recommend: Vec<Vec<OmaDependency>>,
    pub suggest: Vec<Vec<OmaDependency>>,
}

impl PkgInfo {
    pub fn new(cache: &Cache, version_raw: RawVersion, pkg: &Package) -> Result<Self> {
        // 直接传入 &Version 会遇到 version.uris 生命周期问题，所以这里传入 RawVersion，然后就地创建 Version
        let version = Version::new(version_raw, cache);
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

        let recommend = OmaDependency::map_deps(version.recommends().unwrap_or(&vec![]));
        let suggest = OmaDependency::map_deps(version.suggests().unwrap_or(&vec![]));

        let provides = pkg
            .provides()
            .map(|x| x.name().to_string())
            .collect::<Vec<_>>();

        let deps = deps_to_map(version.depends_map());
        let rdeps = deps_to_map(pkg.rdepends_map());

        let raw_pkg = pkg.unique();

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
            raw_pkg,
            recommend,
            suggest,
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
        let name = split_arg.next().unwrap();
        let pkg =
            get_real_pkg(cache, name).context(fl!("can-not-get-pkg-from-database", name = name))?;
        let pkg = Package::new(cache, pkg);

        let version_str = split_arg.collect::<String>();

        let version = pkg.get_version(&version_str).context(fl!(
            "can-not-get-pkg-version-from-database",
            name = pkg.name(),
            version = version_str
        ))?;

        let oma_pkg = PkgInfo::new(cache, version.unique(), &pkg)?;

        res.push((oma_pkg, true));
    } else if input.contains('/') && !input.ends_with(".deb") {
        let mut split_arg = input.split('/');
        let name = split_arg.next().unwrap();
        let branch = split_arg.collect::<String>();

        let pkg =
            get_real_pkg(cache, name).context(fl!("can-not-get-pkg-from-database", name = name))?;
        let pkg = Package::new(cache, pkg);

        let mut sort = vec![];

        for i in pkg.versions() {
            let item = i.get_record(RecordField::Filename);

            if let Some(item) = item {
                if item.split('/').nth(1) == Some(&branch) {
                    sort.push(i)
                }
            }
        }

        if sort.is_empty() {
            bail!(fl!(
                "can-not-get-package-with-branch",
                name = name,
                branch = branch
            ));
        }

        sort.sort_by(|x, y| rust_apt::util::cmp_versions(x.version(), y.version()));

        let version = sort.last().unwrap();
        let oma_pkg = PkgInfo::new(cache, version.unique(), &pkg)?;

        res.push((oma_pkg, true));
    } else if input.ends_with(".deb") {
        let sort = PackageSort::default().only_virtual();
        let glob = cache
            .packages(&sort)?
            .filter(|x| glob_match_with_captures(input, x.name()).is_some())
            .collect::<Vec<_>>();

        for i in glob {
            let real_pkg = get_real_pkg(cache, i.name());
            if let Some(pkg) = real_pkg {
                let pkg = Package::new(cache, pkg);

                let path = format!(
                    "file:{}",
                    apt_style_url(
                        Path::new(i.name())
                            .canonicalize()?
                            .to_str()
                            .unwrap_or(i.name())
                    )
                );

                for ver in pkg.versions() {
                    let oma_pkg = PkgInfo::new(cache, ver.unique(), &pkg)?;
                    let has = oma_pkg.apt_sources.iter().any(|x| x == &path);
                    res.push((oma_pkg, has));
                }
            }
        }
    } else {
        let sort = PackageSort::default();
        let mut search_res = cache
            .packages(&sort)?
            .filter(|x| glob_match_with_captures(input, x.name()).is_some())
            .collect::<Vec<_>>();

        let virt_pkg = get_real_pkg(cache, input);

        if !search_res.iter().any(|x| Some(x.unique()) == virt_pkg) {
            if let Some(pkg) = virt_pkg {
                let pkg = Package::new(cache, pkg);
                search_res.push(pkg);
            }
        }

        for pkg in &search_res {
            let versions = pkg.versions();

            for ver in versions {
                let oma_pkg = PkgInfo::new(cache, ver.unique(), pkg)?;
                res.push((oma_pkg, pkg.candidate() == Some(ver)));
            }
        }
    }

    Ok(res)
}

pub fn get_real_pkg(cache: &Cache, pkgname: &str) -> Option<RawPackage> {
    let mut res = None;
    let sort = PackageSort::default().only_virtual();
    let mut pkgs = cache.packages(&sort).unwrap();

    let r = pkgs.find(|x| x.name() == pkgname);

    if let Some(r) = r {
        let mut provides = r.provides();
        res = provides.next().map(|x| x.target_pkg());
    }

    res.or(cache.get(pkgname).map(|x| x.unique()))
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
    pub comp_symbol: Option<String>,
    pub ver: Option<String>,
    pub target_ver: Option<String>,
    pub comp_ver: Option<String>,
}

impl OmaDependency {
    fn new(dep: &BaseDep) -> Self {
        Self {
            name: dep.name().to_owned(),
            comp_symbol: dep.comp().map(|x| x.to_string()),
            ver: dep.version().map(|x| x.to_string()),
            target_ver: dep.target_ver().ok().map(|x| x.to_string()),
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

#[derive(PartialEq, Eq, Debug, Clone)]
enum PackageStatus {
    Avail,
    Installed,
    Upgrade,
}

impl Display for PackageStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PackageStatus::Avail => write!(f, "{}", style(fl!("pkg-search-avail")).dim()),
            PackageStatus::Installed => write!(f, "{}", style(fl!("pkg-search-installed")).green()),
            PackageStatus::Upgrade => write!(f, "{}", style(fl!("pkg-search-upgrade")).yellow()),
        }
    }
}

impl PartialOrd for PackageStatus {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for PackageStatus {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        match self {
            PackageStatus::Avail => match other {
                PackageStatus::Avail => std::cmp::Ordering::Equal,
                PackageStatus::Installed => std::cmp::Ordering::Greater,
                PackageStatus::Upgrade => std::cmp::Ordering::Less,
            },
            PackageStatus::Installed => match other {
                PackageStatus::Avail => std::cmp::Ordering::Less,
                PackageStatus::Installed => std::cmp::Ordering::Equal,
                PackageStatus::Upgrade => std::cmp::Ordering::Less,
            },
            PackageStatus::Upgrade => match other {
                PackageStatus::Avail => std::cmp::Ordering::Greater,
                PackageStatus::Installed => std::cmp::Ordering::Greater,
                PackageStatus::Upgrade => std::cmp::Ordering::Equal,
            },
        }
    }
}

struct SearchEntry {
    pkgname: String,
    pkginfo: PkgInfo,
    status: PackageStatus,
    provide: Option<String>,
}

impl Indexable for SearchEntry {
    fn strings(&self) -> Vec<String> {
        vec![
            self.pkgname.clone(),
            self.pkginfo.description.clone().unwrap_or("".to_string()),
        ]
    }
}

pub fn search_pkgs(cache: &Cache, input: &str) -> Result<()> {
    let input = input.to_lowercase();
    let sort = PackageSort::default().include_virtual();
    let packages = cache.packages(&sort)?;

    let mut pkg_map = HashMap::new();

    for pkg in packages {
        if pkg.name().contains("-dbg") {
            continue;
        }

        let status = if pkg.is_upgradable() {
            PackageStatus::Upgrade
        } else if pkg.is_installed() {
            PackageStatus::Installed
        } else {
            PackageStatus::Avail
        };

        if let Some(cand) = pkg.candidate() {
            pkg_map.insert(
                pkg.name().to_string(),
                SearchEntry {
                    pkgname: pkg.name().to_string(),
                    pkginfo: PkgInfo::new(cache, cand.unique(), &pkg)?,
                    status,
                    provide: None,
                },
            );
            continue;
        }

        let real_pkgs = pkg
            .provides()
            .map(|x| (x.name().to_string(), x.target_pkg()));

        for (provide, i) in real_pkgs {
            let pkg = Package::new(cache, i.unique());

            let status = if pkg.is_upgradable() {
                PackageStatus::Upgrade
            } else if pkg.is_installed() {
                PackageStatus::Installed
            } else {
                PackageStatus::Avail
            };

            if let Some(cand) = pkg.candidate() {
                pkg_map.insert(
                    i.name().to_string(),
                    SearchEntry {
                        pkgname: pkg.name().to_string(),
                        pkginfo: PkgInfo::new(cache, cand.unique(), &pkg)?,
                        status,
                        provide: Some(provide.to_string()),
                    },
                );
            }
        }
    }

    let mut search_index: SearchIndex<String> = SearchIndex::default();

    pkg_map
        .iter()
        .for_each(|(key, value)| search_index.insert(key, value));

    let res = search_index.search(&input);

    if res.is_empty() {
        bail!(fl!("could-not-find-pkg-from-keyword", c = input));
    }

    let height = crate::WRITER.get_height();

    let mut output = vec![];

    for i in res {
        let mut full_match = false;
        let entry = pkg_map.get(i).unwrap();

        let pkg = &entry.pkginfo;

        let mut pkg_info_line = if pkg.section == Some("Bases".to_owned()) {
            style(&pkg.package).bold().blue().to_string()
        } else {
            style(&pkg.package).bold().to_string()
        };

        pkg_info_line.push(' ');

        if entry.status == PackageStatus::Upgrade {
            let p = cache.get(&pkg.package).unwrap();
            let installed = p.installed().unwrap();
            let now_version = installed.version();
            pkg_info_line.push_str(&format!(
                "{} -> {}",
                style(now_version).yellow(),
                style(&pkg.version).green()
            ));
        } else {
            pkg_info_line.push_str(&style(&pkg.version).green().to_string());
        }

        if pkg.has_dbg {
            pkg_info_line.push(' ');
            pkg_info_line.push_str(&style(fl!("debug-symbol-available")).dim().to_string());
        }

        if pkg.package == input || entry.provide == Some(input.to_string()) {
            pkg_info_line.push(' ');
            full_match = true;
            pkg_info_line.push_str(
                &style(format!("[{}]", fl!("full-match")))
                    .yellow()
                    .bold()
                    .to_string(),
            );
        }

        output.push((
            entry.status.clone(),
            pkg_info_line,
            pkg.description.clone().unwrap_or("".to_owned()),
            full_match,
        ));
    }

    output.sort_by(|a, b| b.0.cmp(&a.0));

    if output.len() * 2 <= height.into() {
        let fm = output.iter().position(|x| x.3);

        if let Some(fm) = fm {
            let (prefix, line, desc, _) = output.remove(fm);
            crate::WRITER.writeln(&prefix.to_string(), &line, false)?;
            crate::WRITER.writeln("", &desc, false)?;
        }

        for (prefix, line, desc, _) in output {
            crate::WRITER.writeln(&prefix.to_string(), &line, false)?;
            crate::WRITER.writeln("", &desc, false)?;
        }
    } else {
        let mut pager = Pager::new(false, false)?;
        let mut out = pager.get_writer()?;

        ALLOWCTRLC.store(true, Ordering::Relaxed);

        let fm = output.iter().position(|x| x.3);

        if let Some(fm) = fm {
            let (prefix, line, desc, _) = output.remove(fm);
            writeln!(out, "{}{line}", gen_prefix(&prefix.to_string())).ok();
            writeln!(out, "{}{desc}", gen_prefix("")).ok();
        }

        for (prefix, line, desc, _) in output {
            writeln!(out, "{}{line}", gen_prefix(&prefix.to_string())).ok();
            writeln!(out, "{}{desc}", gen_prefix("")).ok();
        }

        drop(out);
        pager.wait_for_exit().ok();
    }

    Ok(())
}

/// Mark package as install status
pub fn mark_install(
    cache: &Cache,
    pkgname: &str,
    ver: RawVersion,
    reinstall: bool,
    is_local: bool,
    pb: Option<&ProgressBar>,
) -> Result<()> {
    let pkg = cache.get(pkgname).unwrap();
    let ver = Version::new(ver, cache);
    ver.set_candidate();

    let version = ver.version();

    if pkg.installed().as_ref() == Some(&ver) && !reinstall {
        if let Some(pb) = pb {
            crate::WRITER.writeln_with_pb(
                pb,
                &style("INFO").blue().bold().to_string(),
                fl!("already-installed", name = pkg.name(), version = version).as_str(),
            )?;
        } else {
            info!(
                "{}",
                fl!("already-installed", name = pkg.name(), version = version)
            );
        }
        return Ok(());
    } else if pkg.installed().as_ref() == Some(&ver) && reinstall {
        if ver.uris().next().is_none() {
            bail!(fl!(
                "can-not-mark-reinstall",
                name = pkg.name(),
                version = version
            ));
        }
        pkg.mark_reinstall(true);
    } else {
        pkg.mark_install(true, true);
        if !pkg.marked_install() && !pkg.marked_downgrade() && !pkg.marked_upgrade() {
            // apt 会先就地检查这个包的表面依赖是否满足要求，如果不满足则直接返回错误，而不是先交给 resolver
            let fined = find_unmet_deps_with_markinstall(cache, &ver, true)?;
            if fined {
                bail!("")
            } else {
                let d = if is_local {
                    ver.uris().next().unwrap_or(pkg.name().to_string())
                } else {
                    pkg.name().to_string()
                };
                bail!(fl!("mayble-dep-issue", name = d));
            }
        }
    }

    pkg.protect();

    Ok(())
}

/// Mark package as delete status
pub fn mark_delete(pkg: &Package, is_purge: bool) -> Result<()> {
    let oma_config = CONFIG
        .get_or_try_init(crate::Config::read)?
        .general
        .protect_essentials;

    if pkg.is_essential() && oma_config {
        bail!(fl!("pkg-is-essential", name = pkg.name()));
    } else if !oma_config {
        let theme = ColorfulTheme::default();
        let delete = Confirm::with_theme(&theme)
            .with_prompt(format!(
                "DELETE THIS PACKAGE? PACKAGE {} IS ESSENTIAL!",
                pkg.name()
            ))
            .default(false)
            .interact()?;
        if !delete {
            info!("Not confirmed.");
            return Ok(());
        }
        info!(
            "If you are absolutely sure, please type the following:\n{}",
            style("Do as I say!").bold()
        );
        if Input::<String>::with_theme(&theme)
            .with_prompt("Your turn")
            .interact()?
            != "Do as I say!"
        {
            info!("Prompt answered incorrectly. Not confirmed.");
            return Ok(());
        }
    }

    pkg.mark_delete(is_purge);
    pkg.protect();

    Ok(())
}
