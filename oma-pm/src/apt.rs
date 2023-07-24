use std::path::{Path, PathBuf};

use oma_console::{
    console,
    console::style,
    dialoguer::{theme::ColorfulTheme, Confirm, Input},
    info,
    writer::Writer,
};
use oma_fetch::{
    DownloadEntry, DownloadError, DownloadSource, DownloadSourceType, OmaFetcher, Summary,
};
use rust_apt::{
    cache::{Cache, Upgrade},
    new_cache,
    package::{Package, Version},
    records::RecordField,
};

pub use rust_apt::config::Config as AptConfig;

use crate::{
    operation::{InstallEntry, OmaOperation, RemoveEntry, RemoveTag},
    pkginfo::PkgInfo,
    progress::{NoProgress, OmaAptInstallProgress},
    query::{OmaDatabase, OmaDatabaseError},
};

pub struct OmaApt {
    pub cache: Cache,
    config: AptConfig,
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
    #[error("Ivaild file name: {0}")]
    InvalidFileName(String),
    #[error(transparent)]
    DownlaodError(#[from] DownloadError),
    #[error(transparent)]
    IOError(#[from] std::io::Error),
}

pub struct AptArgs {
    yes: bool,
    force_yes: bool,
    dpkg_force_confnew: bool,
    dpkg_force_all: bool,
}

impl Default for AptArgs {
    fn default() -> Self {
        Self {
            yes: false,
            force_yes: false,
            dpkg_force_confnew: false,
            dpkg_force_all: false,
        }
    }
}

type OmaAptResult<T> = Result<T, OmaAptError>;

impl OmaApt {
    pub fn new() -> OmaAptResult<Self> {
        Ok(Self {
            cache: new_cache!()?,
            config: AptConfig::new(),
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

    pub fn remove(
        &self,
        pkgs: Vec<PkgInfo>,
        purge: bool,
        protect: bool,
        cli_output: bool,
    ) -> OmaAptResult<()> {
        for pkg in pkgs {
            let pkg = Package::new(&self.cache, pkg.raw_pkg.unique());
            if pkg.is_essential() {
                if protect {
                    return Err(OmaAptError::PkgIsEssential(pkg.name().to_string()));
                } else {
                    if cli_output {
                        ask_user_do_as_i_say(&pkg)?;
                    } else {
                        return Err(OmaAptError::PkgIsEssential(pkg.name().to_string()));
                    }
                }
            }
            pkg.mark_delete(purge);
        }

        Ok(())
    }

    pub fn commit(self, network_thread: Option<usize>, args_config: AptArgs) -> OmaAptResult<()> {
        let v = self.operation_vec()?;

        let download_pkg_list = v
            .into_iter()
            .filter(|x| !matches!(x, &OmaOperation::Remove(_)))
            .map(|x| {
                let OmaOperation::Install(v) = x else { unreachable!() };
                v
            })
            .collect::<Vec<_>>();

        let tokio = tokio::runtime::Builder::new_multi_thread()
            .enable_io()
            .build()?;

        let path = self.get_archive_dir();

        let (success, failed) = tokio.block_on(async move {
            Self::download_pkgs(download_pkg_list, network_thread, &path).await
        })?;

        tracing::debug!("Success: {success:?}");
        tracing::debug!("Failed: {failed:?}");

        let mut no_progress = NoProgress::new_box();

        self.cache.get_archives(&mut no_progress)?;

        let mut progress = OmaAptInstallProgress::new_box(
            AptConfig::new(),
            args_config.yes,
            args_config.force_yes,
            args_config.dpkg_force_confnew,
            args_config.dpkg_force_all,
        );

        self.cache.do_install(&mut progress)?;

        Ok(())
    }

    async fn download_pkgs(
        download_pkg_list: Vec<InstallEntry>,
        network_thread: Option<usize>,
        download_dir: &Path,
    ) -> OmaAptResult<(Vec<Summary>, Vec<DownloadError>)> {
        let mut download_list = vec![];
        let mut total_size = 0;

        for entry in download_pkg_list {
            let uris = entry.pkg_urls();
            let sources = uris
                .into_iter()
                .map(|x| {
                    let source_type = if x.starts_with("file:") {
                        DownloadSourceType::Local
                    } else {
                        DownloadSourceType::Http
                    };

                    DownloadSource::new(x.to_string(), source_type)
                })
                .collect::<Vec<_>>();

            let filename = uris
                .first()
                .and_then(|x| x.split('/').last())
                .take()
                .ok_or_else(|| OmaAptError::InvalidFileName(entry.name().to_string()))?;

            let download_entry = DownloadEntry::new(
                sources,
                apt_style_filename(filename, entry.new_version().to_string())?,
                download_dir.to_path_buf(),
                Some(entry.checksum().to_owned()),
                true,
                Some(format!(
                    "{} {} ({})",
                    entry.name(),
                    entry.new_version(),
                    entry.arch()
                )),
            );

            total_size += entry.download_size();

            download_list.push(download_entry);
        }
        let downloader =
            OmaFetcher::new(None, true, Some(total_size), download_list, network_thread)?;

        let res = downloader.start_download().await;

        let (mut success, mut failed) = (vec![], vec![]);

        for i in res {
            match i {
                Ok(s) => success.push(s),
                Err(e) => failed.push(e),
            }
        }

        Ok((success, failed))
    }

    pub fn select_pkg(&self, keywords: Vec<&str>) -> OmaAptResult<Vec<PkgInfo>> {
        select_pkg(keywords, &self.cache)
    }

    fn get_archive_dir(&self) -> PathBuf {
        let archives_dir = self.config.get("Dir::Cache::Archives");

        let path = if let Some(archives_dir) = archives_dir {
            if !Path::new(&archives_dir).is_absolute() {
                PathBuf::from(format!("/var/cache/apt/{archives_dir}"))
            } else {
                PathBuf::from(archives_dir)
            }
        } else {
            PathBuf::from("/var/cache/apt/archives/")
        };

        path
    }

    pub fn operation_vec(&self) -> OmaAptResult<Vec<OmaOperation>> {
        let mut res = vec![];
        let changes = self.cache.get_changes(false)?;

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

                let size = cand.installed_size();

                let install_entry = InstallEntry::new(
                    pkg.name().to_string(),
                    None,
                    version.to_string(),
                    None,
                    size,
                    uri,
                    checksum,
                    pkg.arch().to_string(),
                    cand.size(),
                );

                res.push(OmaOperation::Install(install_entry));

                // If the package is marked install then it will also
                // show up as marked upgrade, downgrade etc.
                // Check this first and continue.
                continue;
            }

            if pkg.marked_upgrade() {
                let install_entry = pkg_delta(&pkg)?;

                res.push(OmaOperation::Upgrade(install_entry));
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

                res.push(OmaOperation::Remove(remove_entry));
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
                    pkg.arch().to_string(),
                    0,
                );

                res.push(OmaOperation::ReInstall(install_entry));
            }

            if pkg.marked_downgrade() {
                let install_entry = pkg_delta(&pkg)?;

                res.push(OmaOperation::Downgrade(install_entry));
            }
        }

        Ok(res)
    }
}

fn ask_user_do_as_i_say(pkg: &Package<'_>) -> Result<(), OmaAptError> {
    let writer = Writer::default();
    let theme = ColorfulTheme::default();
    let delete = Confirm::with_theme(&theme)
        .with_prompt(format!(
            "DELETE THIS PACKAGE? PACKAGE {} IS ESSENTIAL!",
            pkg.name()
        ))
        .default(false)
        .interact()?;
    if !delete {
        info!(writer, "Not confirmed.");
        return Ok(());
    }
    info!(
        writer,
        "If you are absolutely sure, please type the following: {}",
        style("Do as I say!").bold()
    );

    if Input::<String>::with_theme(&theme)
        .with_prompt("Your turn")
        .interact()?
        != "Do as I say!"
    {
        info!(writer, "Prompt answered incorrectly. Not confirmed.");
        return Ok(());
    }

    Ok(())
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
        new_pkg.arch().to_string(),
        cand.size(),
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

fn apt_style_filename(filename: &str, version: String) -> OmaAptResult<String> {
    let mut filename_split = filename.split('_');

    let package = filename_split
        .next()
        .take()
        .ok_or_else(|| OmaAptError::InvalidFileName(filename.to_owned()))?;

    let arch_deb = filename_split
        .nth(1)
        .take()
        .ok_or_else(|| OmaAptError::InvalidFileName(filename.to_owned()))?;

    let arch_deb = if arch_deb == "noarch.deb" {
        "all.deb"
    } else {
        arch_deb
    };

    let version = version.replace(':', "%3a");
    let filename = format!("{package}_{version}_{arch_deb}").replace("%2b", "+");

    Ok(filename)
}
