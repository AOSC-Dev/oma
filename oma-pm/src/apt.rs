use std::{
    borrow::Cow,
    cell::OnceCell,
    fmt,
    io::{self, ErrorKind, Write},
    path::{Path, PathBuf},
    process::Command,
};

use ahash::HashSet;
use apt_auth_config::AuthConfig;
use bon::{builder, Builder};
use chrono::Local;

use flume::Sender;
pub use oma_apt::cache::Upgrade;

use oma_apt::{
    cache::{Cache, PackageSort},
    error::{AptError, AptErrors},
    new_cache,
    progress::{AcquireProgress, InstallProgress},
    raw::IntoRawIter,
    records::RecordField,
    util::{apt_lock, apt_lock_inner, apt_unlock, apt_unlock_inner, DiskSpace},
    DepFlags, Package, PkgCurrentState, Version,
};

use oma_console::console::{self};
use oma_fetch::{
    checksum::{Checksum, ChecksumError},
    reqwest::Client,
    DownloadEntry, DownloadError, DownloadManager, DownloadSource, DownloadSourceType, Event,
    Summary,
};
use oma_utils::{
    dpkg::{get_selections, is_hold, DpkgError},
    human_bytes::HumanBytes,
};

pub use oma_apt::config::Config as AptConfig;
use tokio::runtime::Runtime;
use tracing::{debug, info, warn};

pub use oma_pm_operation_type::*;
use zbus::{connection, Connection};

use crate::{
    dbus::{change_status, OmaBus, Status},
    matches::MatcherError,
    pkginfo::{OmaPackage, OmaPackageWithoutVersion, PtrIsNone},
    progress::{InstallProgressArgs, InstallProgressManager, OmaAptInstallProgress},
};

const TIME_FORMAT: &str = "%H:%M:%S on %Y-%m-%d";

#[derive(Debug, Clone, Builder)]
pub struct OmaAptArgs {
    #[builder(default = true)]
    install_recommends: bool,
    #[builder(default)]
    install_suggests: bool,
    #[builder(default)]
    no_install_recommends: bool,
    #[builder(default = true)]
    no_install_suggests: bool,
    #[builder(default = "/".to_string())]
    sysroot: String,
    #[builder(default)]
    yes: bool,
    #[builder(default)]
    force_yes: bool,
    #[builder(default)]
    dpkg_force_confnew: bool,
    #[builder(default)]
    dpkg_force_unsafe_io: bool,
    #[builder(default)]
    another_apt_options: Vec<String>,
}

pub struct OmaApt {
    /// The summary of all the apt operations in rust-apt
    pub cache: Cache,
    /// Config for apt, See Document for `AptConfig`.
    pub config: AptConfig,
    /// The hash of packages that will be autoremoved.
    autoremove: HashSet<u64>,
    /// Whether dry-run or not.
    dry_run: bool,
    /// The hash of packages that have been selected.
    select_pkgs: HashSet<u64>,
    /// The Tokio runtime used for asynchronous operations.
    /// It is required for any async operations within `OmaApt`
    tokio: Runtime,
    connection: Option<Connection>,
    /// A list of lists containing broken packages that have unmet dependencies.
    /// Each inner vector represents a group of packages with unmet dependencies.
    unmet: Vec<Vec<BrokenPackage>>,
    /// The path for archive.
    archive_dir: OnceCell<PathBuf>,
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
    OmaDatabaseError(#[from] MatcherError),
    #[error("Failed to mark package for reinstallation: {0}")]
    MarkReinstallError(String, String),
    #[error("Dependencies unmet")]
    DependencyIssue(Vec<Vec<BrokenPackage>>),
    #[error("Package: {0} is essential.")]
    PkgIsEssential(String),
    #[error("Package: {0} has no available candidate.")]
    PkgNoCandidate(String),
    #[error("Package: {0} has no SHA256 checksum.")]
    PkgNoChecksum(String),
    #[error("Package: {0}: {1} is not available from any mirror.")]
    PkgUnavailable(String, String),
    #[error("Invalid file name: {0}")]
    InvalidFileName(String),
    #[error(transparent)]
    DownloadError(#[from] DownloadError),
    #[error("Failed to create async runtime: {0}")]
    FailedCreateAsyncRuntime(std::io::Error),
    #[error("Failed to create file or directory: {0}: {1}")]
    FailedOperateDirOrFile(String, std::io::Error),
    #[error("Failed to calculate available storage space: {0}")]
    FailedGetAvailableSpace(std::io::Error),
    #[error("Failed to run `dpkg --configure -a': {0}")]
    DpkgFailedConfigure(std::io::Error),
    #[error("Failed to run `dpkg --triggers-only --pending': {0}")]
    DpkgTriggers(std::io::Error),
    #[error("Insufficient disk space: {0} needed, but only {1} is available.")]
    DiskSpaceInsufficient(HumanBytes, HumanBytes),
    #[error("Unable to commit change(s): {0}")]
    CommitErr(String),
    #[error("Failed to mark package status: {0} is not installed")]
    MarkPkgNotInstalled(String),
    #[error(transparent)]
    DpkgError(#[from] DpkgError),
    #[error("Failed to download {0} package(s).")]
    FailedToDownload(usize, Vec<DownloadError>),
    #[error("Failed to obtain parent path: {0:?}")]
    FailedGetParentPath(PathBuf),
    #[error("Failed to get canonicalized path: {0}")]
    FailedGetCanonicalize(String, std::io::Error),
    #[error(transparent)]
    PtrIsNone(#[from] PtrIsNone),
    #[error(transparent)]
    ChecksumError(#[from] ChecksumError),
    #[error("Blocking installation due to features markers.")]
    Features,
}

pub type OmaAptResult<T> = Result<T, OmaAptError>;

/// Represent the mode of filter.
#[derive(Debug)]
pub enum FilterMode {
    Default,
    Installed,
    Upgradable,
    Automatic,
    Manual,
    Names,
    AutoRemovable,
}

/// Represent the sort methods of summary.
#[derive(PartialEq, Eq)]
pub enum SummarySort {
    Names,
    Operation,
    NoSort,
}

pub struct DownloadConfig<'a> {
    /// Represent how many thread will be used in downloading
    pub network_thread: Option<usize>,
    /// The directory which stores downloaded files
    pub download_dir: Option<&'a Path>,
    /// The config for authorization
    pub auth: &'a AuthConfig,
}

pub struct CommitDownloadConfig<'a> {
    /// Represent how many thread will be used in downloading
    pub network_thread: Option<usize>,
    /// The config for authorization
    pub auth: &'a AuthConfig,
}

impl OmaApt {
    /// Create a new apt manager
    pub fn new(
        /// Path for local deb package files
        local_debs: Vec<String>,
        /// Config for oma, See Document for `OmaAptArgs`
        args: OmaAptArgs,
        /// Whether dry-run or not
        dry_run: bool,
        /// Config for apt, See Document for `AptConfig`
        config: AptConfig,
    ) -> OmaAptResult<Self> {
        let config = Self::init_config(config, args)?;

        let bus = OmaBus {
            status: Status::Configing,
        };

        let tokio = tokio::runtime::Builder::new_multi_thread()
            .enable_time()
            .enable_io()
            .build()
            .map_err(OmaAptError::FailedCreateAsyncRuntime)?;

        let conn = tokio.block_on(async {
            match Self::create_session(bus).await {
                Ok(conn) => Some(conn),
                Err(e) => {
                    debug!("Failed to create D-Bus session: {:?}", e);
                    None
                }
            }
        });

        Ok(Self {
            cache: new_cache!(&local_debs)?,
            config,
            autoremove: HashSet::with_hasher(ahash::RandomState::new()),
            dry_run,
            select_pkgs: HashSet::with_hasher(ahash::RandomState::new()),
            tokio,
            connection: conn,
            unmet: vec![],
            archive_dir: OnceCell::new(),
        })
    }

    async fn create_session(bus: OmaBus) -> Result<Connection, zbus::Error> {
        let conn = connection::Builder::system()?
            .name("io.aosc.Oma")?
            .serve_at("/io/aosc/Oma", bus)?
            .build()
            .await?;

        debug!("zbus session created");

        Ok(conn)
    }

    /// Init apt config (before create new apt manager)
    fn init_config(config: AptConfig, args: OmaAptArgs) -> OmaAptResult<AptConfig> {
        let OmaAptArgs {
            install_recommends,
            install_suggests,
            no_install_recommends,
            no_install_suggests,
            sysroot,
            yes,
            force_yes,
            dpkg_force_confnew,
            dpkg_force_unsafe_io,
            another_apt_options,
        } = args;

        let sysroot = Path::new(&sysroot);
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

        let install_recommend = if install_recommends {
            true
        } else if no_install_recommends {
            false
        } else {
            config.bool("APT::Install-Recommends", true)
        };

        let install_suggests = if install_suggests {
            true
        } else if no_install_suggests {
            false
        } else {
            config.bool("APT::Install-Suggests", false)
        };

        config.set("APT::Install-Recommends", &install_recommend.to_string());
        debug!("APT::Install-Recommends is set to {install_recommend}");

        config.set("APT::Install-Suggests", &install_suggests.to_string());
        debug!("APT::Install-Suggests is set to {install_suggests}");

        if yes {
            config.set("APT::Get::Assume-Yes", "true");
            debug!("APT::Get::Assume-Yes is set to true");
        }

        let mut dpkg_args = vec![];
        let opts = config.get("Dpkg::Options::");
        if let Some(ref opts) = opts {
            dpkg_args.push(opts.as_str());
        }

        if dpkg_force_confnew {
            dpkg_args.push("--force-confnew");
            debug!("Dpkg::Options:: is set to --force-confnew");
        } else if yes {
            // --force-confdef reason:
            // https://unix.stackexchange.com/questions/641099/any-possible-conflict-between-using-both-force-confold-and-force-confnew-wit/642541#642541
            let args = &["--force-confold", "--force-confdef"];
            dpkg_args.extend_from_slice(args);
            debug!("Dpkg::Options:: added --force-confold --force-confdef");
        }

        if force_yes {
            // warn!("{}", fl!("force-auto-mode"));
            config.set("APT::Get::force-yes", "true");
            debug!("APT::Get::force-Yes is set to true");
        }

        if dpkg_force_unsafe_io {
            dpkg_args.push("--force-unsafe-io");
            debug!("Dpkg::Options:: added --force-unsafe-io");
        }

        if yes || force_yes {
            std::env::set_var("DEBIAN_FRONTEND", "noninteractive");
        }

        let dir = config.get("Dir").unwrap_or("/".to_owned());
        let root_arg = format!("--root={dir}");
        dpkg_args.push(&root_arg);

        config.set_vector("Dpkg::Options::", &dpkg_args);

        for kv in another_apt_options {
            let (k, v) = kv.split_once('=').unwrap_or((&kv, ""));
            config.set(k, v);
            debug!("{k}={v} is set");
        }

        Ok(config)
    }

    /// Get upgradable packages count
    pub fn count_pending_upgradable_pkgs(&self) -> OmaAptResult<usize> {
        let sort = PackageSort::default().upgradable();
        let dir = self.config.get("Dir").unwrap_or_else(|| "/".to_string());
        let selection_status = get_selections(dir)?;
        let upgradable = self
            .cache
            .packages(&sort)
            .filter(|pkg| !is_hold(&pkg.fullname(true), &selection_status))
            .count();

        Ok(upgradable)
    }

    /// Get autoremovable packages count
    pub fn count_pending_autoremovable_pkgs(&self) -> usize {
        let sort = PackageSort::default().auto_removable();
        let auto_removable = self.cache.packages(&sort).count();

        auto_removable
    }

    pub fn count_installed_packages(&self) -> usize {
        let sort = PackageSort::default().installed();

        self.cache.packages(&sort).count()
    }

    /// Set apt manager status as upgrade
    pub fn upgrade(&self, mode: Upgrade) -> OmaAptResult<()> {
        self.cache.upgrade(mode)?;

        Ok(())
    }

    /// Set apt manager status as install
    pub fn install(
        &mut self,
        pkgs: &[OmaPackage],
        reinstall: bool,
    ) -> OmaAptResult<Vec<(String, String)>> {
        let mut no_marked_install = vec![];

        let install_recommends = self.config.bool("APT::Install-Recommends", true);

        for pkg in pkgs {
            let marked_install = mark_install(&self.cache, pkg, reinstall, install_recommends)?;

            let pkg_index = pkg.raw_pkg.index();

            debug!(
                "Pkg {} {} marked install: {marked_install}",
                pkg.raw_pkg.fullname(true),
                pkg.version_raw.version()
            );

            if !marked_install {
                no_marked_install.push((
                    pkg.raw_pkg.fullname(true),
                    pkg.version_raw.version().to_string(),
                ));
            } else if !self.select_pkgs.contains(&pkg_index) {
                self.select_pkgs.insert(pkg_index);
            }
        }

        Ok(no_marked_install)
    }

    /// Download packages
    pub fn download(
        &self,
        client: &Client,
        pkgs: Vec<OmaPackage>,
        config: DownloadConfig<'_>,
        tx: flume::Sender<Event>,
        dry_run: bool,
    ) -> OmaAptResult<(Vec<Summary>, Vec<DownloadError>)> {
        let DownloadConfig {
            network_thread,
            download_dir,
            auth,
        } = config;

        let mut download_list = vec![];
        for pkg in pkgs {
            let name = pkg.raw_pkg.fullname(true);
            let ver = Version::new(pkg.version_raw, &self.cache);
            let install_size = ver.installed_size();
            if !ver.is_downloadable() {
                return Err(OmaAptError::PkgUnavailable(name, ver.version().to_string()));
            }

            let sha256 = ver.get_record(RecordField::SHA256);
            let md5 = ver.get_record(RecordField::MD5sum);
            let sha512 = ver.sha512();

            if ver.uris().all(|x| !x.starts_with("file"))
                && sha256
                    .as_ref()
                    .or(md5.as_ref())
                    .or(sha512.as_ref())
                    .is_none()
            {
                return Err(OmaAptError::PkgNoChecksum(name));
            }

            let entry = InstallEntry::builder()
                .name(pkg.raw_pkg.fullname(true))
                .name_without_arch(pkg.raw_pkg.name().to_string())
                .new_version(ver.version().to_string())
                .new_size(install_size)
                .pkg_urls(ver.uris().collect::<Vec<_>>())
                .arch(ver.arch().to_string())
                .download_size(ver.size())
                .op(InstallOperation::Download)
                .maybe_sha256(sha256)
                .maybe_sha512(sha512)
                .maybe_md5(md5)
                .index(pkg.raw_pkg.index())
                .build();

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
                &download_list,
                network_thread,
                download_dir.unwrap_or(Path::new(".")),
                auth,
                tx,
            )
            .await
        })?;

        Ok(res)
    }

    /// Set apt manager status as remove
    pub fn remove(
        &mut self,
        pkgs: impl IntoIterator<Item = OmaPackageWithoutVersion>,
        purge: bool,
        no_autoremove: bool,
    ) -> OmaAptResult<Vec<String>> {
        debug!("is purge: {purge}");
        let mut no_marked_remove = vec![];

        for pkg in pkgs {
            let pkg = pkg.package(&self.cache);
            let is_marked_delete = mark_delete(&pkg, purge)?;
            if !is_marked_delete {
                no_marked_remove.push(pkg.fullname(true));
            } else if !self.select_pkgs.contains(&pkg.index()) {
                self.select_pkgs.insert(pkg.index());
            }
        }

        if !no_autoremove {
            // 需要先计算依赖才知道后面多少软件包是不必要的
            self.resolve(false, purge)?;

            // 删除不必要的软件包
            self.autoremove(purge)?;

            // 若设置了 purge 参数，则需要把解析器所有标记为已删除的包标记为 purge
            if purge {
                self.cache
                    .get_changes(false)
                    .filter(|pkg| pkg.marked_delete())
                    .for_each(|pkg| {
                        pkg.mark_delete(true);
                        pkg.protect();
                    });
            }
        }

        Ok(no_marked_remove)
    }

    /// find autoremove and remove it
    pub fn autoremove(&mut self, purge: bool) -> OmaAptResult<()> {
        let sort = PackageSort::default().installed();
        let pkgs = self.cache.packages(&sort);

        for pkg in pkgs {
            if pkg.is_auto_removable() && !pkg.marked_delete() {
                pkg.mark_delete(purge);
                pkg.protect();

                self.autoremove.insert(pkg.index());
            }
        }

        Ok(())
    }

    pub fn get_architectures(&self) -> Vec<String> {
        self.config.get_architectures()
    }

    /// Commit changes
    pub fn commit(
        self,
        client: &Client,
        config: CommitDownloadConfig<'_>,
        install_progress_manager: Box<dyn InstallProgressManager>,
        tx: Sender<Event>,
        op: &OmaOperation,
    ) -> OmaAptResult<()> {
        let CommitDownloadConfig {
            network_thread,
            auth,
        } = config;

        let v = op;
        let v_str = v.to_string();

        let sysroot = self.config.get("Dir").unwrap_or("/".to_string());

        if self.dry_run {
            debug!("op: {v:?}");
            return Ok(());
        }

        let download_pkg_list = &v.install;

        let path = self.get_archive_dir();

        let conn = self.connection.clone();
        let (success, failed) = self.tokio.block_on(async move {
            if let Some(conn) = conn {
                change_status(&conn, "Downloading").await.ok();
            }

            Self::download_pkgs(client, download_pkg_list, network_thread, path, auth, tx).await
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

        self.cache.get_archives(&mut no_progress).inspect_err(|e| {
            debug!("Get exception: {e}. Try to unlock apt lock");
            apt_unlock();
        })?;

        let args = InstallProgressArgs {
            config: self.config,
            tokio: self.tokio,
            connection: self.connection,
        };

        let mut progress =
            InstallProgress::new(OmaAptInstallProgress::new(args, install_progress_manager));

        debug!("Try to unlock apt lock inner");

        apt_unlock_inner();

        debug!("Do install");

        self.cache.do_install(&mut progress).inspect_err(|e| {
            debug!("do_install got except: {e}");
            apt_lock_inner().ok();
            apt_unlock();
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

    pub fn fix_broken(&mut self, fix_resolver: bool, fix_dpkg_status: bool) -> OmaAptResult<()> {
        if fix_resolver {
            self.cache.fix_broken();
        }

        self.cache.resolve(fix_resolver)?;
        let changes = self.cache.get_changes(false).collect::<Vec<_>>();

        let sort = PackageSort::default().installed();
        let pkgs = self.cache.packages(&sort);

        let mut need_reconfigure = false;
        let mut need_retriggers = false;

        let dpkg_update_path =
            Path::new(&self.config.get("Dir").unwrap_or_else(|| "/".to_string()))
                .join("var/lib/dpkg/updates");

        if dpkg_update_path
            .read_dir()
            .map_err(|e| {
                OmaAptError::FailedOperateDirOrFile(dpkg_update_path.display().to_string(), e)
            })?
            .count()
            != 0
        {
            need_reconfigure = true;
            need_retriggers = true;
        } else {
            for pkg in pkgs {
                // current_state 的定义来自 apt 的源码:
                //    enum PkgCurrentState {NotInstalled=0,UnPacked=1,HalfConfigured=2,
                //    HalfInstalled=4,ConfigFiles=5,Installed=6,
                //    TriggersAwaited=7,TriggersPending=8};
                if pkg.current_state() != PkgCurrentState::Installed && !changes.contains(&pkg) {
                    debug!(
                        "pkg {} current state is {:?}",
                        pkg.fullname(true),
                        pkg.current_state()
                    );
                    match pkg.current_state() {
                        PkgCurrentState::NotInstalled | PkgCurrentState::HalfInstalled => {
                            pkg.mark_reinstall(true);
                        }
                        PkgCurrentState::HalfConfigured | PkgCurrentState::UnPacked => {
                            need_reconfigure = true;
                        }
                        PkgCurrentState::TriggersAwaited | PkgCurrentState::TriggersPending => {
                            need_retriggers = true;
                        }
                        _ => continue,
                    }
                }
            }
        }

        if !fix_dpkg_status {
            if need_reconfigure || need_retriggers {
                warn!("Your system has broken status, Please run `oma fix-broken' to fix it.");
            }
            return Ok(());
        }

        if need_reconfigure {
            self.run_dpkg_configure()?;
        }

        if need_retriggers {
            self.run_dpkg_triggers()?;
        }

        Ok(())
    }

    /// Resolve apt dependencies
    pub fn resolve(&mut self, no_fixbroken: bool, all_purge: bool) -> OmaAptResult<()> {
        self.resolve_inner(no_fixbroken)?;

        if all_purge {
            self.cache
                .get_changes(false)
                .filter(|x| x.marked_delete())
                .for_each(|pkg| {
                    pkg.mark_delete(true);
                    pkg.protect();
                });

            self.resolve_inner(no_fixbroken)?;
        }

        Ok(())
    }

    fn resolve_inner(&mut self, no_fixbroken: bool) -> Result<(), OmaAptError> {
        if let Err(e) = self.cache.resolve(!no_fixbroken) {
            debug!("{e:#?}");
            for pkg in self.cache.iter() {
                let res = broken_pkg(&self.cache, &pkg, false);
                if !res.is_empty() {
                    self.unmet.extend(res);
                }
            }
            return Err(OmaAptError::DependencyIssue(self.unmet.to_vec()));
        }

        Ok(())
    }

    fn run_dpkg_configure(&self) -> OmaAptResult<()> {
        info!("Running `dpkg --configure -a' ...");

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

    fn run_dpkg_triggers(&self) -> OmaAptResult<()> {
        info!("Running `dpkg --triggers-only -a' ...");

        let cmd = Command::new("dpkg")
            .arg("--root")
            .arg(self.config.get("Dir").unwrap_or_else(|| "/".to_owned()))
            .arg("--triggers-only")
            .arg("-a")
            .spawn()
            .map_err(OmaAptError::DpkgFailedConfigure)?;

        if let Err(e) = cmd.wait_with_output() {
            return Err(OmaAptError::DpkgTriggers(io::Error::new(
                ErrorKind::Other,
                format!("dpkg return non-zero code: {:?}", e),
            )));
        }

        Ok(())
    }

    /// Download packages (inner)
    async fn download_pkgs(
        client: &Client,
        download_pkg_list: &[InstallEntry],
        network_thread: Option<usize>,
        download_dir: &Path,
        auth_config: &AuthConfig,
        sender: flume::Sender<Event>,
    ) -> OmaAptResult<(Vec<Summary>, Vec<DownloadError>)> {
        if download_pkg_list.is_empty() {
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
                        DownloadSourceType::Local(false)
                    } else {
                        let auth = auth_config.find_package_url(x);

                        DownloadSourceType::Http {
                            auth: auth.map(|x| (x.user.to_owned(), x.password.to_owned())),
                        }
                    };

                    DownloadSource {
                        url: x.to_string(),
                        source_type,
                    }
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

            let download_entry = DownloadEntry::builder()
                .source(sources)
                .filename(apt_style_filename(entry))
                .dir(download_dir.to_path_buf())
                .allow_resume(true)
                .msg(msg)
                .maybe_hash({
                    if let Some(checksum) = entry.sha256() {
                        Some(Checksum::from_sha256_str(checksum)?)
                    } else if let Some(checksum) = entry.sha512() {
                        Some(Checksum::from_sha512_str(checksum)?)
                    } else if let Some(checksum) = entry.md5() {
                        Some(Checksum::from_md5_str(checksum)?)
                    } else {
                        None
                    }
                })
                .build();

            total_size += entry.download_size();

            download_list.push(download_entry);
        }

        let downloader = DownloadManager::builder()
            .client(client)
            .download_list(download_list)
            .maybe_threads(network_thread)
            .total_size(total_size)
            .build();

        let recv = downloader.get_recviver().clone();

        let event_worker = tokio::spawn(async move {
            loop {
                match recv.recv_async().await {
                    Ok(v) => {
                        if let Err(e) = sender.send_async(v.clone()).await {
                            debug!("{e}");
                        }

                        if let Event::AllDone = v {
                            break;
                        }
                    }
                    Err(e) => {
                        debug!("{e}");
                        break;
                    }
                }
            }
        });

        let res = downloader.start_download().await;
        let _ = event_worker.await;

        let (mut success, mut failed) = (vec![], vec![]);

        for i in res {
            match i {
                Ok(s) => success.push(s),
                Err(e) => failed.push(e),
            }
        }

        Ok((success, failed))
    }

    /// Get apt archive dir
    pub fn get_archive_dir(&self) -> &Path {
        self.archive_dir.get_or_init(|| {
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
        })
    }

    /// Mark version status (hold/unhold)
    pub fn mark_version_status<'a>(
        &'a self,
        pkgs: &'a [String],
        hold: bool,
        dry_run: bool,
    ) -> OmaAptResult<Vec<(&'a str, bool)>> {
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
        pkgs: Vec<OmaPackage>,
        auto: bool,
        dry_run: bool,
    ) -> OmaAptResult<Vec<(String, bool)>> {
        let mut res = vec![];
        for pkg in pkgs {
            let pkg = Package::new(&self.cache, pkg.raw_pkg);

            if !pkg.is_installed() {
                return Err(OmaAptError::MarkPkgNotInstalled(pkg.fullname(true)));
            }

            if pkg.is_auto_installed() {
                if auto {
                    res.push((pkg.fullname(true), false));
                    debug!(
                        "pkg {} set to auto = {auto} is set = false",
                        pkg.fullname(true)
                    );
                } else {
                    pkg.mark_auto(false);
                    res.push((pkg.fullname(true), true));
                    debug!(
                        "pkg {} set to auto = {auto} is set = true",
                        pkg.fullname(true)
                    );
                }
            } else if auto {
                pkg.mark_auto(true);
                res.push((pkg.fullname(true), true));
                debug!(
                    "pkg {} set to auto = {auto} is set = true",
                    pkg.fullname(true)
                );
            } else {
                res.push((pkg.fullname(true), false));
                debug!(
                    "pkg {} set to auto = {auto} is set = false",
                    pkg.fullname(true)
                );
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
        sort: SummarySort,
        how_handle_essential: impl Fn(&str) -> bool,
        how_handle_features: impl Fn(&HashSet<Box<str>>) -> bool,
    ) -> OmaAptResult<OmaOperation> {
        #[cfg(feature = "aosc")]
        let mut features = HashSet::with_hasher(ahash::RandomState::new());

        #[cfg(not(feature = "aosc"))]
        let features = HashSet::with_hasher(ahash::RandomState::new());

        let mut install = vec![];
        let mut remove = vec![];
        let mut autoremovable = (0, 0);
        let changes = self.cache.get_changes(sort == SummarySort::Names);

        for pkg in changes {
            if pkg.marked_new_install() {
                let cand = pkg
                    .candidate()
                    .take()
                    .ok_or_else(|| OmaAptError::PkgNoCandidate(pkg.fullname(true)))?;

                let uri = cand.uris().collect::<Vec<_>>();
                let not_local_source = uri.iter().all(|x| !x.starts_with("file:"));
                let version = cand.version();
                let size = cand.installed_size();

                let mut sha256 = None;
                let mut md5 = None;
                let mut sha512 = None;

                if not_local_source {
                    sha256 = cand.get_record(RecordField::SHA256);
                    md5 = cand.get_record(RecordField::MD5sum);
                    sha512 = cand.sha512();

                    if sha256
                        .as_ref()
                        .or(md5.as_ref())
                        .or(sha512.as_ref())
                        .is_none()
                    {
                        return Err(OmaAptError::PkgNoChecksum(pkg.to_string()));
                    }
                }

                let entry = InstallEntry::builder()
                    .name(pkg.fullname(true))
                    .name_without_arch(pkg.name().to_string())
                    .new_version(version.to_string())
                    .new_size(size)
                    .pkg_urls(uri)
                    .arch(cand.arch().to_string())
                    .download_size(cand.size())
                    .op(InstallOperation::Install)
                    .automatic(self.select_pkgs.iter().all(|x| pkg.index() != *x))
                    .maybe_md5(md5)
                    .maybe_sha256(sha256)
                    .maybe_sha512(sha512)
                    .index(pkg.index())
                    .build();

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

                if !self.dry_run && pkg.is_essential() && !how_handle_essential(&name) {
                    return Err(OmaAptError::PkgIsEssential(name));
                }

                #[cfg(feature = "aosc")]
                if let Some(feat) = pkg
                    .installed()
                    .and_then(|x| x.get_record("X-AOSC-Features"))
                {
                    for i in feat.split_ascii_whitespace() {
                        if !features.contains(i) {
                            features.insert(Box::from(i));
                        }
                    }
                }

                let is_purge = pkg.marked_purge();

                let mut tags = vec![];
                if is_purge {
                    tags.push(RemoveTag::Purge);
                }

                if self.autoremove.contains(&pkg.index()) {
                    tags.push(RemoveTag::AutoRemove);
                }

                if !self.autoremove.contains(&pkg.index())
                    && !self.select_pkgs.contains(&pkg.index())
                {
                    tags.push(RemoveTag::Resolver);
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
                    pkg.index(),
                );

                remove.push(remove_entry);
            }

            if pkg.marked_reinstall() {
                // 如果一个包被标记为重装，则肯定已经安装
                // 所以请求已安装版本应该直接 unwrap
                let version = pkg.installed().unwrap();
                let uri = version.uris().collect::<Vec<_>>();
                let not_local_source = uri.iter().all(|x| !x.starts_with("file:"));

                let mut sha256 = None;
                let mut md5 = None;
                let mut sha512 = None;

                if not_local_source {
                    sha256 = version.get_record(RecordField::SHA256);
                    md5 = version.get_record(RecordField::MD5sum);
                    sha512 = version.sha512();

                    if sha256
                        .as_ref()
                        .or(md5.as_ref())
                        .or(sha512.as_ref())
                        .is_none()
                    {
                        return Err(OmaAptError::PkgNoChecksum(pkg.to_string()));
                    }
                }

                let entry = InstallEntry::builder()
                    .name(pkg.fullname(true))
                    .name_without_arch(pkg.name().to_string())
                    .new_version(version.version().to_string())
                    .old_size(version.installed_size())
                    .new_size(version.installed_size())
                    .pkg_urls(uri)
                    .arch(version.arch().to_string())
                    .download_size(version.size())
                    .op(InstallOperation::ReInstall)
                    .automatic(self.select_pkgs.iter().all(|x| pkg.index() != *x))
                    .maybe_sha256(sha256)
                    .maybe_sha512(sha512)
                    .maybe_md5(md5)
                    .index(pkg.index())
                    .build();

                install.push(entry);
            }

            if pkg.marked_downgrade() {
                let install_entry = pkg_delta(&pkg, InstallOperation::Downgrade)?;

                install.push(install_entry);
            }
        }

        for pkg in self.filter_pkgs(&[FilterMode::AutoRemovable])? {
            if !pkg.marked_delete() {
                let ver = pkg.installed().unwrap();
                autoremovable.0 += 1;
                autoremovable.1 += ver.installed_size();
            }
        }

        let disk_size = self.cache.depcache().disk_size();

        let disk_size = match disk_size {
            DiskSpace::Require(n) => ("+".into(), n),
            DiskSpace::Free(n) => ("-".into(), n),
        };

        let total_download_size = self.cache.depcache().download_size();

        if sort == SummarySort::Operation {
            let mut is_resolver_delete = vec![];
            for (index, i) in remove.iter().enumerate() {
                if i.details().contains(&RemoveTag::Resolver) {
                    is_resolver_delete.push(index);
                }
            }

            for i in is_resolver_delete {
                let entry = remove.remove(i);
                remove.insert(0, entry);
            }

            for i in &self.select_pkgs {
                if let Some(pos) = install.iter().position(|x| x.index() == *i) {
                    let entry = install.remove(pos);
                    install.insert(0, entry);
                }

                if let Some(pos) = remove.iter().position(|x| x.index() == *i) {
                    let entry = remove.remove(pos);
                    remove.insert(0, entry);
                }
            }
        }

        if !self.dry_run && !features.is_empty() && !how_handle_features(&features) {
            return Err(OmaAptError::Features);
        }

        Ok(OmaOperation {
            install,
            remove,
            disk_size,
            total_download_size,
            autoremovable,
        })
    }

    /// Check available disk space
    pub fn check_disk_size(&self, op: &OmaOperation) -> OmaAptResult<()> {
        let (symbol, n) = &op.disk_size;
        let n = *n as i64;
        let download_size = op.total_download_size as i64;

        let need_space = match symbol.as_ref() {
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
                FilterMode::AutoRemovable => sort.auto_removable(),
                _ => sort,
            };
        }

        let pkgs = self.cache.packages(&sort);

        Ok(pkgs)
    }
}

/// Mark package as delete.
fn mark_delete(pkg: &Package, purge: bool) -> OmaAptResult<bool> {
    if pkg.marked_delete() {
        return Ok(true);
    }

    let removed_but_has_config = pkg.current_state() == PkgCurrentState::ConfigFiles;

    debug!("removed_but_has_config: {}", removed_but_has_config);

    if !pkg.is_installed() && !removed_but_has_config {
        debug!(
            "Package {} is not installed. No need to remove.",
            pkg.fullname(true)
        );
        return Ok(false);
    }

    pkg.mark_delete(purge || removed_but_has_config);
    pkg.protect();

    Ok(true)
}

fn pkg_delta(new_pkg: &Package, op: InstallOperation) -> OmaAptResult<InstallEntry> {
    let cand = new_pkg
        .candidate()
        .take()
        .ok_or_else(|| OmaAptError::PkgNoCandidate(new_pkg.fullname(true)))?;

    let uri = cand.uris().collect::<Vec<_>>();
    let not_local_source = uri.iter().all(|x| !x.starts_with("file:"));

    let new_version = cand.version();
    // 如果一个包有版本修改，则肯定之前已经安装
    // 所以请求已安装版本应该直接 unwrap
    let installed = new_pkg.installed().unwrap();
    let old_version = installed.version();

    let mut sha256 = None;
    let mut md5 = None;
    let mut sha512 = None;

    if not_local_source {
        sha256 = cand.get_record(RecordField::SHA256);
        md5 = cand.get_record(RecordField::MD5sum);
        sha512 = cand.sha512();

        if sha256
            .as_ref()
            .or(md5.as_ref())
            .or(sha512.as_ref())
            .is_none()
        {
            return Err(OmaAptError::PkgNoChecksum(new_pkg.to_string()));
        }
    }

    let install_entry = InstallEntry::builder()
        .name(new_pkg.fullname(true))
        .name_without_arch(new_pkg.name().to_string())
        .old_version(old_version.to_string())
        .new_version(new_version.to_owned())
        .old_size(installed.installed_size())
        .new_size(cand.installed_size())
        .pkg_urls(cand.uris().collect::<Vec<_>>())
        .arch(cand.arch().to_string())
        .download_size(cand.size())
        .op(op)
        .maybe_sha256(sha256)
        .maybe_sha512(sha512)
        .maybe_md5(md5)
        .index(new_pkg.index())
        .build();

    Ok(install_entry)
}

/// Mark package as install.
fn mark_install(
    cache: &Cache,
    pkginfo: &OmaPackage,
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
                    pkg.fullname(true),
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

    debug!("marked_install: {}", pkg.marked_new_install());
    debug!("marked_downgrade: {}", pkg.marked_downgrade());
    debug!("marked_upgrade: {}", pkg.marked_upgrade());
    debug!("marked_keep: {}", pkg.marked_keep());
    debug!("{} will marked install", pkg.fullname(true));

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
                warn!("Recommend {} does not exist.", dep.name);
            }
        }
    }
}

#[cfg(not(feature = "aosc"))]
fn also_install_recommends(_ver: &Version, _cache: &Cache) {}

/// Represent a broken package, including the name and reason.
#[derive(Debug, Clone)]
pub struct BrokenPackage {
    pub name: String,
    pub why: (String, String),
    pub reason: Option<BrokenPackageReason>,
}

#[derive(Debug, Clone)]
pub enum BrokenPackageReason {
    ToBeInstall(String),
    NotGoingToBeInstall,
    VirtualPkg,
    NotInstallable,
}

impl fmt::Display for BrokenPackageReason {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            BrokenPackageReason::ToBeInstall(version) => {
                write!(f, "but {version} is to be installed")
            }
            BrokenPackageReason::NotGoingToBeInstall => {
                write!(f, "but it is not going to be installed")
            }
            BrokenPackageReason::VirtualPkg => write!(f, "but it is a virtual package"),
            BrokenPackageReason::NotInstallable => write!(f, "but it is not installable"),
        }
    }
}

fn broken_pkg(cache: &Cache, pkg: &Package, now: bool) -> Vec<Vec<BrokenPackage>> {
    let mut result = vec![];
    // If the package isn't broken for the state Return None
    if (now && !pkg.is_now_broken()) || (!now && !pkg.is_inst_broken()) {
        return result;
    };

    let name = pkg.fullname(true);

    // Pick the proper version based on now status.
    // else Return with just the package name like Apt does.
    let Some(ver) = (match now {
        true => pkg.installed(),
        false => pkg.install_version(),
    }) else {
        return result;
    };

    // ShowBrokenDeps
    for dep in ver.depends_map().values().flatten() {
        let mut v = vec![];
        for base_dep in dep.iter() {
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

            let mut dep_reason = base_dep.target_package().fullname(true);

            if let (Ok(ver_str), Some(comp)) = (base_dep.target_ver(), base_dep.comp_type()) {
                dep_reason += &format!(" ({comp} {ver_str})");
            }

            let why = (base_dep.dep_type().to_string(), dep_reason);

            let mut reason = None;

            let target = base_dep.target_package();
            if !target.has_provides() {
                if let Some(target_ver) = target.install_version() {
                    reason = Some(BrokenPackageReason::ToBeInstall(
                        target_ver.version().to_string(),
                    ));
                } else if target.candidate().is_some() {
                    reason = Some(BrokenPackageReason::NotGoingToBeInstall);
                } else if target.has_provides() {
                    // TODO: (By upstream ???)
                    reason = Some(BrokenPackageReason::VirtualPkg);
                } else {
                    reason = Some(BrokenPackageReason::NotInstallable);
                }
            }

            v.push(BrokenPackage {
                name: name.to_string(),
                why,
                reason,
            });
        }

        if !v.is_empty() {
            result.push(v);
        }
    }

    result
}

/// trans filename to apt style file name
fn apt_style_filename(entry: &InstallEntry) -> String {
    let package = entry.name_without_arch();
    let version = entry.new_version();
    let arch = entry.arch();

    let version = version.replace(':', "%3a");

    format!("{package}_{version}_{arch}.deb").replace("%2b", "+")
}
