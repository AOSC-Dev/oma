use std::{
    borrow::Cow,
    fmt::Display,
    io::Write,
    path::{Path, PathBuf},
    process::Command,
};

use chrono::Local;
use derive_builder::Builder;
use oma_apt::raw::util::raw::apt_lock;
use oma_apt::{
    cache::{Cache, PackageSort, Upgrade},
    new_cache,
    package::{Package, Version},
    raw::{
        progress::AptInstallProgress,
        util::raw::{apt_lock_inner, apt_unlock, apt_unlock_inner},
    },
    records::RecordField,
    util::DiskSpace,
};
use oma_console::{
    console::{self, style},
    debug, error,
    indicatif::HumanBytes,
    warn,
};
use oma_fetch::{
    DownloadEntryBuilder, DownloadEntryBuilderError, DownloadError, DownloadEvent, DownloadSource,
    DownloadSourceType, OmaFetcher, Summary,
};
use oma_utils::dpkg::{is_hold, DpkgError};

pub use oma_apt::config::Config as AptConfig;

use serde::{Deserialize, Serialize};

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

const TIME_FORMAT: &str = "%H:%M:%S on %Y-%m-%d";

#[derive(Builder, Default, Clone)]
#[builder(default)]
pub struct OmaAptArgs {
    install_recommends: bool,
    install_suggests: bool,
    no_install_recommends: bool,
    no_install_suggests: bool,
    sysroot: String,
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
    RustApt(#[from] oma_apt::util::Exception),
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
    #[error("Package: {0}: {1} has no mirror available.")]
    PkgUnavailable(String, String),
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
    #[error("Has {0} package failed to download.")]
    FailedToDownload(usize, Vec<DownloadError>),
}

#[derive(Default, Builder)]
#[builder(default)]
pub struct AptArgs {
    yes: bool,
    force_yes: bool,
    dpkg_force_confnew: bool,
    dpkg_force_all: bool,
    no_progress: bool,
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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OmaOperation {
    pub install: Vec<InstallEntry>,
    pub remove: Vec<RemoveEntry>,
    pub disk_size: (String, u64),
    pub total_download_size: u64,
}

impl Display for OmaOperation {
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
                        install.push(format!("{name}:{arch} ({version}, automatic)"));
                    }
                }
                InstallOperation::ReInstall => {
                    reinstall.push(format!("{name}:{arch} ({version})"));
                }
                InstallOperation::Upgrade => {
                    // Upgrade 的情况下 old_version 的值肯定存在，因此直接 unwreap
                    upgrade.push(format!(
                        "{name}:{arch} ({}, {version})",
                        ins.old_version().unwrap()
                    ));
                }
                InstallOperation::Downgrade => {
                    downgrade.push(format!("{name}:{arch} ({version})"));
                }
            }
        }

        for rm in &self.remove {
            let tags = rm.details();
            let name = rm.name();
            let version = rm.version();
            let arch = rm.arch();

            let mut s = format!("{name}:{arch}");
            if let Some(ver) = version {
                s.push_str(&format!(" ({ver})"));
            }

            if tags.contains(&RemoveTag::Purge) {
                purge.push(s);
            } else {
                remove.push(s);
            }
        }

        if !install.is_empty() {
            writeln!(f, "Install: {}", install.join(", "))?;
        }

        if !upgrade.is_empty() {
            writeln!(f, "Upgrade: {}", upgrade.join(", "))?;
        }

        if !reinstall.is_empty() {
            writeln!(f, "ReInstall: {}", reinstall.join(", "))?;
        }

        if !downgrade.is_empty() {
            writeln!(f, "Downgrade: {}", downgrade.join(", "))?;
        }

        if !remove.is_empty() {
            writeln!(f, "Remove: {}", remove.join(", "))?;
        }

        if !purge.is_empty() {
            writeln!(f, "Purge: {}", purge.join(", "))?;
        }

        let (symbol, n) = &self.disk_size;
        writeln!(f, "Size-delta: {symbol}{}", HumanBytes(n.to_owned()))?;

        Ok(())
    }
}

impl OmaApt {
    /// Create a new apt manager
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

    /// Init apt config (before create new apt manager)
    fn init_config(args: OmaAptArgs) -> OmaAptResult<AptConfig> {
        let config = AptConfig::new();
        config.set("Dir", &args.sysroot);
        config.set(
            "Dir::State::status",
            &Path::new(&args.sysroot)
                .join("var/lib/dpkg/status")
                .display()
                .to_string(),
        );

        let install_recommend = if args.install_recommends {
            true
        } else if args.no_install_recommends {
            false
        } else {
            config
                .get("APT::Install-Recommends")
                .map(|x| x == "true")
                .unwrap_or(true)
        };

        let install_suggests = if args.install_suggests {
            true
        } else if args.no_install_suggests {
            false
        } else {
            config
                .get("APT::Install-Suggests")
                .map(|x| x == "true")
                .unwrap_or(false)
        };

        config.set("APT::Install-Recommends", &install_recommend.to_string());
        debug!("APT::Install-Recommends is set to {install_recommend}");

        config.set("APT::Install-Suggests", &install_suggests.to_string());
        debug!("APT::Install-Suggests is set to {install_suggests}");

        Ok(config)
    }

    /// Get upgradable and removable packages
    pub fn available_action(&self) -> OmaAptResult<(usize, usize)> {
        let sort = PackageSort::default().upgradable();
        let upgradable = self
            .cache
            .packages(&sort)?
            .filter(|x| !is_hold(x.name()).unwrap_or(false))
            .count();

        let sort = PackageSort::default().auto_removable();
        let removable = self.cache.packages(&sort)?.count();

        Ok((upgradable, removable))
    }

    /// Set apt manager status as upgrade
    pub fn upgrade(&self) -> OmaAptResult<()> {
        self.cache.upgrade(&Upgrade::FullUpgrade)?;

        Ok(())
    }

    /// Set apt manager status as install
    pub fn install(
        &mut self,
        pkgs: &[PkgInfo],
        reinstall: bool,
    ) -> OmaAptResult<Vec<(String, String)>> {
        let mut no_marked_install = vec![];
        for pkg in pkgs {
            let marked_install = mark_install(&self.cache, pkg, reinstall)?;
            debug!(
                "Pkg {} {} marked install: {marked_install}",
                pkg.raw_pkg.name(),
                pkg.version_raw.version()
            );

            if !marked_install {
                no_marked_install.push((
                    pkg.raw_pkg.name().to_string(),
                    pkg.version_raw.version().to_string(),
                ));
            } else if !self.select_pkgs.contains(&pkg.raw_pkg.name().to_string()) {
                self.select_pkgs.push(pkg.raw_pkg.name().to_string());
            }
        }

        Ok(no_marked_install)
    }

    /// Find system is broken
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

    /// Download packages
    pub fn download<F>(
        &self,
        pkgs: Vec<PkgInfo>,
        network_thread: Option<usize>,
        download_dir: Option<&Path>,
        dry_run: bool,
        callback: F,
    ) -> OmaAptResult<(Vec<Summary>, Vec<DownloadError>)>
    where
        F: Fn(usize, DownloadEvent, Option<u64>) + Clone + Send + Sync,
    {
        let mut download_list = vec![];
        for pkg in pkgs {
            let name = pkg.raw_pkg.name().to_string();
            let ver = Version::new(pkg.version_raw, &self.cache);
            let install_size = ver.installed_size();
            if !ver.is_downloadable() {
                return Err(OmaAptError::PkgUnavailable(name, ver.version().to_string()));
            }
            let mut entry = InstallEntryBuilder::default();
            entry.name(pkg.raw_pkg.name().to_string());
            entry.new_version(ver.version().to_string());
            entry.new_size(install_size);
            entry.pkg_urls(ver.uris().collect::<Vec<_>>());
            entry.arch(ver.arch().to_string());
            entry.download_size(ver.size());
            entry.op(InstallOperation::Download);

            if ver.uris().all(|x| !x.starts_with("file")) {
                entry.checksum(
                    ver.get_record(RecordField::SHA256)
                        .ok_or_else(|| OmaAptError::PkgNoChecksum(name))?,
                );
            }
            let entry = entry.build()?;

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
                callback,
            )
            .await
        })?;

        Ok(res)
    }

    /// Set apt manager status as remove
    pub fn remove<F>(
        &mut self,
        pkgs: &[PkgInfo],
        purge: bool,
        no_autoremove: bool,
        callback: F,
    ) -> OmaAptResult<Vec<String>>
    where
        F: Fn(&str) -> bool + Copy,
    {
        let mut no_marked_remove = vec![];
        for pkg in pkgs {
            let is_marked_delete = mark_delete(&self.cache, pkg, purge, callback)?;
            if !is_marked_delete {
                no_marked_remove.push(pkg.raw_pkg.name().to_string());
            } else if !self.select_pkgs.contains(&pkg.raw_pkg.name().to_string()) {
                self.select_pkgs.push(pkg.raw_pkg.name().to_string());
            }
        }

        // 寻找系统有哪些不必要的软件包
        if !no_autoremove {
            // FIXME: 需要先计算依赖才知道后面多少软件包是不必要的
            self.resolve(false)?;
            self.autoremove(purge)?;
        }

        Ok(no_marked_remove)
    }

    /// find autoremove and remove it
    fn autoremove(&mut self, purge: bool) -> OmaAptResult<()> {
        let sort = PackageSort::default().installed();
        let pkgs = self.cache.packages(&sort)?;

        for pkg in pkgs {
            if pkg.is_auto_removable() && !pkg.marked_delete() {
                pkg.mark_delete(purge);
                pkg.protect();

                self.autoremove.push(pkg.name().to_string());
            }
        }

        Ok(())
    }

    /// Commit changes
    pub fn commit<F>(
        self,
        network_thread: Option<usize>,
        args_config: &AptArgs,
        callback: F,
    ) -> OmaAptResult<()>
    where
        F: Fn(usize, DownloadEvent, Option<u64>) + Clone + Send + Sync,
    {
        let v = self.summary()?;
        let v_str = v.to_string();

        let start_time = Local::now();

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
            Self::download_pkgs(download_pkg_list, network_thread, &path, callback).await
        })?;

        if !failed.is_empty() {
            return Err(OmaAptError::FailedToDownload(failed.len(), failed));
        }

        debug!("Success: {success:?}");

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
            args_config.no_progress,
        );

        apt_unlock_inner();

        self.cache.do_install(&mut progress).map_err(|e| {
            apt_lock_inner().ok();
            apt_unlock();
            e
        })?;

        apt_unlock();

        let end_time = Local::now().format(TIME_FORMAT).to_string();

        std::fs::create_dir_all("/var/log/oma/")?;

        let mut log = std::fs::OpenOptions::new()
            .append(true)
            .create(true)
            .write(true)
            .open("/var/log/oma/history")?;

        writeln!(log, "Start-Date: {start_time}").ok();

        let args = std::env::args().collect::<Vec<_>>().join(" ");

        if !args.is_empty() {
            writeln!(log, "Commandline: {args}").ok();
        }

        if let Some((user, uid)) = std::env::var("SUDO_USER")
            .ok()
            .zip(std::env::var("SUDO_UID").ok())
        {
            writeln!(log, "Requested-By: {user} ({uid})").ok();
        }

        write!(log, "{v_str}").ok();
        writeln!(log, "End-Date: {end_time}\n").ok();

        Ok(())
    }

    /// Resolve apt dependencies
    pub fn resolve(&self, no_fixbroken: bool) -> OmaAptResult<()> {
        let need_fix = self.check_broken()?;

        if no_fixbroken && need_fix {
            warn!("Your system has broken status, Please run `oma fix-broken' to fix it.");
        }

        if !no_fixbroken {
            self.cache.fix_broken();
        }

        if self.cache.resolve(!no_fixbroken).is_err() {
            let unmet = find_unmet_deps(&self.cache)?;
            return Err(OmaAptError::DependencyIssue(unmet));
        }

        Ok(())
    }

    /// Download packages (inner)
    async fn download_pkgs<F>(
        download_pkg_list: Vec<InstallEntry>,
        network_thread: Option<usize>,
        download_dir: &Path,
        callback: F,
    ) -> OmaAptResult<(Vec<Summary>, Vec<DownloadError>)>
    where
        F: Fn(usize, DownloadEvent, Option<u64>) + Clone + Send + Sync,
    {
        if download_pkg_list.is_empty() {
            callback(0, DownloadEvent::AllDone, None);
            return Ok((vec![], vec![]));
        }

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

            let new_version = if console::measure_text_width(entry.new_version()) > 25 {
                console::truncate_str(entry.new_version(), 25, "...")
            } else {
                Cow::Borrowed(entry.new_version())
            };

            let msg = format!("{} {new_version} ({})", entry.name(), entry.arch());

            let mut download_entry = DownloadEntryBuilder::default();
            download_entry.source(sources);
            download_entry
                .filename(apt_style_filename(filename, entry.new_version().to_string())?.into());
            download_entry.dir(download_dir.to_path_buf());
            download_entry.allow_resume(true);
            download_entry.msg(msg);

            if let Some(checksum) = entry.checksum() {
                download_entry.hash(checksum);
            }

            let download_entry = download_entry.build()?;

            total_size += entry.download_size();

            download_list.push(download_entry);
        }

        let downloader = OmaFetcher::new(None, download_list, network_thread)?;

        let res = downloader
            .start_download(|count, event| callback(count, event, Some(total_size)))
            .await;

        let (mut success, mut failed) = (vec![], vec![]);

        for i in res {
            match i {
                Ok(s) => success.push(s),
                Err(e) => failed.push(e),
            }
        }

        Ok((success, failed))
    }

    /// Select packages from give some strings
    pub fn select_pkg(
        &mut self,
        keywords: &[&str],
        select_dbg: bool,
        filter_candidate: bool,
        available_candidate: bool,
    ) -> OmaAptResult<(Vec<PkgInfo>, Vec<String>)> {
        select_pkg(
            keywords,
            &self.cache,
            select_dbg,
            filter_candidate,
            available_candidate,
        )
    }

    /// Get apt archive dir
    pub fn get_archive_dir(&self) -> PathBuf {
        let archives_dir = self
            .config
            .get("Dir::Cache::Archives")
            .unwrap_or("archives/".to_string());
        let cache = self
            .config
            .get("Dir::Cache")
            .unwrap_or("var/cache/apt".to_string());

        let dir = self.config.get("Dir").unwrap_or("/".to_string());

        let archive_dir_p = PathBuf::from(archives_dir);
        if archive_dir_p.is_absolute() {
            return archive_dir_p;
        }

        let cache_dir_p = PathBuf::from(cache);
        if cache_dir_p.is_absolute() {
            return cache_dir_p.join(archive_dir_p);
        }

        let dir_p = PathBuf::from(dir);

        dir_p.join(cache_dir_p).join(archive_dir_p)
    }

    /// Mark version status (hold/unhold)
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

        let res = oma_utils::dpkg::mark_version_status(pkgs, hold, dry_run)?;

        Ok(res)
    }

    /// Mark version status (auto/manual)
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

    /// Show changes summary
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
                let not_local_source = uri.iter().all(|x| !x.starts_with("file:"));
                let version = cand.version();
                let checksum = cand.get_record(RecordField::SHA256);

                let size = cand.installed_size();

                let mut entry = InstallEntryBuilder::default();
                entry.name(pkg.name().to_string());
                entry.new_version(version.to_string());
                entry.new_size(size);
                entry.pkg_urls(uri);
                entry.arch(cand.arch().to_string());
                entry.download_size(cand.size());
                entry.op(InstallOperation::Install);
                entry.automatic(!self.select_pkgs.contains(&pkg.name().to_string()));

                if not_local_source {
                    entry.checksum(
                        checksum
                            .ok_or_else(|| OmaAptError::PkgNoChecksum(pkg.name().to_string()))?,
                    );
                }

                let entry = entry.build()?;

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

                let installed = pkg.installed();
                let version = installed.as_ref().map(|x| x.version().to_string());
                let size = installed.as_ref().map(|x| x.installed_size());

                let remove_entry = RemoveEntry::new(
                    name.to_string(),
                    version,
                    size.unwrap_or(0),
                    tags,
                    installed
                        .map(|x| x.arch().to_string())
                        .unwrap_or("unknown".to_string()),
                );

                remove.push(remove_entry);
            }

            if pkg.marked_reinstall() {
                // 如果一个包被标记为重装，则肯定已经安装
                // 所以请求已安装版本应该直接 unwrap
                let version = pkg.installed().unwrap();
                let checksum = version.get_record(RecordField::SHA256);
                let uri = version.uris().collect::<Vec<_>>();
                let not_local_source = uri.iter().all(|x| !x.starts_with("file:"));

                let mut entry = InstallEntryBuilder::default();
                entry.name(pkg.name().to_string());
                entry.new_version(version.version().to_string());
                entry.old_size(version.installed_size());
                entry.new_size(version.installed_size());
                entry.pkg_urls(uri);
                entry.arch(version.arch().to_string());
                entry.download_size(version.size());
                entry.op(InstallOperation::ReInstall);
                entry.automatic(!self.select_pkgs.contains(&pkg.name().to_string()));

                if not_local_source {
                    entry.checksum(
                        checksum
                            .ok_or_else(|| OmaAptError::PkgNoChecksum(pkg.name().to_string()))?,
                    );
                }

                let entry = entry.build()?;

                install.push(entry);
            }

            if pkg.marked_downgrade() {
                let install_entry = pkg_delta(&pkg, InstallOperation::Downgrade)?;

                install.push(install_entry);
            }
        }

        let disk_size = self.cache.depcache().disk_size();

        let disk_size = match disk_size {
            DiskSpace::Require(n) => ("+".to_string(), n),
            DiskSpace::Free(n) => ("-".to_string(), n),
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

    /// Check available disk space
    pub fn check_disk_size(&self) -> OmaAptResult<()> {
        let op = self.summary()?;

        let (symbol, n) = op.disk_size;
        let n = n as i64;
        let download_size = op.total_download_size as i64;

        let need_space = match symbol.as_str() {
            "+" => download_size + n,
            "-" => download_size - n,
            _ => unreachable!(),
        };

        let available_disk_size =
            fs4::available_space(self.config.get("Dir").unwrap_or("/".to_string()))? as i64;

        if available_disk_size < need_space {
            return Err(OmaAptError::DiskSpaceInsufficient(
                HumanBytes(need_space as u64),
                HumanBytes(available_disk_size as u64),
            ));
        }

        debug!("available_disk_size is: {available_disk_size}, need: {need_space}");

        Ok(())
    }

    /// Filters pkgs
    pub fn filter_pkgs(
        &self,
        query_mode: &[FilterMode],
    ) -> OmaAptResult<impl Iterator<Item = oma_apt::package::Package>> {
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

/// Mark package as delete.
fn mark_delete<F>(
    cache: &Cache,
    pkg: &PkgInfo,
    purge: bool,
    how_handle_essential: F,
) -> OmaAptResult<bool>
where
    F: Fn(&str) -> bool + Copy,
{
    let pkg = Package::new(cache, pkg.raw_pkg.unique());
    let removed_but_has_config = pkg.current_state() == 5;
    if !pkg.is_installed() && !removed_but_has_config {
        debug!(
            "Package {} is not installed. No need to remove.",
            pkg.name()
        );
        return Ok(false);
    }

    if pkg.is_essential() {
        let remove_essential = how_handle_essential(pkg.name());
        if !remove_essential {
            return Err(OmaAptError::PkgIsEssential(pkg.name().to_string()));
        }
    }

    pkg.mark_delete(purge || removed_but_has_config);
    pkg.protect();

    Ok(true)
}

fn pkg_delta(new_pkg: &Package, op: InstallOperation) -> OmaAptResult<InstallEntry> {
    let cand = new_pkg
        .candidate()
        .take()
        .ok_or_else(|| OmaAptError::PkgNoCandidate(new_pkg.name().to_string()))?;

    let uri = cand.uris().collect::<Vec<_>>();
    let not_local_source = uri.iter().all(|x| !x.starts_with("file:"));

    let new_version = cand.version();
    // 如果一个包有版本修改，则肯定之前已经安装
    // 所以请求已安装版本应该直接 unwrap
    let installed = new_pkg.installed().unwrap();
    let old_version = installed.version();

    let checksum = cand.get_record(RecordField::SHA256);

    let mut install_entry = InstallEntryBuilder::default();
    install_entry.name(new_pkg.name().to_string());
    install_entry.old_version(old_version.to_string());
    install_entry.new_version(new_version.to_owned());
    install_entry.old_size(installed.installed_size());
    install_entry.new_size(cand.installed_size());
    install_entry.pkg_urls(cand.uris().collect::<Vec<_>>());
    install_entry.arch(cand.arch().to_string());
    install_entry.download_size(cand.size());
    install_entry.op(op);

    if not_local_source {
        install_entry.checksum(
            checksum.ok_or_else(|| OmaAptError::PkgNoChecksum(new_pkg.name().to_string()))?,
        );
    }

    let install_entry = install_entry.build()?;

    Ok(install_entry)
}

/// Select pkg from give strings (inber)
fn select_pkg(
    keywords: &[&str],
    cache: &Cache,
    select_dbg: bool,
    filter_candidate: bool,
    available_candidate: bool,
) -> OmaAptResult<(Vec<PkgInfo>, Vec<String>)> {
    let db = OmaDatabase::new(cache)?;
    let mut pkgs = vec![];
    let mut no_result = vec![];
    for keyword in keywords {
        let res = match keyword {
            x if x.ends_with(".deb") => db.query_local_glob(x)?,
            x if x.split_once('/').is_some() => {
                db.query_from_branch(x, filter_candidate, select_dbg)?
            }
            x if x.split_once('=').is_some() => db.query_from_version(x, select_dbg)?,
            x => db.query_from_glob(x, filter_candidate, select_dbg, available_candidate)?,
        };

        for i in &res {
            debug!("{} {}", i.raw_pkg.name(), i.version_raw.version());
        }

        if res.is_empty() {
            no_result.push(keyword.to_string());
            continue;
        }

        pkgs.extend(res);
    }

    Ok((pkgs, no_result))
}

/// Mark package as install.
fn mark_install(cache: &Cache, pkginfo: &PkgInfo, reinstall: bool) -> OmaAptResult<bool> {
    let pkg = pkginfo.raw_pkg.unique();
    let version = pkginfo.version_raw.unique();
    let ver = Version::new(version, cache);
    let pkg = Package::new(cache, pkg);
    ver.set_candidate();

    if let Some(installed) = pkg.installed() {
        if installed.version() == ver.version()
            && !reinstall
            && installed.package_files().any(|inst| {
                ver.package_files()
                    .any(|ver| ver.archive().ok() == inst.archive().ok())
            })
        {
            return Ok(false);
        } else if installed.version() == ver.version() && reinstall {
            if !ver.is_downloadable() {
                return Err(OmaAptError::MarkReinstallError(
                    pkg.name().to_string(),
                    ver.version().to_string(),
                ));
            }

            let is_marked = pkg.mark_reinstall(true);
            pkg.protect();
            return Ok(is_marked);
        }
    }

    pkg.mark_install(true, true);
    debug!("marked_install: {}", pkg.marked_install());
    debug!("marked_downgrade: {}", pkg.marked_downgrade());
    debug!("marked_upgrade: {}", pkg.marked_upgrade());
    if !pkg.marked_install() && !pkg.marked_downgrade() && !pkg.marked_upgrade() {
        // apt 会先就地检查这个包的表面依赖是否满足要求，如果不满足则直接返回错误，而不是先交给 resolver
        let v = find_unmet_deps_with_markinstall(cache, &ver);
        return Err(OmaAptError::DependencyIssue(v));
    }

    debug!("{} will marked install", pkg.name());
    pkg.protect();

    Ok(true)
}

/// trans filename to apt style file name
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
