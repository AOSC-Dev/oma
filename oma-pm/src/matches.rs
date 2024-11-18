use std::path::Path;

use bon::{builder, Builder};
use cxx::UniquePtr;
use glob_match::glob_match;
use oma_apt::{
    cache::{Cache, PackageSort},
    error::{AptError, AptErrors},
    raw::{IntoRawIter, PkgIterator},
    records::RecordField,
    Package, Version,
};
use oma_utils::url_no_escape::url_no_escape;
use tracing::{debug, info};

use crate::pkginfo::{OmaPackage, PtrIsNone};

#[derive(Debug, thiserror::Error)]
pub enum PackagesMatcherError {
    #[error(transparent)]
    AptErrors(#[from] AptErrors),
    #[error(transparent)]
    AptError(#[from] AptError),
    #[error(transparent)]
    AptCxxException(#[from] cxx::Exception),
    #[error("Invalid pattern: {0}")]
    InvalidPattern(String),
    #[error("Can not find package {0} from database")]
    NoPackage(String),
    #[error("Pkg {0} has no version {1}")]
    NoVersion(String, String),
    #[error("Pkg {0} No candidate")]
    NoCandidate(String),
    #[error("Can not find path for local package {0}")]
    NoPath(String),
    #[error(transparent)]
    PtrIsNone(#[from] PtrIsNone),
}

pub enum SearchEngine {
    Indicium(Box<dyn Fn(usize)>),
    Strsim,
    Text,
}

#[derive(Builder)]
pub struct PackagesMatcher<'a> {
    cache: &'a Cache,
    #[builder(default = true)]
    filter_candidate: bool,
    #[builder(default = false)]
    select_dbg: bool,
    #[builder(default = false)]
    filter_downloadable_candidate: bool,
    native_arch: &'a str,
}

pub type PackagesMatcherResult<T> = Result<T, PackagesMatcherError>;

impl<'a> PackagesMatcher<'a> {
    pub fn match_pkgs(
        &self,
        keywords: impl IntoIterator<Item = &'a str>,
    ) -> PackagesMatcherResult<(Vec<OmaPackage>, Vec<String>)> {
        let mut pkgs = vec![];
        let mut no_result = vec![];
        for keyword in keywords {
            let res = match keyword {
                x if x.ends_with(".deb") => self.match_local_glob(x)?,
                x if x.split_once('/').is_some() => self.match_from_branch(x)?,
                x if x.split_once('=').is_some() => self.match_from_version(x)?,
                x => self.match_from_glob(x)?,
            };

            for i in &res {
                debug!("{} {}", i.raw_pkg.fullname(true), i.version_raw.version());
            }

            if res.is_empty() {
                no_result.push(keyword.to_string());
                continue;
            }

            pkgs.extend(res);
        }

        Ok((pkgs, no_result))
    }

    /// Query package from give local file glob
    pub fn match_local_glob(&self, file_glob: &str) -> PackagesMatcherResult<Vec<OmaPackage>> {
        let mut res = vec![];
        let sort = PackageSort::default().only_virtual();

        let glob = self
            .cache
            .packages(&sort)
            .filter(|x| glob_match::glob_match(file_glob, x.name()));

        for i in glob {
            let real_pkg = real_pkg(&i);
            if let Some(real_pkg) = real_pkg {
                let pkg = Package::new(self.cache, real_pkg);
                let path = url_no_escape(&format!(
                    "file:{}",
                    Path::new(i.name())
                        .canonicalize()
                        .map_err(|_| PackagesMatcherError::NoPath(pkg.fullname(true)))?
                        .to_str()
                        .unwrap_or(pkg.name())
                ));

                let versions = pkg.versions().collect::<Vec<_>>();

                for ver in &versions {
                    let info = OmaPackage::new(ver, &pkg);

                    let has = ver.uris().any(|x| url_no_escape(&x) == path);
                    if has {
                        res.push(info);
                    }
                }
            }
        }

        Ok(res.into_iter().flatten().collect())
    }

    /// Query package from give glob (like: apt*)
    pub fn match_from_glob(&self, glob: &str) -> PackagesMatcherResult<Vec<OmaPackage>> {
        let mut res = vec![];
        let sort = PackageSort::default().include_virtual();

        if glob == "266" {
            info!("吃我一拳！！！");
        }

        let pkgs = self.cache.packages(&sort).filter(|x| {
            if glob.contains(':') {
                glob_match(glob, &x.fullname(false))
            } else {
                glob_match(glob, x.name()) && x.arch() == self.native_arch
            }
        });

        let pkgs = pkgs
            .filter_map(|x| real_pkg(&x))
            .map(|x| Package::new(self.cache, x));

        for pkg in pkgs {
            debug!("Select pkg: {}", pkg.fullname(true));
            let versions = pkg.versions().collect::<Vec<_>>();
            debug!("Versions: {:?}", versions);
            let mut candidated = false;
            for ver in versions {
                let pkginfo = OmaPackage::new(&ver, &pkg)?;
                let has_dbg = has_dbg(self.cache, &pkg, &ver);

                let is_cand = pkg.candidate().map(|x| x == ver).unwrap_or(false);

                debug!("version: {}, is cand: {}", ver, is_cand);

                if self.filter_candidate && is_cand {
                    if !self.filter_downloadable_candidate || ver.is_downloadable() {
                        // 存在 Packages 文件中版本相同、路径相同、内容不同的情况，因此一个包可能有两个 candidate 对象
                        // 这里只上传其中一个
                        if !candidated {
                            res.push(pkginfo);
                        }

                        candidated = true;
                    } else {
                        let ver = pkg.versions().find(|x| x.is_downloadable());

                        if let Some(ver) = ver {
                            res.push(OmaPackage::new(&ver, &pkg)?);
                        }
                    }
                } else if !self.filter_candidate {
                    res.push(pkginfo);
                }

                if has_dbg && self.select_dbg && (is_cand || !self.filter_candidate) {
                    self.match_debug_packages(&pkg, &ver, &mut res)?;
                }
            }
        }

        // 确保数组第一个是 candidate version
        if !self.filter_candidate {
            let candidate_list = res
                .iter()
                .enumerate()
                .filter_map(|(idx, pkg)| {
                    if pkg.is_candidate_version(self.cache) {
                        Some(idx)
                    } else {
                        None
                    }
                })
                .collect::<Vec<_>>();

            for idx in candidate_list {
                let pkg = res.remove(idx);
                res.insert(0, pkg);
            }
        }

        Ok(res)
    }

    /// Query package from give package and version (like: apt=2.5.4)
    pub fn match_from_version(&self, pat: &str) -> PackagesMatcherResult<Vec<OmaPackage>> {
        let (pkgname, version_str) = pat
            .split_once('=')
            .ok_or_else(|| PackagesMatcherError::InvalidPattern(pat.to_string()))?;

        let pkg = self
            .cache
            .get(pkgname)
            .ok_or_else(|| PackagesMatcherError::NoPackage(pat.to_string()))?;

        let version = pkg.get_version(version_str).ok_or_else(|| {
            PackagesMatcherError::NoVersion(pkgname.to_string(), version_str.to_string())
        })?;

        let mut res = vec![];

        let pkginfo = OmaPackage::new(&version, &pkg)?;
        let has_dbg = has_dbg(self.cache, &pkg, &version);

        res.push(pkginfo);

        if has_dbg && self.select_dbg {
            self.match_debug_packages(&pkg, &version, &mut res)?;
        }

        Ok(res)
    }

    /// Query package from give package and branch (like: apt/stable)
    pub fn match_from_branch(&self, pat: &str) -> PackagesMatcherResult<Vec<OmaPackage>> {
        let mut res = vec![];
        let (pkgname, branch) = pat
            .split_once('/')
            .ok_or_else(|| PackagesMatcherError::InvalidPattern(pat.to_string()))?;

        let pkg = self
            .cache
            .get(pkgname)
            .ok_or_else(|| PackagesMatcherError::NoPackage(pat.to_string()))?;

        let mut sort = vec![];

        for i in pkg.versions() {
            let item = i.get_record(RecordField::Filename);

            if let Some(item) = item {
                if item.split('/').nth(1) == Some(branch) {
                    sort.push(i)
                }
            }
        }

        sort.sort_by(|x, y| oma_apt::util::cmp_versions(x.version(), y.version()));

        if self.filter_candidate {
            let version = sort.last();
            if let Some(version) = version {
                let pkginfo = OmaPackage::new(version, &pkg)?;
                let has_dbg = has_dbg(self.cache, &pkg, version);

                if has_dbg && self.select_dbg {
                    self.match_debug_packages(&pkg, version, &mut res)?;
                }

                res.push(pkginfo);
            }
        } else {
            for i in sort {
                let pkginfo = OmaPackage::new(&i, &pkg)?;
                let has_dbg = has_dbg(self.cache, &pkg, &i);

                if has_dbg && self.select_dbg {
                    self.match_debug_packages(&pkg, &i, &mut res)?;
                }

                res.push(pkginfo);
            }
        }

        Ok(res)
    }

    /// Select -dpg package
    fn match_debug_packages(
        &self,
        pkg: &Package,
        version: &Version,
        res: &mut Vec<OmaPackage>,
    ) -> PackagesMatcherResult<()> {
        let dbg_pkg_name = format!("{}-dbg:{}", pkg.name(), version.arch());
        let dbg_pkg = self.cache.get(&dbg_pkg_name);
        let version_str = version.version();

        if let Some(dbg_pkg) = dbg_pkg {
            let dbg_ver = dbg_pkg.get_version(version_str);
            if let Some(dbg_ver) = dbg_ver {
                let pkginfo_dbg = OmaPackage::new(&dbg_ver, &dbg_pkg)?;
                res.push(pkginfo_dbg);
            }
        }

        Ok(())
    }

    /// Find mirror candidate and downloadable package version.
    pub fn find_candidate_by_pkgname(&self, pkg: &str) -> PackagesMatcherResult<OmaPackage> {
        if let Some(pkg) = self.cache.get(pkg) {
            // FIXME: candidate 版本不一定是源中能下载的版本
            // 所以要一个个版本遍历直到找到能下载的版本中最高的版本
            for version in pkg.versions() {
                if version.is_downloadable() {
                    let pkginfo = OmaPackage::new(&version, &pkg)?;
                    debug!(
                        "Pkg: {} selected version: {}",
                        pkg.fullname(true),
                        version.version(),
                    );
                    return Ok(pkginfo);
                }
            }
        }

        Err(PackagesMatcherError::NoCandidate(pkg.to_string()))
    }
}

/// Get real pkg from real pkg or virtual package
pub fn real_pkg(pkg: &Package) -> Option<UniquePtr<PkgIterator>> {
    if !pkg.has_versions() {
        if let Some(provide) = pkg.provides().next() {
            return unsafe { provide.target_pkg() }.make_safe();
        }
    }

    unsafe { pkg.unique() }.make_safe()
}

pub fn has_dbg(cache: &Cache, pkg: &Package<'_>, ver: &Version) -> bool {
    let dbg_pkg = format!("{}-dbg:{}", pkg.name(), ver.arch());
    let dbg_pkg = cache.get(&dbg_pkg);

    let has_dbg = if let Some(dbg_pkg) = dbg_pkg {
        dbg_pkg.versions().any(|x| x.version() == ver.version())
    } else {
        false
    };

    has_dbg
}

#[cfg(test)]
mod test {
    use crate::test::TEST_LOCK;

    use super::PackagesMatcher;
    use oma_apt::new_cache;
    use oma_utils::dpkg::dpkg_arch;

    #[test]
    fn test_glob_search() {
        let _lock = TEST_LOCK.lock().unwrap();
        let cache = new_cache!().unwrap();
        let arch = dpkg_arch("/").unwrap();
        let matcher = PackagesMatcher::builder()
            .cache(&cache)
            .filter_candidate(true)
            .filter_downloadable_candidate(false)
            .select_dbg(false)
            .native_arch(&arch)
            .build();

        let res_filter = matcher.match_from_glob("apt*").unwrap();

        let matcher = PackagesMatcher::builder()
            .cache(&cache)
            .filter_candidate(false)
            .filter_downloadable_candidate(false)
            .select_dbg(false)
            .native_arch(&arch)
            .build();

        let res = matcher.match_from_glob("apt*").unwrap();

        for i in res_filter {
            i.pkg_info(&cache).unwrap();
        }

        println!("---\n");

        for i in res {
            i.pkg_info(&cache).unwrap();
        }
    }

    #[test]
    fn test_virtual_pkg_search() {
        let _lock = TEST_LOCK.lock().unwrap();
        let cache = new_cache!().unwrap();
        let arch = dpkg_arch("/").unwrap();

        let matcher = PackagesMatcher::builder()
            .cache(&cache)
            .filter_candidate(true)
            .filter_downloadable_candidate(false)
            .select_dbg(false)
            .native_arch(&arch)
            .build();

        let res_filter = matcher.match_from_glob("telegram").unwrap();

        for i in res_filter {
            i.pkg_info(&cache).unwrap();
        }
    }

    #[test]
    fn test_branch_search() {
        let _lock = TEST_LOCK.lock().unwrap();
        let cache = new_cache!().unwrap();
        let arch = dpkg_arch("/").unwrap();

        let matcher = PackagesMatcher::builder()
            .cache(&cache)
            .filter_candidate(true)
            .filter_downloadable_candidate(false)
            .select_dbg(false)
            .native_arch(&arch)
            .build();

        let res_filter = matcher.match_from_branch("apt/stable").unwrap();

        for i in res_filter {
            i.pkg_info(&cache).unwrap();
        }
    }
}
