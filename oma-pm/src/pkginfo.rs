use std::{collections::HashMap, fmt::Display};

use oma_apt::{
    cache::Cache,
    package::{BaseDep, DepType, Dependency, Package, Version},
    raw::package::{RawPackage, RawVersion},
    records::RecordField,
};
use oma_utils::human_bytes::HumanBytes;

#[derive(Debug)]
pub struct OmaDependency {
    pub name: String,
    pub comp_symbol: Option<String>,
    pub ver: Option<String>,
    pub target_ver: Option<String>,
    pub comp_ver: Option<String>,
}

impl From<&BaseDep<'_>> for OmaDependency {
    fn from(dep: &BaseDep) -> Self {
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
}

pub struct OmaDependencyGroup(Vec<Vec<OmaDependency>>);

impl OmaDependencyGroup {
    pub fn inner(self) -> Vec<Vec<OmaDependency>> {
        self.0
    }
}

impl Display for OmaDependencyGroup {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        for (i, d) in self.0.iter().enumerate() {
            if d.len() == 1 {
                let dep = d.first().unwrap();
                f.write_str(&dep.name)?;
                if let Some(comp) = &dep.comp_ver {
                    f.write_str(&format!(" ({comp})"))?;
                }
                if i != self.0.len() - 1 {
                    f.write_str(", ")?;
                }
            } else {
                let total = d.len() - 1;
                for (num, base_dep) in d.iter().enumerate() {
                    f.write_str(&base_dep.name)?;
                    if let Some(comp) = &base_dep.comp_ver {
                        f.write_str(&format!(" ({comp})"))?;
                    }
                    if i != self.0.len() - 1 {
                        if num != total {
                            f.write_str(" | ")?;
                        } else {
                            f.write_str(", ")?;
                        }
                    }
                }
            }
        }

        Ok(())
    }
}

impl OmaDependency {
    pub fn map_deps(deps: &[Dependency]) -> OmaDependencyGroup {
        let mut res = vec![];

        for dep in deps {
            if dep.is_or() {
                let mut v = vec![];
                for base_dep in &dep.base_deps {
                    v.push(Self::from(base_dep));
                }
                res.push(v);
            } else {
                let lone_dep = dep.first();
                res.push(vec![Self::from(lone_dep)]);
            }
        }

        OmaDependencyGroup(res)
    }
}

#[derive(Debug, Eq, PartialEq, Hash)]
pub enum OmaDepType {
    Depends,
    PreDepends,
    Suggests,
    Recommends,
    Conflicts,
    Replaces,
    Obsoletes,
    Breaks,
    Enhances,
}

impl Display for OmaDepType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self)
    }
}

impl From<&DepType> for OmaDepType {
    fn from(v: &oma_apt::package::DepType) -> Self {
        match v {
            oma_apt::package::DepType::Depends => OmaDepType::Depends,
            oma_apt::package::DepType::PreDepends => OmaDepType::PreDepends,
            oma_apt::package::DepType::Suggests => OmaDepType::Suggests,
            oma_apt::package::DepType::Recommends => OmaDepType::Recommends,
            oma_apt::package::DepType::Conflicts => OmaDepType::Conflicts,
            oma_apt::package::DepType::Replaces => OmaDepType::Replaces,
            oma_apt::package::DepType::Obsoletes => OmaDepType::Obsoletes,
            oma_apt::package::DepType::Breaks => OmaDepType::Breaks,
            oma_apt::package::DepType::Enhances => OmaDepType::Enhances,
        }
    }
}

pub struct PkgInfo {
    pub section: Option<String>,
    pub maintainer: String,
    pub installed_size: u64,
    pub download_size: u64,
    pub apt_manual_installed: Option<String>,
    pub apt_sources: Vec<String>,
    pub description: Option<String>,
    pub has_dbg: bool,
    pub provides: Vec<String>,
    pub deps: HashMap<OmaDepType, OmaDependencyGroup>,
    pub version_raw: RawVersion,
    pub rdeps: HashMap<OmaDepType, OmaDependencyGroup>,
    pub raw_pkg: RawPackage,
    pub recommend: OmaDependencyGroup,
    pub suggest: OmaDependencyGroup,
    pub is_candidate: bool,
    pub arch: String,
    pub checksum: Option<String>,
}

impl PkgInfo {
    pub fn new(cache: &Cache, version_raw: RawVersion, pkg: &Package) -> Self {
        // 直接传入 &Version 会遇到 version.uris 生命周期问题，所以这里传入 RawVersion，然后就地创建 Version
        let version = Version::new(version_raw, cache);
        let version_raw = version.unique();

        let section = version.section().ok().map(|x| x.to_owned());

        let maintainer = version
            .get_record(RecordField::Maintainer)
            .unwrap_or("Null <null>".to_owned());

        let checksum = version.get_record(RecordField::SHA256);

        let installed_size = version.installed_size();
        let download_size = version.size();

        let is_cand = pkg.candidate().map(|x| x.unique()) == Some(version.unique());

        let apt_sources = version.uris().collect::<Vec<_>>();
        let description = version.description();

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

        let deps = version
            .depends_map()
            .iter()
            .map(|(x, y)| (OmaDepType::from(x), OmaDependency::map_deps(y)))
            .collect::<HashMap<_, _>>();

        let rdeps = pkg
            .rdepends_map()
            .iter()
            .map(|(x, y)| (OmaDepType::from(x), OmaDependency::map_deps(y)))
            .collect::<HashMap<_, _>>();

        let raw_pkg = pkg.unique();

        let arch = version.arch();

        Self {
            section,
            maintainer,
            installed_size,
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
            is_candidate: is_cand,
            arch: arch.to_string(),
            checksum,
        }
    }

    pub fn set_candidate(&mut self, cache: &Cache) {
        let version = Version::new(self.version_raw.unique(), cache);
        version.set_candidate();
        self.is_candidate = true;
    }
}

impl Display for PkgInfo {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&format!("Package: {}\n", self.raw_pkg.name()))?;
        f.write_str(&format!("Version: {}\n", self.version_raw.version()))?;
        f.write_str(&format!(
            "Section: {}\n",
            self.section.as_deref().unwrap_or("unknow")
        ))?;

        f.write_str(&format!("Maintainer: {}\n", self.maintainer))?;
        f.write_str(&format!(
            "Installed-Size: {}\n",
            HumanBytes(self.installed_size)
        ))?;

        for (t, deps) in &self.deps {
            f.write_str(&format!("{t}: {deps}\n"))?;
        }

        f.write_str(&format!(
            "Download-Size: {}\n",
            HumanBytes(self.download_size)
        ))?;

        f.write_str("APT-Source:\n")?;

        for i in &self.apt_sources {
            f.write_str(&format!(
                " {}\n",
                source_url_to_apt_style(i).unwrap_or(i.to_string())
            ))?;
        }

        f.write_str(&format!(
            "Description: {}",
            self.description.as_deref().unwrap_or("No description")
        ))?;

        Ok(())
    }
}

// input: like http://50.50.1.183/debs/pool/stable/main/f/fish_3.6.0-0_amd64.deb
// output: http://50.50.1.183/debs stable main
fn source_url_to_apt_style(s: &str) -> Option<String> {
    let mut s_split = s.split('/');
    let component = s_split.nth_back(2)?;
    let branch = s_split.next_back()?;
    let _ = s_split.next_back();

    let host = &s_split.collect::<Vec<_>>().join("/");

    Some(format!("{host} {branch} {component}"))
}

#[test]
fn test_source_url_to_apt_style() {
    let url = "http://50.50.1.183/debs/pool/stable/main/f/fish_3.6.0-0_amd64.deb";
    let s = source_url_to_apt_style(url);

    assert_eq!(s, Some("http://50.50.1.183/debs stable main".to_string()));
}

#[test]
fn test_pkginfo_display() {
    use oma_apt::new_cache;
    let cache = new_cache!().unwrap();
    let pkg = cache.get("apt").unwrap();
    let version = pkg.candidate().unwrap().unique();
    let info = PkgInfo::new(&cache, version, &pkg);
    println!("{info}")
}
