use rust_apt::{
    cache::{Cache, Upgrade},
    new_cache,
    package::{Package, Version},
};

use crate::{
    pkginfo::PkgInfo,
    query::{OmaDatabase, OmaDatabaseError},
};

pub struct OmaApt {
    cache: Cache,
}

#[derive(Debug, thiserror::Error)]
pub enum OmaAptError {
    #[error(transparent)]
    RustApt(#[from] rust_apt::util::Exception),
    #[error(transparent)]
    OmaDatabaseError(#[from] OmaDatabaseError),
    #[error("Failed to mark reinstall pkg: {0}")]
    MarkReinstallError(String),
    #[error("Dep issue")]
    DependencyIssue,
}

type OmaAptResult<T> = Result<T, OmaAptError>;

impl OmaApt {
    pub fn new() -> OmaAptResult<Self> {
        Ok(Self {
            cache: new_cache!()?,
        })
    }

    pub fn upgrade(&self) -> OmaAptResult<()> {
        self.cache.upgrade(&Upgrade::FullUpgrade)?;

        Ok(())
    }

    pub fn install(&self, keywords: Vec<&str>, reinstall: bool) -> OmaAptResult<()> {
        let pkgs = select_pkg(keywords, &self.cache)?;

        for pkg in pkgs {
            mark_install(&self.cache, pkg, reinstall)?;
        }

        Ok(())
    }
}

fn select_pkg(keywords: Vec<&str>, cache: &Cache) -> OmaAptResult<Vec<PkgInfo>> {
    let db = OmaDatabase::new(cache)?;
    let mut pkgs = vec![];
    for keyword in keywords {
        pkgs.extend(match keyword {
            x if x.ends_with(".deb") => db.query_local_glob(x)?,
            x if x.split_once('/').is_some() => db.query_from_branch(x, true)?,
            x if x.split_once('=').is_some() => vec![db.query_from_version(x)?],
            x => db.query_from_glob(x, true)?,
        });
    }

    Ok(pkgs)
}

fn mark_install(cache: &Cache, pkginfo: PkgInfo, reinstall: bool) -> OmaAptResult<()> {
    let pkg = pkginfo.raw_pkg;
    let version = pkginfo.version_raw;
    let ver = Version::new(version, cache);
    let pkg = Package::new(cache, pkg);
    ver.set_candidate();

    if pkg.installed().as_ref() == Some(&ver) && !reinstall {
        tracing::info!("already-installed");

        return Ok(());
    } else if pkg.installed().as_ref() == Some(&ver) && reinstall {
        if ver.uris().next().is_none() {
            return Err(OmaAptError::MarkReinstallError(pkg.name().to_string()));
        }
        pkg.mark_reinstall(true);
    } else {
        pkg.mark_install(true, true);
        if !pkg.marked_install() && !pkg.marked_downgrade() && !pkg.marked_upgrade() {
            // apt 会先就地检查这个包的表面依赖是否满足要求，如果不满足则直接返回错误，而不是先交给 resolver
            // TODO: 依赖信息显示
            tracing::error!("Dep issue: {}", pkg.name());
            return Err(OmaAptError::DependencyIssue);
        }
    }

    pkg.protect();

    Ok(())
}
