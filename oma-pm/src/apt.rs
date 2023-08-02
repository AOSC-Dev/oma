use std::{
    path::{Path, PathBuf},
    process::Command,
};

use derive_builder::Builder;
use oma_console::{
    console::style,
    debug,
    dialoguer::{theme::ColorfulTheme, Confirm, Input},
    error, info, warn,
};
use oma_fetch::{
    DownloadEntry, DownloadError, DownloadSource, DownloadSourceType, OmaFetcher, Summary,
};
use rust_apt::{
    cache::{Cache, PackageSort, Upgrade},
    new_cache,
    package::{Package, Version},
    raw::util::raw::{apt_lock_inner, apt_unlock, apt_unlock_inner},
    records::RecordField,
    util::{apt_lock, DiskSpace},
};

pub use rust_apt::config::Config as AptConfig;

use crate::{
    operation::{
        InstallEntry, InstallEntryBuilder, InstallEntryBuilderError, InstallOperation, RemoveEntry,
        RemoveTag,
    },
    pkginfo::PkgInfo,
    progress::{NoProgress, OmaAptInstallProgress},
    query::{OmaDatabase, OmaDatabaseError},
    unmet::{find_unmet_deps, find_unmet_deps_with_markinstall, UnmetDep},
};

#[derive(Builder, Default)]
#[builder(default)]
pub struct OmaAptArgs {
    install_recommends: bool,
    install_suggests: bool,
    no_install_recommends: bool,
    no_install_suggests: bool,
}

pub struct OmaApt {
    pub cache: Cache,
    pub config: AptConfig,
    autoremove: Vec<String>,
}

#[derive(Debug, thiserror::Error)]
pub enum OmaAptError {
    #[error(transparent)]
    RustApt(#[from] rust_apt::util::Exception),
    #[error(transparent)]
    OmaDatabaseError(#[from] OmaDatabaseError),
    #[error("Failed to mark reinstall pkg: {0}")]
    MarkReinstallError(String),
    #[error("Dep issue: {0:?}")]
    DependencyIssue(Vec<UnmetDep>),
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
    #[error(transparent)]
    InstallEntryBuilderError(#[from] InstallEntryBuilderError),
    #[error("Failed to run dpkg --configure -a: {0}")]
    DpkgFailedConfigure(String),
}

#[derive(Default, Builder)]
#[builder(default)]
pub struct AptArgs {
    yes: bool,
    force_yes: bool,
    dpkg_force_confnew: bool,
    dpkg_force_all: bool,
}

impl AptArgs {
    pub fn yes(&self) -> bool {
        self.yes
    }

    pub fn force_yes(&self) -> bool {
        self.force_yes
    }

    pub fn dpkg_force_confnew(&self) -> bool {
        self.dpkg_force_confnew
    }

    pub fn dpkg_force_all(&self) -> bool {
        self.dpkg_force_all
    }
}

pub type OmaAptResult<T> = Result<T, OmaAptError>;

pub enum FilterMode {
    Default,
    Installed,
    Upgradable,
    Automatic,
    Names,
}

pub struct OmaOperation<'a> {
    pub install: Vec<InstallEntry>,
    pub remove: Vec<RemoveEntry>,
    pub disk_size: (&'a str, u64),
}

impl OmaApt {
    pub fn new(local_debs: Vec<String>, args: OmaAptArgs) -> OmaAptResult<Self> {
        let config = Self::init_config(args)?;

        Ok(Self {
            cache: new_cache!(&local_debs)?,
            config: config,
            autoremove: vec![],
        })
    }

    fn init_config(args: OmaAptArgs) -> OmaAptResult<AptConfig> {
        let config = AptConfig::new();

        let install_recommend = if args.install_recommends {
            true
        } else if args.no_install_recommends {
            false
        } else {
            match config.get("APT::Install-Recommends").as_deref() {
                Some("true") => true,
                Some("false") => false,
                _ => true,
            }
        };

        let install_suggests = if args.install_suggests {
            true
        } else if args.no_install_suggests {
            false
        } else {
            match config.get("APT::Install-Suggests").as_deref() {
                Some("true") => true,
                Some("false") => false,
                _ => false,
            }
        };

        config.set(
            "APT::Install-Recommends",
            match install_recommend {
                true => "true",
                false => "false",
            },
        );

        config.set(
            "APT::Install-Suggests",
            match install_suggests {
                true => "true",
                false => "false",
            },
        );

        Ok(config)
    }

    pub fn available_action(&self) -> OmaAptResult<(usize, usize)> {
        let sort = PackageSort::default().upgradable();
        let upgradable = self.cache.packages(&sort)?.collect::<Vec<_>>().len();

        let sort = PackageSort::default().auto_removable();
        let removable = self.cache.packages(&sort)?.collect::<Vec<_>>().len();

        Ok((upgradable, removable))
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

    pub fn check_broken(&self) -> OmaAptResult<bool> {
        let sort = PackageSort::default().installed();
        let pkgs = self.cache.packages(&sort)?;

        // let mut reinstall = vec![];

        let mut need = false;

        for pkg in pkgs {
            // current_state 的定义来自 apt 的源码:
            //    enum PkgCurrentState {NotInstalled=0,UnPacked=1,HalfConfigured=2,
            //    HalfInstalled=4,ConfigFiles=5,Installed=6,
            //    TriggersAwaited=7,TriggersPending=8};
            if pkg.current_state() != 6 {
                debug!(
                    "pkg {} current state is {}",
                    pkg.name(),
                    pkg.current_state()
                );
                need = true;
                match pkg.current_state() {
                    4 => {
                        pkg.mark_reinstall(true);
                        // reinstall.push(pkg.name().to_string());
                    }
                    _ => continue,
                }
            }
        }

        Ok(need)
    }

    pub fn download(
        &self,
        pkgs: Vec<PkgInfo>,
        network_thread: Option<usize>,
        download_dir: Option<&Path>,
    ) -> OmaAptResult<(Vec<Summary>, Vec<DownloadError>)> {
        let mut download_list = vec![];
        for pkg in pkgs {
            let name = pkg.raw_pkg.name().to_string();
            let entry = InstallEntryBuilder::default()
                .name(pkg.raw_pkg.name().to_string())
                .new_version(pkg.version_raw.version().to_string())
                .new_size(pkg.installed_size)
                .pkg_urls(pkg.apt_sources)
                .checksum(
                    pkg.checksum
                        .ok_or_else(|| OmaAptError::PkgNoChecksum(name))?,
                )
                .arch(pkg.arch)
                .download_size(pkg.download_size)
                .op(InstallOperation::Download)
                .build()?;

            download_list.push(entry);
        }

        let tokio = tokio::runtime::Builder::new_multi_thread()
            .enable_io()
            .enable_time()
            .build()?;

        let res = tokio.block_on(async move {
            Self::download_pkgs(
                download_list,
                network_thread,
                download_dir.unwrap_or(Path::new(".")),
            )
            .await
        })?;

        Ok(res)
    }

    pub fn remove(
        &mut self,
        pkgs: Vec<PkgInfo>,
        purge: bool,
        protect: bool,
        cli_output: bool,
        no_autoremove: bool,
    ) -> OmaAptResult<()> {
        for pkg in pkgs {
            let pkg = Package::new(&self.cache, pkg.raw_pkg.unique());
            if pkg.is_essential() {
                if protect {
                    return Err(OmaAptError::PkgIsEssential(pkg.name().to_string()));
                } else if cli_output {
                    if !ask_user_do_as_i_say(&pkg)? {
                        return Err(OmaAptError::PkgIsEssential(pkg.name().to_string()));
                    }
                } else {
                    return Err(OmaAptError::PkgIsEssential(pkg.name().to_string()));
                }
            }
            pkg.mark_delete(purge);
        }

        // 寻找系统有哪些不必要的软件包
        if !no_autoremove {
            self.autoremove(purge)?;
        }

        Ok(())
    }

    fn autoremove(&mut self, purge: bool) -> OmaAptResult<()> {
        let sort = PackageSort::default().installed();
        let pkgs = self.cache.packages(&sort)?;

        for pkg in pkgs {
            if pkg.is_auto_removable() {
                pkg.mark_delete(purge);

                self.autoremove.push(pkg.name().to_string());
            }
        }

        Ok(())
    }

    pub fn commit(
        self,
        network_thread: Option<usize>,
        args_config: &AptArgs,
    ) -> OmaAptResult<()> {

        let v = self.operation_vec()?;

        let download_pkg_list = v.install;

        let tokio = tokio::runtime::Builder::new_multi_thread()
            .enable_time()
            .enable_io()
            .build()?;

        let path = self.get_archive_dir();

        let (success, failed) = tokio.block_on(async move {
            Self::download_pkgs(download_pkg_list, network_thread, &path).await
        })?;

        debug!("Success: {success:?}");
        debug!("Failed: {failed:?}");

        let mut no_progress = NoProgress::new_box();

        if let Err(e) = apt_lock() {
            let e_str = e.to_string();
            if e_str.contains("dpkg --configure -a") {
                // info!(
                //     "{} {} ...",
                //     fl!("dpkg-was-interrupted"),
                //     style("dpkg --configure -a").green().bold()
                // );
                let cmd = Command::new("dpkg")
                    .arg("--configure")
                    .arg("-a")
                    .output()
                    .map_err(|e| OmaAptError::DpkgFailedConfigure(e.to_string()))?;

                if !cmd.status.success() {
                    return Err(OmaAptError::DpkgFailedConfigure(format!(
                        "code: {:?}",
                        cmd.status.code()
                    )));
                }

                apt_lock()?;
            } else {
                return Err(e.into());
            }
        }

        self.cache.get_archives(&mut no_progress).map_err(|e| {
            apt_unlock();
            e
        })?;

        let mut progress = OmaAptInstallProgress::new_box(
            self.config,
            args_config.yes,
            args_config.force_yes,
            args_config.dpkg_force_confnew,
            args_config.dpkg_force_all,
        );

        apt_lock_inner()?;

        self.cache.do_install(&mut progress)?;

        unlock_apt();

        Ok(())
    }

    pub fn resolve(&self, no_fixbroken: bool) -> OmaAptResult<()> {
        if self.cache.resolve(!no_fixbroken).is_err() {
            let unmet = find_unmet_deps(&self.cache)?;
            return Err(OmaAptError::DependencyIssue(unmet));
        }

        let need_fix = self.check_broken()?;

        if no_fixbroken && need_fix {
            warn!("Your system has broken status, Please run `oma fix-broken' to fix it.");
        }
        if self.cache.resolve(!no_fixbroken).is_err() {
            let unmet = find_unmet_deps(&self.cache)?;
            return Err(OmaAptError::DependencyIssue(unmet));
        }

        if !no_fixbroken {
            self.cache.fix_broken();
        }
    
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
                .iter()
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

    pub fn select_pkg(
        &self,
        keywords: Vec<&str>,
        select_dbg: bool,
        filter_candidate: bool,
    ) -> OmaAptResult<Vec<PkgInfo>> {
        select_pkg(keywords, &self.cache, select_dbg, filter_candidate)
    }

    pub fn get_archive_dir(&self) -> PathBuf {
        let archives_dir = self.config.get("Dir::Cache::Archives");

        if let Some(archives_dir) = archives_dir {
            if !Path::new(&archives_dir).is_absolute() {
                PathBuf::from(format!("/var/cache/apt/{archives_dir}"))
            } else {
                PathBuf::from(archives_dir)
            }
        } else {
            PathBuf::from("/var/cache/apt/archives/")
        }
    }

    pub fn operation_vec(&self) -> OmaAptResult<OmaOperation> {
        let mut install = vec![];
        let mut remove = vec![];
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

                let entry = InstallEntryBuilder::default()
                    .name(pkg.name().to_string())
                    .new_version(version.to_string())
                    .new_size(size)
                    .pkg_urls(uri)
                    .checksum(checksum)
                    .arch(pkg.arch().to_string())
                    .download_size(cand.size())
                    .op(InstallOperation::Install)
                    .build()?;

                install.push(entry);

                // If the package is marked install then it will also
                // show up as marked upgrade, downgrade etc.
                // Check this first and continue.
                continue;
            }

            if pkg.marked_upgrade() {
                let install_entry = pkg_delta(&pkg, InstallOperation::Upgrade)?;

                install.push(install_entry);
            }

            if pkg.marked_delete() {
                let name = pkg.name();
                let is_purge = pkg.marked_purge();

                let mut tags = vec![];
                if is_purge {
                    tags.push(RemoveTag::Purge);
                }

                if self.autoremove.contains(&pkg.name().to_string()) {
                    tags.push(RemoveTag::AutoRemove);
                }

                let installed = pkg.installed().unwrap();
                let version = installed.version();
                let size = installed.size();

                let remove_entry =
                    RemoveEntry::new(name.to_string(), version.to_owned(), size, tags);

                remove.push(remove_entry);
            }

            if pkg.marked_reinstall() {
                let version = pkg.installed().unwrap();

                let checksum = version
                    .get_record(RecordField::SHA256)
                    .ok_or_else(|| OmaAptError::PkgNoChecksum(pkg.name().to_string()))?;

                let install_entry = InstallEntryBuilder::default()
                    .name(pkg.name().to_string())
                    .new_version(version.version().to_string())
                    .old_size(version.installed_size())
                    .new_size(version.installed_size())
                    .pkg_urls(version.uris().collect())
                    .checksum(checksum)
                    .arch(pkg.arch().to_string())
                    .download_size(0)
                    .op(InstallOperation::ReInstall)
                    .build()?;

                install.push(install_entry);
            }

            if pkg.marked_downgrade() {
                let install_entry = pkg_delta(&pkg, InstallOperation::Downgrade)?;

                install.push(install_entry);
            }
        }

        let disk_size = self.cache.depcache().disk_size();

        let disk_size = match disk_size {
            DiskSpace::Require(n) => ("+", n),
            DiskSpace::Free(n) => ("-", n),
        };

        Ok(OmaOperation {
            install,
            remove,
            disk_size,
        })
    }

    pub fn filter_pkgs(
        &self,
        query_mode: &[FilterMode],
    ) -> OmaAptResult<impl Iterator<Item = rust_apt::package::Package>> {
        let mut sort = PackageSort::default();

        for i in query_mode {
            sort = match i {
                FilterMode::Installed => sort.installed(),
                FilterMode::Upgradable => sort.upgradable(),
                FilterMode::Automatic => sort.auto_installed(),
                FilterMode::Names => sort.names(),
                _ => sort,
            };
        }

        let pkgs = self.cache.packages(&sort)?;

        Ok(pkgs)
    }
}

fn ask_user_do_as_i_say(pkg: &Package<'_>) -> Result<bool, OmaAptError> {
    let theme = ColorfulTheme::default();
    let delete = Confirm::with_theme(&theme)
        .with_prompt(format!(
            "DELETE THIS PACKAGE? PACKAGE {} IS ESSENTIAL!",
            pkg.name()
        ))
        .default(false)
        .interact()?;
    if !delete {
        info!("Not confirmed.");
        return Ok(false);
    }
    info!(
        "If you are absolutely sure, please type the following: {}",
        style("Do as I say!").bold()
    );

    if Input::<String>::with_theme(&theme)
        .with_prompt("Your turn")
        .interact()?
        != "Do as I say!"
    {
        info!("Prompt answered incorrectly. Not confirmed.");
        return Ok(false);
    }

    Ok(true)
}

fn pkg_delta(new_pkg: &Package, op: InstallOperation) -> OmaAptResult<InstallEntry> {
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

    let install_entry = InstallEntryBuilder::default()
        .name(new_pkg.name().to_string())
        .old_version(old_version.to_string())
        .new_version(new_version.to_owned())
        .old_size(installed.installed_size())
        .new_size(cand.installed_size())
        .pkg_urls(cand.uris().collect::<Vec<_>>())
        .checksum(checksum)
        .arch(new_pkg.arch().to_string())
        .download_size(cand.size())
        .op(op)
        .build()?;

    Ok(install_entry)
}

pub fn select_pkg(
    keywords: Vec<&str>,
    cache: &Cache,
    select_dbg: bool,
    filter_candidate: bool,
) -> OmaAptResult<Vec<PkgInfo>> {
    let db = OmaDatabase::new(cache)?;
    let mut pkgs = vec![];
    for keyword in keywords {
        pkgs.extend(match keyword {
            x if x.ends_with(".deb") => db.query_local_glob(x)?,
            x if x.split_once('/').is_some() => {
                db.query_from_branch(x, filter_candidate, select_dbg)?
            }
            x if x.split_once('=').is_some() => db.query_from_version(x, select_dbg)?,
            x => db.query_from_glob(x, filter_candidate, select_dbg)?,
        });
    }

    Ok(pkgs)
}

fn unlock_apt() {
    apt_unlock_inner();
    apt_unlock();
}

fn mark_install(cache: &Cache, pkginfo: PkgInfo, reinstall: bool) -> OmaAptResult<()> {
    let pkg = pkginfo.raw_pkg;
    let version = pkginfo.version_raw;
    let ver = Version::new(version, cache);
    let pkg = Package::new(cache, pkg);
    ver.set_candidate();

    if pkg.installed().as_ref() == Some(&ver) && !reinstall {
        info!("already-installed");

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
            let v = find_unmet_deps_with_markinstall(cache, &ver);
            return Err(OmaAptError::DependencyIssue(v));
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
