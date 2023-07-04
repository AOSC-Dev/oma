use rust_apt::{
    cache::{Cache, PackageSort},
    new_cache,
};

use crate::pkginfo::PkgInfo;

#[derive(Debug, thiserror::Error)]
pub enum OmaDatabaseError {
    #[error(transparent)]
    RustApt(#[from] rust_apt::util::Exception),
}

pub struct OmaDatabase {
    cache: Cache,
}

pub type OmaDatabaseResult<T> = Result<T, OmaDatabaseError>;

impl OmaDatabase {
    pub fn new() -> OmaDatabaseResult<Self> {
        Ok(Self {
            cache: new_cache!()?,
        })
    }

    pub fn search_from_glob(
        &self,
        glob: &str,
        filter_candidate: bool,
    ) -> OmaDatabaseResult<Vec<PkgInfo>> {
        let mut res = vec![];
        let sort = PackageSort::default().include_virtual();
        let pkgs = self
            .cache
            .packages(&sort)?
            .filter(|x| glob_match::glob_match_with_captures(glob, x.name()).is_some());

        for pkg in pkgs {
            let versions = pkg.versions();
            for ver in versions {
                let pkginfo = PkgInfo::new(&self.cache, ver.unique(), &pkg);
                if filter_candidate && pkginfo.is_candidate {
                    res.push(pkginfo);
                } else if !filter_candidate {
                    res.push(pkginfo);
                }
            }
        }

        Ok(res)
    }
}

#[test]
fn test_glob_search() {
    let db = OmaDatabase::new().unwrap();
    let res_filter = db.search_from_glob("apt*", true).unwrap();
    let res = db.search_from_glob("apt*", false).unwrap();

    for i in res_filter {
        println!("{}", i);
    }

    println!("---\n");

    for i in res {
        println!("{}", i);
    }
}
