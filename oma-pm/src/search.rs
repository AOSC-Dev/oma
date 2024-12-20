use ahash::{AHashMap, RandomState};
use glob_match::glob_match;
use indexmap::map::Entry;
use indicium::simple::{Indexable, SearchIndex};
use memchr::memmem;
use oma_apt::{
    cache::{Cache, PackageSort},
    error::{AptError, AptErrors},
    raw::IntoRawIter,
    Package,
};
use serde::{Deserialize, Serialize};
use std::{
    fmt::Debug,
    fs,
    io::{self, BufReader, ErrorKind},
};
use tracing::warn;

type IndexSet<T> = indexmap::IndexSet<T, RandomState>;
type IndexMap<K, V> = indexmap::IndexMap<K, V, RandomState>;

use crate::{
    matches::has_dbg,
    pkginfo::{OmaPackage, PtrIsNone},
};

#[derive(PartialEq, Eq, Debug, Clone, Copy, Serialize, Deserialize)]
pub enum PackageStatus {
    Avail,
    Installed,
    Upgrade,
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

#[derive(Serialize, Deserialize)]
pub struct SearchEntry {
    name: String,
    description: String,
    provides: IndexSet<String>,
    has_dbg: bool,
    #[cfg(feature = "aosc")]
    section_is_base: bool,
}

impl Debug for SearchEntry {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SearchEntry")
            .field("pkgname", &self.name)
            .field("description", &self.description)
            .field("provides", &self.provides)
            .field("has_dbg", &self.has_dbg)
            .finish()
    }
}

impl Indexable for SearchEntry {
    fn strings(&self) -> Vec<String> {
        let mut v = vec![self.name.clone(), self.description.clone()];
        let provides = self.provides.clone().into_iter();
        v.extend(provides);
        v
    }
}

#[derive(Debug, thiserror::Error)]
pub enum OmaSearchError {
    #[error(transparent)]
    AptErrors(#[from] AptErrors),
    #[error(transparent)]
    AptError(#[from] AptError),
    #[error(transparent)]
    AptCxxException(#[from] cxx::Exception),
    #[error("No result found: {0}")]
    NoResult(String),
    #[error("Failed to get candidate version: {0}")]
    FailedGetCandidate(String),
    #[error(transparent)]
    PtrIsNone(#[from] PtrIsNone),
}

pub type OmaSearchResult<T> = Result<T, OmaSearchError>;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SearchResult {
    pub name: String,
    pub desc: String,
    pub old_version: Option<String>,
    pub new_version: String,
    pub full_match: bool,
    pub dbg_package: bool,
    pub status: PackageStatus,
    #[cfg(feature = "aosc")]
    pub is_base: bool,
}

pub struct IndiciumSearch<'a> {
    cache: &'a Cache,
    pkg_map: IndexMap<String, SearchEntry>,
    index: SearchIndex<String>,
}

pub trait OmaSearch {
    fn search(&self, query: &str) -> OmaSearchResult<Vec<SearchResult>>;
}

impl OmaSearch for IndiciumSearch<'_> {
    fn search(&self, query: &str) -> OmaSearchResult<Vec<SearchResult>> {
        let mut search_res = vec![];
        let query = query.to_lowercase();
        let res = self.index.search(&query);

        if res.is_empty() {
            return Err(OmaSearchError::NoResult(query));
        }

        for i in res {
            let Some(entry) = self.search_result(i, Some(&query))? else {
                continue;
            };
            search_res.push(entry);
        }

        search_res.sort_by(|a, b| b.status.cmp(&a.status));

        for i in 0..search_res.len() {
            if search_res[i].full_match {
                let i = search_res.remove(i);
                search_res.insert(0, i);
            }
        }

        Ok(search_res)
    }
}

impl<'a> IndiciumSearch<'a> {
    pub fn new(cache: &'a Cache, progress: impl Fn(usize)) -> OmaSearchResult<Self> {
        let f: Option<IndexMap<String, SearchEntry>> =
            fs::File::open("/var/cache/oma/search_index_cache")
                .ok()
                .map(BufReader::new)
                .and_then(|reader| bincode::deserialize_from(reader).ok());

        if let Some(f) = f {
            let mut search_index: SearchIndex<String> = SearchIndex::default();

            f.iter().enumerate().for_each(|(index, (key, value))| {
                search_index.insert(key, value);
                progress(index);
            });

            Ok(Self {
                cache,
                pkg_map: f,
                index: search_index,
            })
        } else {
            make_search_cache(cache, progress)
        }
    }

    pub fn search_result(
        &self,
        i: &str,
        query: Option<&str>,
    ) -> Result<Option<SearchResult>, OmaSearchError> {
        let entry = self.pkg_map.get(i).unwrap();
        let search_name = entry.name.clone();
        let desc = entry.description.clone();
        let has_dbg = entry.has_dbg;

        let Some(pkg) = self.cache.get(&search_name) else {
            return Ok(None);
        };

        let full_match = if let Some(query) = query {
            query == search_name || entry.provides.iter().any(|x| x == query)
        } else {
            false
        };

        let status = if pkg.is_upgradable() {
            PackageStatus::Upgrade
        } else if pkg.is_installed() {
            PackageStatus::Installed
        } else {
            PackageStatus::Avail
        };

        let old_version = if status != PackageStatus::Upgrade {
            None
        } else {
            pkg.installed().map(|x| x.version().to_string())
        };

        let new_version = pkg
            .candidate()
            .map(|x| x.version().to_string())
            .ok_or_else(|| OmaSearchError::FailedGetCandidate(pkg.fullname(true)))?;

        #[cfg(feature = "aosc")]
        let is_base = entry.section_is_base;

        Ok(Some(SearchResult {
            name: pkg.fullname(true),
            desc,
            old_version,
            new_version,
            full_match,
            dbg_package: has_dbg,
            status,
            #[cfg(feature = "aosc")]
            is_base,
        }))
    }
}

pub fn make_search_cache(
    cache: &Cache,
    progress: impl Fn(usize),
) -> Result<IndiciumSearch<'_>, OmaSearchError> {
    let sort = PackageSort::default().include_virtual();
    let packages = cache.packages(&sort);

    let mut pkg_map = IndexMap::with_hasher(RandomState::new());

    for (i, pkg) in packages.enumerate() {
        let name = pkg.fullname(true);
        progress(i);

        if name.contains("-dbg") {
            continue;
        }

        if let Some(cand) = pkg.candidate() {
            if let Entry::Vacant(e) = pkg_map.entry(name.clone()) {
                e.insert(SearchEntry {
                    name,
                    description: cand
                        .summary()
                        .unwrap_or_else(|| "No description".to_string()),
                    provides: pkg.provides().map(|x| x.to_string()).collect(),
                    has_dbg: has_dbg(cache, &pkg, &cand),
                    #[cfg(feature = "aosc")]
                    section_is_base: cand.section().map(|x| x == "Bases").unwrap_or(false),
                });
            }
        } else {
            // Provides
            let mut real_pkgs = vec![];
            for i in pkg.provides() {
                real_pkgs.push((
                    i.name().to_string(),
                    unsafe { i.target_pkg() }
                        .make_safe()
                        .ok_or(OmaSearchError::PtrIsNone(PtrIsNone))?,
                ));
            }

            for (provide, i) in real_pkgs {
                let pkg = Package::new(cache, i);
                let name = pkg.fullname(true);

                if let Some(cand) = pkg.candidate() {
                    pkg_map
                        .entry(name.clone())
                        .and_modify(|x| {
                            if !x.provides.contains(&provide) {
                                x.provides.insert(provide.clone());
                            }
                        })
                        .or_insert(SearchEntry {
                            name,
                            description: cand
                                .summary()
                                .unwrap_or_else(|| "No description".to_string()),
                            provides: {
                                let mut set = IndexSet::with_hasher(RandomState::new());
                                set.insert(provide.clone());
                                set
                            },
                            has_dbg: has_dbg(cache, &pkg, &cand),
                            #[cfg(feature = "aosc")]
                            section_is_base: cand.section().map(|x| x == "Bases").unwrap_or(false),
                        });
                }
            }
        }
    }

    let mut search_index: SearchIndex<String> = SearchIndex::default();

    pkg_map
        .iter()
        .for_each(|(key, value)| search_index.insert(key, value));

    let ty = Ok(()).and_then(|_| -> io::Result<()> {
        let pkg_map_cache =
            bincode::serialize(&pkg_map).map_err(|e| io::Error::new(ErrorKind::Other, e))?;
        fs::create_dir_all("/var/cache/oma/")?;
        fs::write("/var/cache/oma/search_index_cache", &pkg_map_cache)?;
        Ok(())
    });

    if let Err(e) = ty {
        warn!("Failed to write search index to cache: {e}");
    }

    Ok(IndiciumSearch {
        cache,
        pkg_map,
        index: search_index,
    })
}

pub struct StrSimSearch<'a> {
    cache: &'a Cache,
}

impl OmaSearch for StrSimSearch<'_> {
    fn search(&self, query: &str) -> Result<Vec<SearchResult>, OmaSearchError> {
        let pkgs = self
            .cache
            .packages(&PackageSort::default().include_virtual());

        let mut res = AHashMap::new();

        for pkg in pkgs {
            let name = pkg.fullname(true);
            if let Some(cand) = pkg.candidate() {
                if memmem::find(name.as_bytes(), query.as_bytes()).is_some()
                    && !name.ends_with("-dbg")
                    && !res.contains_key(&name)
                {
                    let oma_pkg = OmaPackage::new(&cand, &pkg)?;
                    res.insert(
                        name.clone(),
                        (oma_pkg, cand.is_installed(), pkg.is_upgradable(), false),
                    );
                }

                if cand
                    .description()
                    .is_some_and(|x| memmem::find(x.as_bytes(), query.as_bytes()).is_some())
                    && !res.contains_key(&name)
                    && !name.ends_with("-dbg")
                {
                    let oma_pkg = OmaPackage::new(&cand, &pkg)?;
                    res.insert(
                        name.clone(),
                        (oma_pkg, cand.is_installed(), pkg.is_upgradable(), false),
                    );
                }
            } else if name == query && pkg.has_provides() {
                let real_pkgs = pkg.provides().flat_map(|x| {
                    unsafe { x.target_pkg() }
                        .make_safe()
                        .ok_or(OmaSearchError::PtrIsNone(PtrIsNone))
                });
                for pkg in real_pkgs {
                    let pkg = Package::new(self.cache, pkg);
                    if let Some(cand) = pkg.candidate() {
                        let oma_pkg = OmaPackage::new(&cand, &pkg)?;

                        res.insert(
                            name.clone(),
                            (oma_pkg, cand.is_installed(), pkg.is_upgradable(), true),
                        );
                    }
                }
            }
        }

        let mut res = res.into_values().collect::<Vec<_>>();

        res.sort_unstable_by(|x, y| {
            let x_score = Self::pkg_score(query, &x.0, x.3);
            let y_score = Self::pkg_score(query, &y.0, y.3);

            let c = y_score.cmp(&x_score);

            if c == std::cmp::Ordering::Equal {
                y.0.raw_pkg.fullname(true).cmp(&x.0.raw_pkg.fullname(true))
            } else {
                c
            }
        });

        let mut v = vec![];

        for (pkginfo, install, upgrade, _) in res {
            let pkg = Package::new(self.cache, pkginfo.raw_pkg);
            let cand = pkg
                .candidate()
                .ok_or_else(|| OmaSearchError::FailedGetCandidate(pkg.fullname(true)))?;

            let name = pkg.fullname(true);

            #[cfg(feature = "aosc")]
            let is_base = name.ends_with("-base");

            let full_match = query == name;

            v.push(SearchResult {
                name,
                desc: cand
                    .summary()
                    .unwrap_or_else(|| "No description".to_string()),
                old_version: {
                    if !upgrade {
                        None
                    } else {
                        pkg.installed().map(|x| x.version().to_string())
                    }
                },
                new_version: cand.version().to_string(),
                full_match,
                dbg_package: has_dbg(self.cache, &pkg, &cand),
                status: if upgrade {
                    PackageStatus::Upgrade
                } else if install {
                    PackageStatus::Installed
                } else {
                    PackageStatus::Avail
                },
                #[cfg(feature = "aosc")]
                is_base,
            });
        }

        v.sort_by(|a, b| b.status.cmp(&a.status));

        for i in 0..v.len() {
            if v[i].full_match {
                let i = v.remove(i);
                v.insert(0, i);
            }
        }

        Ok(v)
    }
}

impl<'a> StrSimSearch<'a> {
    pub fn new(cache: &'a Cache) -> Self {
        Self { cache }
    }

    fn pkg_score(input: &str, pkginfo: &OmaPackage, is_provide: bool) -> u16 {
        if is_provide {
            return 1000;
        }

        (strsim::jaro_winkler(&pkginfo.raw_pkg.fullname(true), input) * 1000.0) as u16
    }
}

pub struct TextSearch<'a> {
    cache: &'a Cache,
}

impl<'a> TextSearch<'a> {
    pub fn new(cache: &'a Cache) -> Self {
        Self { cache }
    }
}

impl OmaSearch for TextSearch<'_> {
    fn search(&self, query: &str) -> OmaSearchResult<Vec<SearchResult>> {
        let mut res = vec![];
        let pkgs = self.cache.packages(&PackageSort::default());

        for pkg in pkgs {
            let name = pkg.fullname(true);
            let cand = pkg.candidate();

            if (memmem::find(name.as_bytes(), query.as_bytes()).is_some()
                || glob_match(query, &name))
                && !name.ends_with("-dbg")
            {
                let full_match = query == name;

                #[cfg(feature = "aosc")]
                let is_base = name.ends_with("-base");

                let upgrade = pkg.is_upgradable();
                let installed = pkg.is_installed();
                if let Some(cand) = cand {
                    res.push(SearchResult {
                        name,
                        desc: cand
                            .summary()
                            .unwrap_or_else(|| "No description".to_string()),
                        old_version: {
                            if !pkg.is_upgradable() {
                                None
                            } else {
                                pkg.installed().map(|x| x.version().to_string())
                            }
                        },
                        new_version: cand.version().to_string(),
                        full_match,
                        dbg_package: has_dbg(self.cache, &pkg, &cand),
                        status: if upgrade {
                            PackageStatus::Upgrade
                        } else if installed {
                            PackageStatus::Installed
                        } else {
                            PackageStatus::Avail
                        },
                        #[cfg(feature = "aosc")]
                        is_base,
                    })
                }
            }
        }

        res.sort_by(|a, b| b.status.cmp(&a.status));

        for i in 0..res.len() {
            if res[i].full_match {
                let i = res.remove(i);
                res.insert(0, i);
            }
        }

        Ok(res)
    }
}

#[test]
fn test() {
    use crate::test::TEST_LOCK;
    use oma_apt::new_cache;
    let _lock = TEST_LOCK.lock().unwrap();

    let packages = std::path::Path::new(&std::env::var_os("CARGO_MANIFEST_DIR").unwrap())
        .join("test_file")
        .join("Packages");
    let cache = new_cache!(&[packages.to_string_lossy().to_string()]).unwrap();

    let searcher = IndiciumSearch::new(&cache, |_| {}).unwrap();
    let res = searcher.search("windows-nt-kernel").unwrap();
    let res2 = searcher.search("pwp").unwrap();

    for i in [res, res2] {
        assert!(i.iter().any(|x| x.name == "qaq"));
        assert!(i.iter().any(|x| x.new_version == "9999:1"));
        assert!(i.iter().any(|x| x.full_match));
        assert!(i.iter().filter(|x| x.name == "qaq").count() == 1)
    }

    let res = searcher.search("qwq").unwrap();
    let res2 = searcher.search("qwqdesktop").unwrap();

    for i in [res, res2] {
        assert!(i.iter().any(|x| x.name == "qwq-desktop"));
        assert!(i.iter().any(|x| x.new_version == "9999:114514"));
        assert!(i.iter().any(|x| x.full_match));
        assert!(i.iter().filter(|x| x.name == "qwq-desktop").count() == 1)
    }

    let res = searcher.search("owo").unwrap();
    let res = res.first().unwrap();

    assert_eq!(res.name, "owo".to_string());
    assert_eq!(res.new_version, "9999:2.6.1-2");
    assert!(res.full_match);
}
