use std::fmt::Display;

use cxx::UniquePtr;
use oma_apt::{
    cache::Cache,
    raw::{IntoRawIter, PkgIterator, VerIterator},
    records::RecordField,
    BaseDep, DepType, Dependency, Package, PackageFile, Version,
};
use oma_utils::human_bytes::HumanBytes;
use small_map::SmallMap;
use thiserror::Error;

use crate::{
    apt::{OmaAptError, OmaAptResult},
    format_description,
};

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
            comp_symbol: dep.comp_type().map(|x| x.to_string()),
            ver: dep.version().map(|x| x.to_string()),
            target_ver: dep.target_ver().ok().map(|x| x.to_string()),
            comp_ver: dep
                .comp_type()
                .and_then(|x| Some(format!("{x} {}", dep.version()?))),
        }
    }
}

#[derive(Debug)]
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
                for base_dep in dep.iter() {
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
    fn from(v: &oma_apt::DepType) -> Self {
        match v {
            oma_apt::DepType::Depends => OmaDepType::Depends,
            oma_apt::DepType::PreDepends => OmaDepType::PreDepends,
            oma_apt::DepType::Suggests => OmaDepType::Suggests,
            oma_apt::DepType::Recommends => OmaDepType::Recommends,
            oma_apt::DepType::Conflicts => OmaDepType::Conflicts,
            oma_apt::DepType::Replaces => OmaDepType::Replaces,
            oma_apt::DepType::Obsoletes => OmaDepType::Obsoletes,
            oma_apt::DepType::DpkgBreaks => OmaDepType::Breaks,
            oma_apt::DepType::Enhances => OmaDepType::Enhances,
        }
    }
}

/// UnsafePkgInfo - For storing package and version information
///
/// Note: that this should be used before the apt `cache` drop, otherwise a segfault will occur.
pub struct PkgInfo {
    pub version_raw: UniquePtr<VerIterator>,
    pub raw_pkg: UniquePtr<PkgIterator>,
}

#[derive(Debug, Error)]
#[error("BUG: pointer should is some")]
pub struct PtrIsNone;

impl PkgInfo {
    pub fn new(version: &Version, pkg: &Package) -> Result<Self, PtrIsNone> {
        // 直接传入 &Version 会遇到 version.uris 生命周期问题，所以这里传入 RawVersion，然后就地创建 Version
        let raw_pkg = unsafe { pkg.unique() }.make_safe().ok_or(PtrIsNone)?;
        let version_raw = unsafe { version.unique() }.make_safe().ok_or(PtrIsNone)?;

        Ok(Self {
            version_raw,
            raw_pkg,
        })
    }

    pub fn print_info(&self, cache: &Cache) -> OmaAptResult<()> {
        println!("Package: {}", self.raw_pkg.name());
        println!("Version: {}", self.version_raw.version());
        let ver = Version::new(
            unsafe { self.version_raw.unique() }
                .make_safe()
                .ok_or_else(|| OmaAptError::PtrIsNone(PtrIsNone))?,
            cache,
        );
        println!("Section: {}", ver.section().as_deref().unwrap_or("unknown"));
        println!(
            "Maintainer: {}",
            ver.get_record(RecordField::Maintainer)
                .unwrap_or("unknown".to_string())
        );
        println!("Installed-Size: {}", HumanBytes(ver.installed_size()));

        for (t, deps) in &self.get_deps(cache)? {
            println!("{t}: {deps}");
        }

        println!("Download-Size: {}", HumanBytes(ver.size()));
        print!("APT-Source:");

        let pkg_files = ver
            .package_files()
            .filter(|x| {
                x.index_type()
                    .map(|x| x != "Debian dpkg status file")
                    .unwrap_or(true)
            })
            .collect::<Vec<_>>();

        print_pkg_files(pkg_files);

        println!(
            "Description: {}",
            format_description(&ver.description().unwrap_or("No description".to_string())).0
        );

        Ok(())
    }

    pub fn get_deps(
        &self,
        cache: &Cache,
    ) -> OmaAptResult<SmallMap<9, OmaDepType, OmaDependencyGroup>> {
        let mut map = SmallMap::new();
        Version::new(
            unsafe { self.version_raw.unique() }
                .make_safe()
                .ok_or_else(|| OmaAptError::PtrIsNone(PtrIsNone))?,
            cache,
        )
        .depends_map()
        .iter()
        .map(|(x, y)| (OmaDepType::from(x), OmaDependency::map_deps(y)))
        .for_each(|(x, y)| {
            map.insert(x, y);
        });

        Ok(map)
    }

    pub fn get_rdeps(
        &self,
        cache: &Cache,
    ) -> OmaAptResult<SmallMap<9, OmaDepType, OmaDependencyGroup>> {
        let mut map = SmallMap::new();
        Package::new(
            cache,
            unsafe { self.raw_pkg.unique() }
                .make_safe()
                .ok_or_else(|| OmaAptError::PtrIsNone(PtrIsNone))?,
        )
        .rdepends()
        .iter()
        .map(|(x, y)| (OmaDepType::from(x), OmaDependency::map_deps(y)))
        .for_each(|(x, y)| {
            map.insert(x, y);
        });

        Ok(map)
    }
}

fn print_pkg_files(pkg_files: Vec<PackageFile>) {
    for i in &pkg_files {
        let index = i.index_file();

        if pkg_files.len() == 1 {
            print!(" ");
        } else {
            print!("  ");
        }

        print!("{}", index.archive_uri(""));

        if let Some(archive) = i.archive() {
            print!(" {}", archive);
        }

        if let Some(comp) = i.component() {
            print!("/{}", comp);
        }

        print!(" ");

        if let Some(arch) = i.arch() {
            print!("{}", arch);
        }

        print!(" ");

        if let Some(f) = i.index_type() {
            let f = match f {
                "Debian Package Index" => "Packages",
                "Debian Translation Index" => "Translation",
                _ => "",
            };
            print!("{}", f);
        }
    }

    println!();
}

#[test]
fn test_pkginfo_display() {
    use oma_apt::new_cache;
    let cache = new_cache!().unwrap();
    let pkg = cache.get("apt").unwrap();
    let version = pkg.candidate().unwrap();
    let info = PkgInfo::new(&version, &pkg).unwrap();
    info.print_info(&cache).unwrap();
}
