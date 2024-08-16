use std::{
    borrow::Cow,
    io::{self, ErrorKind, Write},
    path::{Path, PathBuf},
    process::Command,
};

use chrono::Local;
use derive_builder::Builder;

use oma_apt::{
    cache::{Cache, PackageSort, Upgrade},
    error::{AptError, AptErrors},
    new_cache,
    progress::{AcquireProgress, InstallProgress},
    raw::IntoRawIter,
    records::RecordField,
    util::{apt_lock, apt_lock_inner, apt_unlock, apt_unlock_inner, DiskSpace},
    DepFlags, Package, PkgCurrentState, Version,
};
use oma_console::console::{self, style};
use oma_fetch::{
    checksum::{Checksum, ChecksumError},
    reqwest::Client,
    DownloadEntryBuilder, DownloadEntryBuilderError, DownloadError, DownloadEvent, DownloadSource,
    DownloadSourceType, OmaFetcher, Summary,
};
use oma_utils::{
    dpkg::{dpkg_arch, is_hold, DpkgError},
    human_bytes::HumanBytes,
};

pub use oma_apt::config::Config as AptConfig;
use tokio::runtime::Runtime;
use tracing::{debug, info, warn};

pub use oma_pm_operation_type::*;
use zbus::{Connection, ConnectionBuilder};

use crate::{
    dbus::{change_status, OmaBus, Status},
    pkginfo::{PkgInfo, PtrIsNone},
    progress::{InstallProgressArgs, OmaAptInstallProgress},
    query::{OmaDatabase, OmaDatabaseError},
};

const TIME_FORMAT: &str = "%H:%M:%S on %Y-%m-%d";

#[derive(Builder, Default, Clone)]
#[builder(default)]
pub struct OmaAptArgs {
    install_recommends: bool,
    install_suggests: bool,
    no_install_recommends: bool,
    no_install_suggests: bool,
    #[builder(default = "self.default_sysroot()")]
    sysroot: String,
}

impl OmaAptArgsBuilder {
    fn default_sysroot(&self) -> String {
        String::from("/")
    }
}

pub struct OmaApt {
    pub cache: Cache,
    pub config: AptConfig,
    autoremove: Vec<String>,
    dry_run: bool,
    select_pkgs: Vec<String>,
    tokio: Runtime,
    connection: Option<Connection>,
    unmet: Vec<String>,
}

#[derive(Debug, thiserror::Error)]
pub enum OmaAptError {
    #[error(transparent)]
    AptErrors(#[from] AptErrors),
    #[error(transparent)]
    AptError(#[from] AptError),
    #[error(transparent)]
    AptCxxException(#[from] cxx::Exception),
    #[error(transparent)]
    OmaDatabaseError(#[from] OmaDatabaseError),
    #[error("Failed to mark reinstall pkg: {0}")]
    MarkReinstallError(String, String),
    #[error("Find Dependency problem")]
    DependencyIssue(Vec<String>),
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
    #[error("Failed to create async runtime: {0}")]
    FailedCreateAsyncRuntime(std::io::Error),
    #[error("Failed to create dir or file: {0}: {1}")]
    FailedOperateDirOrFile(String, std::io::Error),
    #[error("Failed to get available space: {0}")]
    FailedGetAvailableSpace(std::io::Error),
    #[error(transparent)]
    InstallEntryBuilderError(#[from] InstallEntryBuilderError),
    #[error("Failed to run dpkg --configure -a: {0}")]
    DpkgFailedConfigure(std::io::Error),
    #[error("Insufficient disk space: need: {0}, available: {1}")]
    DiskSpaceInsufficient(HumanBytes, HumanBytes),
    #[error(transparent)]
    DownloadEntryBuilderError(#[from] DownloadEntryBuilderError),
    #[error("Can not commit: {0}")]
    CommitErr(String),
    #[error("Failed to mark pkg status: {0} is not installed")]
    MarkPkgNotInstalled(String),
    #[error(transparent)]
    DpkgError(#[from] DpkgError),
    #[error("Has {0} package failed to download.")]
    FailedToDownload(usize, Vec<DownloadError>),
    #[error("Failed to get path parent: {0:?}")]
    FailedGetParentPath(PathBuf),
    #[error("Failed to get canonicalize path: {0}")]
    FailedGetCanonicalize(String, std::io::Error),
    #[error(transparent)]
    PtrIsNone(#[from] PtrIsNone),
    #[error(transparent)]
    ChecksumError(#[from] ChecksumError),
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
    Manual,
    Names,
}

impl OmaApt {
    /// Create a new apt manager
    pub fn new(local_debs: Vec<String>, args: OmaAptArgs, dry_run: bool) -> OmaAptResult<Self> {
        let config = Self::init_config(args)?;

        let bus = OmaBus {
            status: Status::Configing,
        };

        let tokio = tokio::runtime::Builder::new_multi_thread()
            .enable_time()
            .enable_io()
            .build()
            .map_err(OmaAptError::FailedCreateAsyncRuntime)?;

        let conn = tokio.block_on(async { Self::create_session(bus).await.ok() });

        Ok(Self {
            cache: new_cache!(&local_debs)?,
            config,
            autoremove: vec![],
            dry_run,
            select_pkgs: vec![],
            tokio,
            connection: conn,
            unmet: vec![],
        })
    }

    async fn create_session(bus: OmaBus) -> Result<Connection, zbus::Error> {
        let conn = ConnectionBuilder::system()?
            .name("io.aosc.Oma")?
            .serve_at("/io/aosc/Oma", bus)?
            .build()
            .await?;

        debug!("zbus session created");

        Ok(conn)
    }

    /// Init apt config (before create new apt manager)
    fn init_config(args: OmaAptArgs) -> OmaAptResult<AptConfig> {
        let config = AptConfig::new();

        let sysroot = Path::new(&args.sysroot);
        let sysroot = sysroot
            .canonicalize()
            .map_err(|e| OmaAptError::FailedGetCanonicalize(sysroot.display().to_string(), e))?;

        config.set("Dir", &sysroot.display().to_string());
        config.set(
            "Dir::State::status",
            &sysroot.join("var/lib/dpkg/status").display().to_string(),
        );

        debug!("Dir is: {:?}", config.get("Dir"));
        debug!(
            "Dir::State::status is: {:?}",
            config.get("Dir::State::status")
        );

        let install_recommend = if args.install_recommends {
            true
        } else if args.no_install_recommends {
            false
        } else {
            config.bool("APT::Install-Recommends", true)
        };

        let install_suggests = if args.install_suggests {
            true
        } else if args.no_install_suggests {
            false
        } else {
            config.bool("APT::Install-Suggests", false)
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
        let dir = self.config.get("Dir").unwrap_or_else(|| "/".to_string());
        let upgradable = self
            .cache
            .packages(&sort)
            .filter(|x| !is_hold(x.name(), &dir).unwrap_or(false))
            .count();

        let sort = PackageSort::default().auto_removable();
        let removable = self.cache.packages(&sort).count();

        Ok((upgradable, removable))
    }

    pub fn installed_packages(&self) -> OmaAptResult<usize> {
        let sort = PackageSort::default().installed();

        Ok(self.cache.packages(&sort).count())
    }

    /// Set apt manager status as upgrade
    pub fn upgrade(&self) -> OmaAptResult<()> {
        self.cache.upgrade(Upgrade::FullUpgrade)?;

        Ok(())
    }

    /// Set apt manager status as install
    pub fn install(
        &mut self,
        pkgs: &[PkgInfo],
        reinstall: bool,
    ) -> OmaAptResult<Vec<(String, String)>> {
        let mut no_marked_install = vec![];

        let install_recommends = self.config.bool("APT::Install-Recommends", true);

        for pkg in pkgs {
            let marked_install = mark_install(&self.cache, pkg, reinstall, install_recommends)?;

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
            } else if !self
                .select_pkgs
                .contains(&pkg.raw_pkg.fullname(true).to_string())
            {
                self.select_pkgs
                    .push(pkg.raw_pkg.fullname(true).to_string());
            }
        }

        Ok(no_marked_install)
    }

    /// Find system is broken
    pub fn check_broken(&self) -> OmaAptResult<bool> {
        let sort = PackageSort::default().installed();
        let pkgs = self.cache.packages(&sort);

        // let mut reinstall = vec![];

        let mut need = false;

        for pkg in pkgs {
            // current_state 的定义来自 apt 的源码:
            //    enum PkgCurrentState {NotInstalled=0,UnPacked=1,HalfConfigured=2,
            //    HalfInstalled=4,ConfigFiles=5,Installed=6,
            //    TriggersAwaited=7,TriggersPending=8};
            if pkg.current_state() != PkgCurrentState::Installed {
                debug!(
                    "pkg {} current state is {:?}",
                    pkg.name(),
                    pkg.current_state()
                );
                need = true;
                match pkg.current_state() {
                    PkgCurrentState::HalfInstalled => {
                        pkg.mark_reinstall(true);
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
        client: &Client,
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
            entry.name(pkg.raw_pkg.fullname(true).to_string());
            entry.new_version(ver.version().to_string());
            entry.new_size(install_size);
            entry.pkg_urls(ver.uris().collect::<Vec<_>>());
            entry.arch(ver.arch().to_string());
            entry.download_size(ver.size());
            entry.op(InstallOperation::Download);

            let sha256 = ver.get_record(RecordField::SHA256);
            let md5 = ver.get_record(RecordField::MD5sum);
            let sha512 = ver.sha512();

            if ver.uris().all(|x| !x.starts_with("file")) {
                if sha256
                    .as_ref()
                    .or(md5.as_ref())
                    .or(sha512.as_ref())
                    .is_none()
                {
                    return Err(OmaAptError::PkgNoChecksum(name));
                }
                entry.sha256(sha256);
                entry.md5(md5);
                entry.sha512(sha512);
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
            .build()
            .map_err(OmaAptError::FailedCreateAsyncRuntime)?;

        let res = tokio.block_on(async move {
            Self::download_pkgs(
                client,
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
    pub fn remove(
        &mut self,
        pkgs: &[PkgInfo],
        purge: bool,
        no_autoremove: bool,
    ) -> OmaAptResult<Vec<String>> {
        let mut no_marked_remove = vec![];
        for pkg in pkgs {
            let is_marked_delete = mark_delete(&self.cache, pkg, purge)?;
            if !is_marked_delete {
                no_marked_remove.push(pkg.raw_pkg.name().to_string());
            } else if !self.select_pkgs.contains(&pkg.raw_pkg.name().to_string()) {
                self.select_pkgs.push(pkg.raw_pkg.name().to_string());
            }
        }

        // 寻找系统有哪些不必要的软件包
        if !no_autoremove {
            // 需要先计算依赖才知道后面多少软件包是不必要的
            self.resolve(false, true)?;
            self.autoremove(purge)?;
        }

        Ok(no_marked_remove)
    }

    /// find autoremove and remove it
    fn autoremove(&mut self, purge: bool) -> OmaAptResult<()> {
        let sort = PackageSort::default().installed();
        let pkgs = self.cache.packages(&sort);

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
        client: &Client,
        network_thread: Option<usize>,
        args_config: &AptArgs,
        callback: F,
        op: OmaOperation,
    ) -> OmaAptResult<()>
    where
        F: Fn(usize, DownloadEvent, Option<u64>) + Clone + Send + Sync,
    {
        let v = op;
        let v_str = v.to_string();

        let sysroot = self.config.get("Dir").unwrap_or("/".to_string());

        if self.dry_run {
            debug!("op: {v:?}");
            return Ok(());
        }

        let download_pkg_list = v.install;

        let path = self.get_archive_dir();

        let conn = self.connection.clone();
        let (success, failed) = self.tokio.block_on(async move {
            if let Some(conn) = conn {
                change_status(&conn, "Downloading").await.ok();
            }

            Self::download_pkgs(client, download_pkg_list, network_thread, &path, callback).await
        })?;

        if !failed.is_empty() {
            return Err(OmaAptError::FailedToDownload(failed.len(), failed));
        }

        debug!("Success: {success:?}");

        let mut no_progress = AcquireProgress::quiet();

        debug!("Try to lock apt");

        if let Err(e) = apt_lock() {
            let e_str = e.to_string();
            if e_str.contains("dpkg --configure -a") {
                self.run_dpkg_configure()?;

                apt_lock()?;
            } else {
                return Err(e.into());
            }
        }

        debug!("Try to get apt archives");

        self.cache.get_archives(&mut no_progress).map_err(|e| {
            debug!("Get exception! Try to unlock apt lock");
            apt_unlock();
            e
        })?;

        let args = InstallProgressArgs {
            config: self.config,
            yes: args_config.yes,
            force_yes: args_config.force_yes,
            dpkg_force_confnew: args_config.dpkg_force_confnew,
            dpkg_force_all: args_config.dpkg_force_all,
            no_progress: args_config.no_progress,
            tokio: self.tokio,
            connection: self.connection,
        };

        let mut progress = InstallProgress::new(OmaAptInstallProgress::new(args));

        debug!("Try to unlock apt lock inner");

        apt_unlock_inner();

        debug!("Do install");

        self.cache.do_install(&mut progress).map_err(|e| {
            apt_lock_inner().ok();
            apt_unlock();
            e
        })?;

        debug!("Try to unlock apt lock");

        apt_unlock();

        let end_time = Local::now().format(TIME_FORMAT).to_string();

        let sysroot = Path::new(&sysroot);
        let history = sysroot.join("var/log/oma/history");
        let parent = history
            .parent()
            .ok_or_else(|| OmaAptError::FailedGetParentPath(history.clone()))?;

        std::fs::create_dir_all(parent)
            .map_err(|e| OmaAptError::FailedOperateDirOrFile(parent.display().to_string(), e))?;

        let mut log = std::fs::OpenOptions::new()
            .append(true)
            .create(true)
            .open(&history)
            .map_err(|e| OmaAptError::FailedOperateDirOrFile(history.display().to_string(), e))?;

        let start_time = Local::now();
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
    pub fn resolve(&mut self, no_fixbroken: bool, fix_dpkg_status: bool) -> OmaAptResult<()> {
        let need_fix_dpkg_status = self.check_broken()?;

        if no_fixbroken && need_fix_dpkg_status {
            warn!("Your system has broken status, Please run `oma fix-broken' to fix it.");
        }

        if need_fix_dpkg_status && fix_dpkg_status {
            self.run_dpkg_configure()?;
        }

        if !no_fixbroken {
            self.cache.fix_broken();
        }

        if self.cache.resolve(!no_fixbroken).is_err() {
            for pkg in self.cache.iter() {
                let res = show_broken_pkg(&self.cache, &pkg, false);
                if !res.is_empty() {
                    self.unmet.extend(res);
                }
            }
            return Err(OmaAptError::DependencyIssue(self.unmet.to_vec()));
        }

        Ok(())
    }

    fn run_dpkg_configure(&self) -> OmaAptResult<()> {
        info!(
            "Running {} ...",
            style("dpkg --configure -a").green().bold()
        );

        let cmd = Command::new("dpkg")
            .arg("--root")
            .arg(self.config.get("Dir").unwrap_or_else(|| "/".to_owned()))
            .arg("--configure")
            .arg("-a")
            .spawn()
            .map_err(OmaAptError::DpkgFailedConfigure)?;

        if let Err(e) = cmd.wait_with_output() {
            return Err(OmaAptError::DpkgFailedConfigure(io::Error::new(
                ErrorKind::Other,
                format!("dpkg return non-zero code: {:?}", e),
            )));
        }

        Ok(())
    }

    /// Download packages (inner)
    async fn download_pkgs<F>(
        client: &Client,
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
                        DownloadSourceType::Local { as_symlink: false }
                    } else {
                        DownloadSourceType::Http
                    };

                    DownloadSource::new(x.to_string(), source_type)
                })
                .collect::<Vec<_>>();

            debug!("Sources is: {:?}", sources);

            let filename = uris
                .first()
                .and_then(|x| x.split('/').last())
                .take()
                .ok_or_else(|| OmaAptError::InvalidFileName(entry.name().to_string()))?;

            debug!("filename is: {}", filename);

            let new_version = if console::measure_text_width(entry.new_version()) > 25 {
                console::truncate_str(entry.new_version(), 25, "...")
            } else {
                Cow::Borrowed(entry.new_version())
            };

            let msg = format!("{} {new_version} ({})", entry.name(), entry.arch());

            let mut download_entry = DownloadEntryBuilder::default();
            download_entry.source(sources);
            download_entry.filename(apt_style_filename(&entry).into());
            download_entry.dir(download_dir.to_path_buf());
            download_entry.allow_resume(true);
            download_entry.msg(msg);

            if let Some(checksum) = entry.sha256() {
                // TODO: 添加更多 checksum 类型
                download_entry.hash(Checksum::from_sha256_str(checksum)?);
            } else if let Some(checksum) = entry.sha512() {
                download_entry.hash(Checksum::from_sha512_str(checksum)?);
            } else if let Some(checksum) = entry.md5() {
                download_entry.hash(Checksum::from_md5_str(checksum)?);
            }

            let download_entry = download_entry.build()?;

            total_size += entry.download_size();

            download_list.push(download_entry);
        }

        let downloader = OmaFetcher::new(client, download_list, network_thread)?;

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
            &dpkg_arch(self.config.get("Dir").unwrap_or("/".to_string()))?,
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

        let res = oma_utils::dpkg::mark_version_status(
            pkgs,
            hold,
            dry_run,
            self.config.get("Dir").unwrap_or("/".to_string()),
        )?;

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
            .commit(&mut AcquireProgress::quiet(), &mut InstallProgress::apt())
            .map_err(|e| OmaAptError::CommitErr(e.to_string()))?;

        Ok(res)
    }

    /// Show changes summary
    pub fn summary(
        &self,
        how_handle_essential: impl Fn(&str) -> bool,
    ) -> OmaAptResult<OmaOperation> {
        let mut install = vec![];
        let mut remove = vec![];
        let changes = self.cache.get_changes(true);

        for pkg in changes {
            if pkg.marked_install() {
                let cand = pkg
                    .candidate()
                    .take()
                    .ok_or_else(|| OmaAptError::PkgNoCandidate(pkg.name().to_string()))?;

                let uri = cand.uris().collect::<Vec<_>>();
                let not_local_source = uri.iter().all(|x| !x.starts_with("file:"));
                let version = cand.version();
                let sha256 = cand.get_record(RecordField::SHA256);
                let md5 = cand.get_record(RecordField::MD5sum);
                let sha512 = cand.sha512();

                let size = cand.installed_size();

                let mut entry = InstallEntryBuilder::default();
                entry.name(pkg.fullname(true).to_string());
                entry.new_version(version.to_string());
                entry.new_size(size);
                entry.pkg_urls(uri);
                entry.arch(cand.arch().to_string());
                entry.download_size(cand.size());
                entry.op(InstallOperation::Install);
                entry.automatic(!self.select_pkgs.contains(&pkg.fullname(true)));

                if not_local_source {
                    if sha256
                        .as_ref()
                        .or(md5.as_ref())
                        .or(sha512.as_ref())
                        .is_none()
                    {
                        return Err(OmaAptError::PkgNoChecksum(pkg.to_string()));
                    }
                    entry.sha256(sha256);
                    entry.md5(md5);
                    entry.sha512(sha512);
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
                let name = pkg.fullname(true);

                if pkg.is_essential() && !how_handle_essential(&name) {
                    return Err(OmaAptError::PkgIsEssential(name));
                }

                let is_purge = pkg.marked_purge();

                let mut tags = vec![];
                if is_purge {
                    tags.push(RemoveTag::Purge);
                }

                if self.autoremove.contains(&pkg.fullname(true)) {
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
                let sha256 = version.get_record(RecordField::SHA256);
                let md5 = version.get_record(RecordField::MD5sum);
                let sha512 = version.sha512();
                let uri = version.uris().collect::<Vec<_>>();
                let not_local_source = uri.iter().all(|x| !x.starts_with("file:"));

                let mut entry = InstallEntryBuilder::default();
                entry.name(pkg.fullname(true));
                entry.new_version(version.version().to_string());
                entry.old_size(version.installed_size());
                entry.new_size(version.installed_size());
                entry.pkg_urls(uri);
                entry.arch(version.arch().to_string());
                entry.download_size(version.size());
                entry.op(InstallOperation::ReInstall);
                entry.automatic(!self.select_pkgs.contains(&pkg.fullname(true)));

                if not_local_source {
                    if sha256
                        .as_ref()
                        .or(md5.as_ref())
                        .or(sha512.as_ref())
                        .is_none()
                    {
                        return Err(OmaAptError::PkgNoChecksum(pkg.to_string()));
                    }
                    entry.sha256(sha256);
                    entry.md5(md5);
                    entry.sha512(sha512);
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
    pub fn check_disk_size(&self, op: &OmaOperation) -> OmaAptResult<()> {
        let (symbol, n) = &op.disk_size;
        let n = *n as i64;
        let download_size = op.total_download_size as i64;

        let need_space = match symbol.as_str() {
            "+" => download_size + n,
            "-" => download_size - n,
            _ => unreachable!(),
        };

        let available_disk_size =
            fs4::available_space(self.config.get("Dir").unwrap_or("/".to_string()))
                .map_err(OmaAptError::FailedGetAvailableSpace)? as i64;

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
    ) -> OmaAptResult<impl Iterator<Item = Package>> {
        let mut sort = PackageSort::default();

        debug!("Filter Mode: {query_mode:?}");

        for i in query_mode {
            sort = match i {
                FilterMode::Installed => sort.installed(),
                FilterMode::Upgradable => sort.upgradable(),
                FilterMode::Automatic => sort.auto_installed(),
                FilterMode::Names => sort.names(),
                FilterMode::Manual => sort.manually_installed(),
                _ => sort,
            };
        }

        let pkgs = self.cache.packages(&sort);

        Ok(pkgs)
    }
}

/// Mark package as delete.
fn mark_delete(cache: &Cache, pkg: &PkgInfo, purge: bool) -> OmaAptResult<bool> {
    let pkg = Package::new(cache, unsafe { pkg.raw_pkg.unique() });
    let removed_but_has_config = pkg.current_state() == PkgCurrentState::ConfigFiles;
    if !pkg.is_installed() && !removed_but_has_config {
        debug!(
            "Package {} is not installed. No need to remove.",
            pkg.name()
        );
        return Ok(false);
    }

    pkg.protect();
    pkg.mark_delete(purge || removed_but_has_config);

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

    let sha256 = cand.get_record(RecordField::SHA256);
    let md5 = cand.get_record(RecordField::MD5sum);
    let sha512 = cand.sha512();

    let mut install_entry = InstallEntryBuilder::default();
    install_entry.name(new_pkg.fullname(true).to_string());
    install_entry.old_version(old_version.to_string());
    install_entry.new_version(new_version.to_owned());
    install_entry.old_size(installed.installed_size());
    install_entry.new_size(cand.installed_size());
    install_entry.pkg_urls(cand.uris().collect::<Vec<_>>());
    install_entry.arch(cand.arch().to_string());
    install_entry.download_size(cand.size());
    install_entry.op(op);

    if not_local_source {
        if sha256
            .as_ref()
            .or(md5.as_ref())
            .or(sha512.as_ref())
            .is_none()
        {
            return Err(OmaAptError::PkgNoChecksum(new_pkg.to_string()));
        }
        install_entry.sha256(sha256);
        install_entry.md5(md5);
        install_entry.sha512(sha512);
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
    native_arch: &str,
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
            x => db.query_from_glob(
                x,
                filter_candidate,
                select_dbg,
                available_candidate,
                native_arch,
            )?,
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
fn mark_install(
    cache: &Cache,
    pkginfo: &PkgInfo,
    reinstall: bool,
    install_recommends: bool,
) -> OmaAptResult<bool> {
    let pkg = unsafe { pkginfo.raw_pkg.unique() }
        .make_safe()
        .ok_or_else(|| OmaAptError::PtrIsNone(PtrIsNone))?;
    let version = unsafe { pkginfo.version_raw.unique() }
        .make_safe()
        .ok_or_else(|| OmaAptError::PtrIsNone(PtrIsNone))?;
    let ver = Version::new(version, cache);
    let pkg = Package::new(cache, pkg);
    ver.set_candidate();

    if let Some(installed) = pkg.installed() {
        if installed.version() == ver.version()
            && !reinstall
            && installed.package_files().any(|inst| {
                ver.package_files()
                    .any(|ver| ver.archive() == inst.archive())
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

            if install_recommends {
                also_install_recommends(&ver, cache);
            }

            return Ok(is_marked);
        }
    }

    pkg.protect();

    mark_install_inner(&pkg);

    debug!("marked_install: {}", pkg.marked_install());
    debug!("marked_downgrade: {}", pkg.marked_downgrade());
    debug!("marked_upgrade: {}", pkg.marked_upgrade());
    debug!("marked_keep: {}", pkg.marked_keep());
    debug!("{} will marked install", pkg.name());

    Ok(true)
}

fn mark_install_inner(pkg: &Package) -> bool {
    // 根据 Packagekit 的源码
    // https://github.com/PackageKit/PackageKit/blob/a0a52ce90adb75a5df7ad1f0b1c9888f2eaf1a7b/backends/apt/apt-job.cpp#L388
    // 先标记 auto_inst 为 true 把所有该包的依赖标记为自动安装
    // 再把本包的 auto_inst 标记为 false，检查依赖问题
    pkg.mark_install(true, true);
    pkg.mark_install(false, true)
}

#[cfg(feature = "aosc")]
fn also_install_recommends(ver: &Version, cache: &Cache) {
    let recommends = ver.recommends();

    if let Some(recommends) = recommends {
        let group = crate::pkginfo::OmaDependency::map_deps(recommends);
        for base_deps in group.inner() {
            for dep in base_deps {
                if let Some(pkg) = cache.get(&dep.name) {
                    if !mark_install_inner(&pkg) {
                        warn!("Failed to mark install recommend: {}", dep.name);
                    } else {
                        pkg.protect();
                    }
                    continue;
                }
                warn!("Recommand {} does not exist.", dep.name);
            }
        }
    }
}

#[cfg(not(feature = "aosc"))]
fn also_install_recommends(_ver: &Version, _cache: &Cache) {}

fn show_broken_pkg(cache: &Cache, pkg: &Package, now: bool) -> Vec<String> {
    let mut result = vec![];
    // If the package isn't broken for the state Return None
    if (now && !pkg.is_now_broken()) || (!now && !pkg.is_inst_broken()) {
        return result;
    };

    let mut line = String::new();

    line += &format!("{} :", pkg.fullname(true));

    // Pick the proper version based on now status.
    // else Return with just the package name like Apt does.
    let Some(ver) = (match now {
        true => pkg.installed(),
        false => pkg.install_version(),
    }) else {
        result.push(line);
        return result;
    };

    let indent = pkg.fullname(false).len() + 3;
    let mut first = true;

    // ShowBrokenDeps
    for dep in ver.depends_map().values().flatten() {
        for (i, base_dep) in dep.iter().enumerate() {
            if !cache.depcache().is_important_dep(base_dep) {
                continue;
            }

            let dep_flag = if now {
                DepFlags::DepGNow
            } else {
                DepFlags::DepInstall
            };

            if cache.depcache().dep_state(base_dep) & dep_flag == dep_flag {
                continue;
            }

            if !first {
                line += &" ".repeat(indent);
            }
            first = false;

            // If it's the first or Dep
            if i > 0 {
                line += &" ".repeat(base_dep.dep_type().as_ref().len() + 3);
            } else {
                line += &format!(" {}: ", base_dep.dep_type())
            }

            line += &base_dep.target_package().fullname(true);

            if let (Ok(ver_str), Some(comp)) = (base_dep.target_ver(), base_dep.comp_type()) {
                line += &format!(" ({comp} {ver_str})");
            }

            let target = base_dep.target_package();
            if !target.has_provides() {
                if let Some(target_ver) = target.install_version() {
                    line += &format!(" but {target_ver} is to be installed")
                } else if target.candidate().is_some() {
                    line += " but it is not going to be installed";
                } else if target.has_provides() {
                    line += " but it is a virtual package";
                } else {
                    line += " but it is not installable";
                }
            }

            if i + 1 != dep.len() {
                line += " or"
            }
            result.push(line.clone());
            line.clear();
        }
    }

    result
}

/// trans filename to apt style file name
fn apt_style_filename(entry: &InstallEntry) -> String {
    let package = entry.name();
    let version = entry.new_version();
    let arch = entry.arch();

    let version = version.replace(':', "%3a");

    format!("{package}_{version}_{arch}.deb").replace("%2b", "+")
}
