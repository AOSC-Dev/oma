use std::{collections::HashMap, io::IsTerminal};

use rust_apt::{
    cache::{Cache, Upgrade},
    new_cache,
    package::{Package, Version},
    records::RecordField,
};

use crate::{
    operation::{InstallEntry, OmaOperation, OperationEntry, RemoveEntry, RemoveTag},
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
    #[error("Package: {0} is essential.")]
    PkgIsEssential(String),
    #[error("Package: {0} is no candidate.")]
    PkgNoCandidate(String),
    #[error("Package: {0} has no SHA256 checksum.")]
    PkgNoChecksum(String),
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

    pub fn install(&self, pkgs: Vec<PkgInfo>, reinstall: bool) -> OmaAptResult<()> {
        for pkg in pkgs {
            mark_install(&self.cache, pkg, reinstall)?;
        }

        Ok(())
    }

    pub fn remove(&self, pkgs: Vec<PkgInfo>, purge: bool, protect: bool) -> OmaAptResult<()> {
        for pkg in pkgs {
            let pkg = Package::new(&self.cache, pkg.raw_pkg.unique());
            if pkg.is_essential() {
                if protect {
                    return Err(OmaAptError::PkgIsEssential(pkg.name().to_string()));
                } else {
                    if std::io::stdout().is_terminal() {
                        todo!()
                    } else {
                        return Err(OmaAptError::PkgIsEssential(pkg.name().to_string()));
                    }
                }
            }
            pkg.mark_delete(purge);
        }

        Ok(())
    }

    pub fn commit(self) -> OmaAptResult<()> {
        todo!()
    }

    pub fn into_operation_map(&self) -> OmaAptResult<HashMap<OmaOperation, Vec<OperationEntry>>> {
        let mut res = HashMap::new();
        let changes = self.cache.get_changes(false)?;

        res.insert(OmaOperation::Install, vec![]);
        res.insert(OmaOperation::Upgrade, vec![]);
        res.insert(OmaOperation::ReInstall, vec![]);
        res.insert(OmaOperation::Remove, vec![]);
        res.insert(OmaOperation::Downgrade, vec![]);

        for pkg in changes {
            if pkg.marked_install() {
                let cand = pkg
                    .candidate()
                    .take()
                    .ok_or_else(|| OmaAptError::PkgNoCandidate(pkg.name().to_string()))?;

                let uri = cand.uris().collect::<Vec<_>>();
                let version = cand.version();
                let checksum = cand
                    .get_record(RecordField::SHA256)
                    .ok_or_else(|| OmaAptError::PkgNoChecksum(pkg.name().to_string()))?;

                let size = cand.size();

                let install_entry = InstallEntry::new(
                    pkg.name().to_string(),
                    None,
                    version.to_string(),
                    None,
                    size,
                    uri,
                    checksum,
                );

                res.get_mut(&OmaOperation::Install)
                    .unwrap()
                    .push(OperationEntry::Install(install_entry));

                // If the package is marked install then it will also
                // show up as marked upgrade, downgrade etc.
                // Check this first and continue.
                continue;
            }

            if pkg.marked_upgrade() {
                let install_entry = pkg_delta(&pkg)?;

                res.get_mut(&OmaOperation::Upgrade)
                    .unwrap()
                    .push(OperationEntry::Install(install_entry));
            }

            if pkg.marked_delete() {
                let name = pkg.name();
                let is_purge = pkg.marked_purge();

                let mut tags = vec![];
                if is_purge {
                    tags.push(RemoveTag::Purge);
                }
                // TODO: autoremove

                let installed = pkg.installed().unwrap();
                let version = installed.version();
                let size = installed.size();

                let remove_entry =
                    RemoveEntry::new(name.to_string(), version.to_owned(), size, tags);

                res.get_mut(&OmaOperation::Remove)
                    .unwrap()
                    .push(OperationEntry::Remove(remove_entry));
            }

            if pkg.marked_reinstall() {
                let version = pkg.installed().unwrap();

                let checksum = version
                    .get_record(RecordField::SHA256)
                    .ok_or_else(|| OmaAptError::PkgNoChecksum(pkg.name().to_string()))?;

                let install_entry = InstallEntry::new(
                    pkg.name().to_string(),
                    None,
                    version.version().to_string(),
                    Some(version.installed_size()),
                    version.installed_size(),
                    version.uris().collect(),
                    checksum,
                );

                res.get_mut(&OmaOperation::ReInstall)
                    .unwrap()
                    .push(OperationEntry::Install(install_entry))
            }

            if pkg.marked_downgrade() {
                let install_entry = pkg_delta(&pkg)?;

                res.get_mut(&OmaOperation::Downgrade)
                    .unwrap()
                    .push(OperationEntry::Install(install_entry));
            }
        }

        Ok(res)
    }
}

fn pkg_delta(new_pkg: &Package) -> OmaAptResult<InstallEntry> {
    let cand = new_pkg
        .candidate()
        .take()
        .ok_or_else(|| OmaAptError::PkgNoCandidate(new_pkg.name().to_string()))?;

    let new_version = cand.version();
    let installed = new_pkg.installed().unwrap();
    let old_version = installed.version();

    let checksum = cand
        .get_record(RecordField::SHA256)
        .ok_or_else(|| OmaAptError::PkgNoChecksum(new_pkg.name().to_string()))?;

    let install_entry = InstallEntry::new(
        new_pkg.name().to_string(),
        Some(old_version.to_string()),
        new_version.to_owned(),
        Some(installed.installed_size()),
        cand.installed_size(),
        cand.uris().collect::<Vec<_>>(),
        checksum,
    );

    Ok(install_entry)
}

pub fn select_pkg(keywords: Vec<&str>, cache: &Cache) -> OmaAptResult<Vec<PkgInfo>> {
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
