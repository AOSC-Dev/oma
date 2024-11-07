use std::fmt::Display;

use ahash::HashMap;
use cxx::UniquePtr;
use oma_apt::{
    cache::Cache,
    raw::{IntoRawIter, PkgIterator, VerIterator},
    records::RecordField,
    BaseDep, DepType, Dependency, Package, PackageFile, Version,
};
use oma_utils::human_bytes::HumanBytes;
use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::apt::{OmaAptError, OmaAptResult};

#[derive(Debug, Serialize, Deserialize)]
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

#[derive(Debug, Serialize, Deserialize)]
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

#[derive(Debug, Eq, PartialEq, Hash, Deserialize, Serialize)]
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

/// PkgInfo - For storing package and version information
///
/// Note: that this should be used before the apt `cache` drop, otherwise a segfault will occur.
pub struct PkgInfo {
    pub version_raw: UniquePtr<VerIterator>,
    pub raw_pkg: UniquePtr<PkgIterator>,
}

#[derive(Debug, Error)]
#[error("BUG: pointer should is some")]
pub struct PtrIsNone;

#[derive(Debug, Deserialize, Serialize)]
pub struct PackageInfo {
    package: Box<str>,
    version: Box<str>,
    section: Box<str>,
    maintainer: String,
    install_size: u64,
    dep_map: HashMap<OmaDepType, OmaDependencyGroup>,
    download_size: u64,
    apt_sources: Vec<AptSource>,
    description: String,
}

impl Display for PackageInfo {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let PackageInfo {
            package,
            version,
            section,
            maintainer,
            install_size,
            dep_map,
            download_size,
            apt_sources,
            description,
        } = self;

        writeln!(f, "Package: {}", package)?;
        writeln!(f, "Version: {}", version)?;
        writeln!(f, "Section: {}", section)?;
        writeln!(f, "Maintainer: {}", maintainer)?;
        writeln!(f, "Install-Size: {}", HumanBytes(*install_size))?;
        for (k, v) in dep_map {
            writeln!(f, "{k}: {v}")?;
        }
        writeln!(f, "Download-Size: {}", HumanBytes(*download_size))?;
        writeln!(f, "APT-Sources: {}", {
            let apt_sources_without_dpkg = apt_sources
                .iter()
                .filter(|x| x.index_type.as_deref() != Some("Debian dpkg status file"))
                .collect::<Vec<_>>();

            let mut s = String::new();

            match apt_sources_without_dpkg.len() {
                0 => s += &apt_sources[0].to_string(),
                1 => s += &apt_sources_without_dpkg[0].to_string(),
                2.. => {
                    s.push('\n');
                    s += &apt_sources_without_dpkg
                        .iter()
                        .map(|x| x.to_string())
                        .collect::<Vec<_>>()
                        .join("\n  ");
                }
            }

            s
        })?;
        writeln!(f, "description: {}", description)?;

        Ok(())
    }
}

#[derive(Debug, Deserialize, Serialize)]
pub struct AptSource {
    archive: Option<Box<str>>,
    component: Option<Box<str>>,
    arch: Option<Box<str>>,
    index_type: Option<Box<str>>,
    archive_uri: String,
}

impl From<PackageFile<'_>> for AptSource {
    fn from(value: PackageFile<'_>) -> Self {
        let index = value.index_file();
        let archive_uri = index.archive_uri("");

        Self {
            archive: value.archive().map(Box::from),
            component: value.component().map(Box::from),
            arch: value.arch().map(Box::from),
            index_type: value.index_type().map(Box::from),
            archive_uri,
        }
    }
}

impl Display for AptSource {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", &self.archive_uri)?;

        if let Some(archive) = &self.archive {
            write!(f, " {}", archive)?;
        }

        if let Some(comp) = &self.component {
            write!(f, "/{}", comp)?;
        }

        if let Some(arch) = &self.arch {
            write!(f, " {}", arch)?;
        }

        if let Some(ft) = &self.index_type {
            let ft = match ft.as_ref() {
                "Debian Package Index" => "Packages",
                "Debian Translation Index" => "Translation",
                _ => "",
            };

            write!(f, " {}", ft)?;
        }

        Ok(())
    }
}

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

    pub fn pkg_info(&self, cache: &Cache) -> OmaAptResult<PackageInfo> {
        let package: Box<str> = Box::from(self.raw_pkg.fullname(true));
        let version: Box<str> = Box::from(self.version_raw.version());
        let ver = Version::new(
            unsafe { self.version_raw.unique() }
                .make_safe()
                .ok_or_else(|| OmaAptError::PtrIsNone(PtrIsNone))?,
            cache,
        );
        let section: Box<str> = Box::from(ver.section().unwrap_or("unknown"));
        let maintainer = ver
            .get_record(RecordField::Maintainer)
            .unwrap_or_else(|| "unknown".to_string());
        let install_size = ver.installed_size();

        let deps_map = self.get_deps(cache)?;

        let download_size = ver.size();

        let pkg_files = ver.package_files().map(AptSource::from).collect::<Vec<_>>();

        let description = ver
            .description()
            .unwrap_or_else(|| "No description".to_string());

        Ok(PackageInfo {
            package,
            version,
            section,
            maintainer,
            install_size,
            dep_map: deps_map,
            download_size,
            apt_sources: pkg_files,
            description,
        })
    }

    pub fn get_deps(&self, cache: &Cache) -> OmaAptResult<HashMap<OmaDepType, OmaDependencyGroup>> {
        let map = Version::new(
            unsafe { self.version_raw.unique() }
                .make_safe()
                .ok_or_else(|| OmaAptError::PtrIsNone(PtrIsNone))?,
            cache,
        )
        .depends_map()
        .iter()
        .map(|(x, y)| (OmaDepType::from(x), OmaDependency::map_deps(y)))
        .collect::<HashMap<_, _>>();

        Ok(map)
    }

    pub fn get_rdeps(
        &self,
        cache: &Cache,
    ) -> OmaAptResult<HashMap<OmaDepType, OmaDependencyGroup>> {
        let map = Package::new(
            cache,
            unsafe { self.raw_pkg.unique() }
                .make_safe()
                .ok_or_else(|| OmaAptError::PtrIsNone(PtrIsNone))?,
        )
        .rdepends()
        .iter()
        .map(|(x, y)| (OmaDepType::from(x), OmaDependency::map_deps(y)))
        .collect::<HashMap<_, _>>();

        Ok(map)
    }
}

#[test]
fn test_pkginfo_display() {
    use crate::test::TEST_LOCK;
    use oma_apt::new_cache;

    let _lock = TEST_LOCK.lock().unwrap();
    let cache = new_cache!().unwrap();
    let pkg = cache.get("apt").unwrap();
    let version = pkg.candidate().unwrap();
    let info = PkgInfo::new(&version, &pkg).unwrap();
    let info = info.pkg_info(&cache).unwrap();
    println!("{info}");
}
