use std::{
    fmt::Display,
    io::Write,
    path::{Path, PathBuf},
    process::Command,
};

use derive_builder::Builder;
use oma_console::{
    console::style,
    debug,
    dialoguer::{theme::ColorfulTheme, Confirm, Input},
    error,
    indicatif::HumanBytes,
    info, warn,
};
use oma_fetch::{
    DownloadEntryBuilder, DownloadEntryBuilderError, DownloadError, DownloadSource,
    DownloadSourceType, OmaFetcher, Summary,
};
use oma_utils::DpkgError;
use once_cell::sync::Lazy;
use rust_apt::{
    cache::{Cache, PackageSort, Upgrade},
    new_cache,
    package::{Package, Version},
    raw::{
        progress::AptInstallProgress,
        util::raw::{apt_lock_inner, apt_unlock, apt_unlock_inner},
    },
    records::RecordField,
    util::{apt_lock, DiskSpace},
};

pub use rust_apt::config::Config as AptConfig;
use time::{macros::offset, OffsetDateTime, UtcOffset};

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

static TIME_OFFSET: Lazy<UtcOffset> =
    Lazy::new(|| UtcOffset::local_offset_at(OffsetDateTime::UNIX_EPOCH).unwrap_or(offset!(UTC)));

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
    dry_run: bool,
    select_pkgs: Vec<String>,
}

#[derive(Debug, thiserror::Error)]
pub enum OmaAptError {
    #[error(transparent)]
    RustApt(#[from] rust_apt::util::Exception),
    #[error(transparent)]
    OmaDatabaseError(#[from] OmaDatabaseError),
    #[error("Failed to mark reinstall pkg: {0}")]
    MarkReinstallError(String, String),
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
    #[error("Insufficient disk space: need: {0}, available: {1}")]
    DiskSpaceInsufficient(HumanBytes, HumanBytes),
    #[error(transparent)]
    DownloadEntryBuilderError(#[from] DownloadEntryBuilderError),
    #[error(transparent)]
    Anyhow(#[from] anyhow::Error),
    #[error("Failed to mark pkg status: {0} is not installed")]
    MarkPkgNotInstalled(String),
    #[error(transparent)]
    DpkgError(#[from] DpkgError),
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

#[derive(Debug)]
pub enum FilterMode {
    Default,
    Installed,
    Upgradable,
    Automatic,
    Names,
}

#[derive(Debug)]
pub struct OmaOperation<'a> {
    pub install: Vec<InstallEntry>,
    pub remove: Vec<RemoveEntry>,
    pub disk_size: (&'a str, u64),
    pub total_download_size: u64,
}

impl<'a> Display for OmaOperation<'a> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut install = vec![];
        let mut upgrade = vec![];
        let mut reinstall = vec![];
        let mut downgrade = vec![];
        let mut remove = vec![];
        let mut purge = vec![];

        for ins in &self.install {
            let name = ins.name();
            let arch = ins.arch();
            let version = ins.new_version();
            match ins.op() {
                InstallOperation::Default | InstallOperation::Download => unreachable!(),
                InstallOperation::Install => {
                    if !ins.automatic() {
                        install.push(format!("{name}:{arch} ({version})"));
                    } else {
                        install.push(format!("{name}:{arch} ({version}, automatic"));
                    }
                }
                InstallOperation::ReInstall => {
                    reinstall.push(format!("{name}:{arch} ({version})"));
                }
                InstallOperation::Upgrade => {
                    upgrade.push(format!(
                        "{name}:{arch} ({}, {version}",
                        ins.old_version().unwrap()
                    ));
                }
                InstallOperation::Downgrade => {
                    downgrade.push(format!("{name}:{arch} ({version}"));
                }
            }
        }

        for rm in &self.remove {
            let tags = rm.details();
            let name = rm.name();
            let version = rm.version();
            let arch = rm.arch();
            if tags.contains(&RemoveTag::Purge) {
                purge.push(format!("{name}:{arch} ({version}"));
            } else {
                remove.push(format!("{name}:{arch} ({version}"))
            }
        }

        if !install.is_empty() {
            writeln!(f, "Install: {}", install.join(","))?;
        }

        if !upgrade.is_empty() {
            writeln!(f, "Upgrade: {}", upgrade.join(","))?;
        }

        if !reinstall.is_empty() {
            writeln!(f, "ReInstall: {}", reinstall.join(","))?;
        }

        if !downgrade.is_empty() {
            writeln!(f, "Downgrade: {}", downgrade.join(","))?;
        }

        if !remove.is_empty() {
            writeln!(f, "Remove: {}", remove.join(","))?;
        }

        if !purge.is_empty() {
            writeln!(f, "Purge: {}", remove.join(","))?;
        }

        Ok(())
    }
}

impl OmaApt {
    pub fn new(local_debs: Vec<String>, args: OmaAptArgs, dry_run: bool) -> OmaAptResult<Self> {
        let config = Self::init_config(args)?;

        Ok(Self {
            cache: new_cache!(&local_debs)?,
            config,
            autoremove: vec![],
            dry_run,
            select_pkgs: vec![],
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

        debug!("APT::Install-Recommends is set to {install_recommend}");

        config.set(
            "APT::Install-Suggests",
            match install_suggests {
                true => "true",
                false => "false",
            },
        );

        debug!("APT::Install-Suggests is set to {install_suggests}");

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

    pub fn install(
        &self,
        pkgs: Vec<PkgInfo>,
        reinstall: bool,
    ) -> OmaAptResult<Vec<(String, String)>> {
        let mut no_marked_install = vec![];
        for pkg in pkgs {
            let marked_install = mark_install(&self.cache, &pkg, reinstall)?;
            if !marked_install {
                no_marked_install.push((
                    pkg.raw_pkg.name().to_string(),
                    pkg.version_raw.version().to_string(),
                ));
            }
        }

        Ok(no_marked_install)
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
        dry_run: bool,
    ) -> OmaAptResult<(Vec<Summary>, Vec<DownloadError>)> {
        let mut download_list = vec![];
        for pkg in pkgs {
            let name = pkg.raw_pkg.name().to_string();
            let ver = Version::new(pkg.version_raw, &self.cache);
            let entry = InstallEntryBuilder::default()
                .name(pkg.raw_pkg.name().to_string())
                .new_version(ver.version().to_string())
                .new_size(pkg.installed_size)
                .pkg_urls(pkg.apt_sources)
                .checksum(
                    pkg.checksum
                        .ok_or_else(|| OmaAptError::PkgNoChecksum(name))?,
                )
                .arch(ver.arch().to_string())
                .download_size(pkg.download_size)
                .op(InstallOperation::Download)
                .build()?;

            download_list.push(entry);
        }

        debug!(
            "Download list: {download_list:?}, download to: {}",
            download_dir.unwrap_or(Path::new(".")).display()
        );

        if dry_run {
            return Ok((vec![], vec![]));
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
    ) -> OmaAptResult<Vec<String>> {
        let mut no_marked_remove = vec![];
        for pkg in pkgs {
            let is_marked_delete = mark_delete(&self.cache, &pkg, protect, cli_output, purge)?;
            if !is_marked_delete {
                no_marked_remove.push(pkg.raw_pkg.name().to_string());
            }
        }

        // 寻找系统有哪些不必要的软件包
        if !no_autoremove {
            self.autoremove(purge)?;
        }

        Ok(no_marked_remove)
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

    pub fn commit(self, network_thread: Option<usize>, args_config: &AptArgs) -> OmaAptResult<()> {
        let v = self.summary()?;
        let v_str = v.to_string();
        let start_time = OffsetDateTime::now_utc()
            .to_offset(*TIME_OFFSET)
            .to_string();

        if self.dry_run {
            debug!("op: {v:?}");
            return Ok(());
        }

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
                debug!(
                    "dpkg-was-interrupted, running {} ...",
                    style("dpkg --configure -a").green().bold()
                );
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

        apt_unlock_inner();

        self.cache.do_install(&mut progress).map_err(|e| {
            apt_lock_inner().ok();
            apt_unlock();
            e
        })?;

        apt_unlock();

        let end_time = OffsetDateTime::now_utc()
            .to_offset(*TIME_OFFSET)
            .to_string();

        std::fs::create_dir_all("/var/log/oma/")?;

        let mut log = std::fs::OpenOptions::new()
            .append(true)
            .create(true)
            .write(true)
            .open("/var/log/oma/history")?;

        writeln!(log, "Start-Date: {start_time}").ok();
        writeln!(log, "{v_str}").ok();
        writeln!(log, "End-Date: {end_time}").ok();

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

            if self.cache.resolve(!no_fixbroken).is_err() {
                let unmet = find_unmet_deps(&self.cache)?;
                return Err(OmaAptError::DependencyIssue(unmet));
            }
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

            let download_entry = DownloadEntryBuilder::default()
                .source(sources)
                .filename(apt_style_filename(
                    filename,
                    entry.new_version().to_string(),
                )?)
                .dir(download_dir.to_path_buf())
                .hash(entry.checksum().to_string())
                .allow_resume(true)
                .msg(format!(
                    "{} {} ({})",
                    entry.name(),
                    entry.new_version(),
                    entry.arch()
                ))
                .build()?;

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
        &mut self,
        keywords: Vec<&str>,
        select_dbg: bool,
        filter_candidate: bool,
    ) -> OmaAptResult<Vec<PkgInfo>> {
        let res = select_pkg(keywords, &self.cache, select_dbg, filter_candidate)?;
        self.select_pkgs = res
            .iter()
            .map(|x| x.raw_pkg.name().to_string())
            .collect::<Vec<_>>();

        Ok(res)
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

    pub fn mark_version_status<'a>(
        &'a self,
        pkgs: &'a [String],
        hold: bool,
        dry_run: bool,
    ) -> OmaAptResult<Vec<(&str, bool)>> {
        for pkg in pkgs {
            if !self
                .cache
                .get(pkg)
                .map(|x| x.is_installed())
                .unwrap_or(false)
            {
                return Err(OmaAptError::MarkPkgNotInstalled(pkg.to_string()));
            }
        }

        let res = oma_utils::mark_version_status(pkgs, hold, dry_run)?;

        Ok(res)
    }

    pub fn mark_install_status(
        self,
        pkgs: Vec<PkgInfo>,
        auto: bool,
        dry_run: bool,
    ) -> OmaAptResult<Vec<(String, bool)>> {
        let mut res = vec![];
        for pkg in pkgs {
            let pkg = Package::new(&self.cache, pkg.raw_pkg);

            if !pkg.is_installed() {
                return Err(OmaAptError::MarkPkgNotInstalled(pkg.name().to_string()));
            }

            if pkg.is_auto_installed() {
                if auto {
                    res.push((pkg.name().to_string(), false));
                    debug!("pkg {} set to auto = {auto} is set = false", pkg.name());
                } else {
                    pkg.mark_auto(false);
                    res.push((pkg.name().to_string(), true));
                    debug!("pkg {} set to auto = {auto} is set = true", pkg.name());
                }
            } else if auto {
                pkg.mark_auto(true);
                res.push((pkg.name().to_string(), true));
                debug!("pkg {} set to auto = {auto} is set = true", pkg.name());
            } else {
                res.push((pkg.name().to_string(), false));
                debug!("pkg {} set to auto = {auto} is set = false", pkg.name());
            }
        }

        if dry_run {
            return Ok(res);
        }

        self.cache
            .commit(
                &mut NoProgress::new_box(),
                &mut AptInstallProgress::new_box(),
            )
            .map_err(|e| anyhow::anyhow!("{e}"))?;

        Ok(res)
    }

    pub fn summary(&self) -> OmaAptResult<OmaOperation> {
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
                    .arch(cand.arch().to_string())
                    .download_size(cand.size())
                    .op(InstallOperation::Install)
                    .automatic(!self.select_pkgs.contains(&pkg.name().to_string()))
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

                let remove_entry = RemoveEntry::new(
                    name.to_string(),
                    version.to_owned(),
                    size,
                    tags,
                    installed.arch().to_owned(),
                );

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
                    .arch(version.arch().to_string())
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

        let total_download_size: u64 = install
            .iter()
            .filter(|x| {
                x.op() == &InstallOperation::Install || x.op() == &InstallOperation::Upgrade
            })
            .map(|x| x.download_size())
            .sum();

        Ok(OmaOperation {
            install,
            remove,
            disk_size,
            total_download_size,
        })
    }

    pub fn check_disk_size(&self) -> OmaAptResult<()> {
        let op = self.summary()?;

        let (symbol, n) = op.disk_size;
        let n = n as i64;
        let download_size = op.total_download_size as i64;

        let need_space = match symbol {
            "+" => download_size + n,
            "-" => download_size - n,
            _ => unreachable!(),
        };

        let available_disk_size = fs4::available_space("/")? as i64;

        if available_disk_size < need_space {
            return Err(OmaAptError::DiskSpaceInsufficient(
                HumanBytes(need_space as u64),
                HumanBytes(available_disk_size as u64),
            ));
        }

        debug!("available_disk_size is: {available_disk_size}, need: {need_space}");

        Ok(())
    }

    pub fn filter_pkgs(
        &self,
        query_mode: &[FilterMode],
    ) -> OmaAptResult<impl Iterator<Item = rust_apt::package::Package>> {
        let mut sort = PackageSort::default();

        debug!("Filter Mode: {query_mode:?}");

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

fn mark_delete(
    cache: &Cache,
    pkg: &PkgInfo,
    protect: bool,
    cli_output: bool,
    purge: bool,
) -> OmaAptResult<bool> {
    let pkg = Package::new(cache, pkg.raw_pkg.unique());
    if !pkg.is_installed() {
        debug!(
            "Package {} is not installed. No need to remove.",
            pkg.name()
        );
        return Ok(false);
    }

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

    Ok(true)
}

fn ask_user_do_as_i_say(pkg: &Package<'_>) -> OmaAptResult<bool> {
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
        .arch(cand.arch().to_string())
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

fn mark_install(cache: &Cache, pkginfo: &PkgInfo, reinstall: bool) -> OmaAptResult<bool> {
    let pkg = pkginfo.raw_pkg.unique();
    let version = pkginfo.version_raw.unique();
    let ver = Version::new(version, cache);
    let pkg = Package::new(cache, pkg);
    ver.set_candidate();

    if pkg.installed().as_ref() == Some(&ver) && !reinstall {
        return Ok(false);
    } else if pkg.installed().as_ref() == Some(&ver) && reinstall {
        if ver.uris().next().is_none() {
            return Err(OmaAptError::MarkReinstallError(
                pkg.name().to_string(),
                ver.version().to_string(),
            ));
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

    debug!("{} will marked install", pkg.name());
    pkg.protect();

    Ok(true)
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
