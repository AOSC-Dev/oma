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
                // 如果数组长度为一，则肯定第一个位置有值
                // 因此直接 unwrap
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
    pub version_raw: RawVersion,
    pub raw_pkg: RawPackage,
}

impl PkgInfo {
    pub fn new(version: &Version, pkg: &Package) -> Self {
        // 直接传入 &Version 会遇到 version.uris 生命周期问题，所以这里传入 RawVersion，然后就地创建 Version
        let raw_pkg = pkg.unique();
        Self {
            version_raw: version.unique(),
            raw_pkg,
        }
    }

    pub fn print_info(&self, cache: &Cache) {
        println!("Package: {}", self.raw_pkg.name());
        println!("Version: {}", self.version_raw.version());
        let ver = Version::new(self.version_raw.unique(), cache);
        println!(
            "Section: {}\n",
            ver.section().as_deref().unwrap_or("unknown")
        );
        println!(
            "Maintainer: {}",
            ver.get_record(RecordField::Maintainer)
                .unwrap_or("unknown".to_string())
        );
        println!("Installed-Size: {}", HumanBytes(ver.installed_size()));

        for (t, deps) in &self.get_deps(cache) {
            println!("{t}: {deps}");
        }

        println!("Download-Size: {}", HumanBytes(ver.size()));
        println!("APT-Source:");

        let uris = ver.uris().collect::<Vec<_>>();

        if uris.is_empty() {
            println!("  unknown");
        } else {
            for i in uris {
                println!(
                    "  {}\n",
                    source_url_to_apt_style(&i).unwrap_or(i.to_string())
                );
            }
        }

        println!(
            "Description: {}",
            ver.description().unwrap_or("No description".to_string())
        );
    }

    pub fn get_deps(&self, cache: &Cache) -> HashMap<OmaDepType, OmaDependencyGroup> {
        Version::new(self.version_raw.unique(), cache)
            .depends_map()
            .iter()
            .map(|(x, y)| (OmaDepType::from(x), OmaDependency::map_deps(y)))
            .collect::<HashMap<_, _>>()
    }

    pub fn get_rdeps(&self, cache: &Cache) -> HashMap<OmaDepType, OmaDependencyGroup> {
        Package::new(cache, self.raw_pkg.unique())
            .rdepends_map()
            .iter()
            .map(|(x, y)| (OmaDepType::from(x), OmaDependency::map_deps(y)))
            .collect::<HashMap<_, _>>()
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
    let version = pkg.candidate().unwrap();
    let info = PkgInfo::new(&version, &pkg);
    info.print_info(&cache);
}
