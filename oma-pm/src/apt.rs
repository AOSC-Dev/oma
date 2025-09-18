use std::{
    fmt,
    io::{self},
    path::{Path, PathBuf},
    process::Command,
};

use ahash::HashSet;
use apt_auth_config::AuthConfig;
use bon::{Builder, builder};
pub use oma_apt::cache::Upgrade;
use once_cell::sync::OnceCell;
use tokio::runtime::Runtime;

use oma_apt::{
    DepFlags, Dependency, Package, PkgCurrentState, Version,
    cache::{Cache, PackageSort},
    error::AptErrors,
    new_cache,
    progress::{AcquireProgress, InstallProgress},
    raw::IntoRawIter,
    records::RecordField,
    util::DiskSpace,
};

use oma_fetch::{Event, Summary, checksum::ChecksumError, reqwest::Client};
use oma_utils::{dpkg::DpkgError, human_bytes::HumanBytes};

pub use oma_apt::config::Config as AptConfig;
use tracing::{debug, error};

#[cfg(feature = "aosc")]
use tracing::warn;

pub use oma_pm_operation_type::*;
use zbus::Connection;

use crate::{
    commit::{CommitNetworkConfig, CustomDownloadMessage, DoInstall},
    dbus::create_session,
    download::download_pkgs,
    matches::MatcherError,
    pkginfo::{OmaDependency, OmaPackage, OmaPackageWithoutVersion, PtrIsNone},
    progress::InstallProgressManager,
    sort::SummarySort,
    utils::is_termux,
};

pub enum InstallProgressOpt {
    TermLike(Box<dyn InstallProgressManager>),
    Fd(i32),
}

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
    /// Summary of all the apt operations in rust-apt.
    pub cache: Cache,
    /// See Document for `AptConfig`.
    pub config: AptConfig,
    /// The set of packages to be autoremoved.
    autoremove: HashSet<u64>,
    /// Toggle for dry-run mode.
    dry_run: bool,
    /// The set of selected packages.
    select_pkgs: HashSet<u64>,
    /// A set of lists containing broken packages that have unmet dependencies.
    /// Each inner vector represents a group of packages with unmet dependencies.
    unmet: Vec<Vec<BrokenPackage>>,
    /// The path for archive.
    archive_dir: OnceCell<PathBuf>,
    pub(crate) tokio: OnceCell<Runtime>,
    pub(crate) conn: OnceCell<Connection>,
    sysroot: PathBuf,
}

#[derive(Debug, thiserror::Error)]
pub enum OmaAptError {
    #[error("Failed to create packages index cache")]
    CreateCache(AptErrors),
    #[error("Path {0} does not exist.")]
    PathNotExist(String),
    #[error("Failed to set upgrade mode")]
    SetUpgradeMode(AptErrors),
    #[error("Failed to lock apt")]
    LockApt(AptErrors),
    #[error("Failed to install package(s)")]
    InstallPackages(AptErrors),
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
    #[error("Unable to mark package status")]
    MarkStatus(AptErrors),
    #[error("Failed to mark package status: {0} is not installed")]
    MarkPkgNotInstalled(String),
    #[error(transparent)]
    DpkgError(#[from] DpkgError),
    #[error("Failed to download {0} package(s).")]
    FailedToDownload(usize),
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

/// Modes of the result filter.
#[derive(Debug)]
pub enum FilterMode {
    Default,
    Installed,
    Upgradable,
    Automatic,
    Manual,
    Names,
    AutoRemovable,
    Hold,
}

pub struct DownloadConfig<'a> {
    /// The number of threads to be used for downloads.
    pub network_thread: Option<usize>,
    /// Path to downloaded files/archives.
    pub download_dir: Option<&'a Path>,
    /// Configuration for repository authorization.
    pub auth: Option<&'a AuthConfig>,
}

impl OmaApt {
    /// Create a new apt manager
    ///
    /// ### Parameters:
    ///
    /// `local_debs`: Path to local deb packages
    /// `args`: Configuration for oma, see documentation for `OmaAptArgs`
    /// `dry_run`: Toggle for dry-run mode
    /// `config`: Configuration for apt, see documentation for `AptConfig`
    pub fn new(
        local_debs: Vec<String>,
        args: OmaAptArgs,
        dry_run: bool,
        config: AptConfig,
    ) -> OmaAptResult<Self> {
        for path in &local_debs {
            if !Path::new(&path).exists() {
                return Err(OmaAptError::PathNotExist(path.to_string()));
            }
        }

        let sysroot = args.sysroot.clone();
        let config = Self::init_config(config, args)?;

        Ok(Self {
            cache: new_cache!(&local_debs).map_err(OmaAptError::CreateCache)?,
            config,
            autoremove: HashSet::with_hasher(ahash::RandomState::new()),
            dry_run,
            select_pkgs: HashSet::with_hasher(ahash::RandomState::new()),
            unmet: vec![],
            archive_dir: OnceCell::new(),
            tokio: OnceCell::new(),
            conn: OnceCell::new(),
            sysroot: sysroot.into(),
        })
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

        // Termux 的 apt 会默认的设置根，所以无需变动
        if !is_termux() {
            let sysroot = Path::new(&sysroot);
            let sysroot = sysroot.canonicalize().map_err(|e| {
                OmaAptError::FailedGetCanonicalize(sysroot.display().to_string(), e)
            })?;

            config.set("Dir", &sysroot.display().to_string());
            config.set(
                "Dir::State::status",
                &sysroot.join("var/lib/dpkg/status").display().to_string(),
            );
        }

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
            unsafe { std::env::set_var("DEBIAN_FRONTEND", "noninteractive") };
        }

        let dir = config.get("Dir").unwrap_or_else(|| "/".to_owned());
        let root_arg = format!("--root={dir}");

        if !is_termux() {
            dpkg_args.push(&root_arg);
        }

        config.set_vector("Dpkg::Options::", &dpkg_args);
        debug!("dpkg args: {dpkg_args:?}");

        for kv in another_apt_options {
            let (k, v) = kv.split_once('=').unwrap_or((&kv, ""));
            config.set(k, v);
            debug!("{k}={v} is set");
        }

        Ok(config)
    }

    /// Get upgradable packages count
    /// result.0 is upgradable package(s) count
    /// result.1 is upgradable package(s) but is held count
    pub fn count_pending_upgradable_pkgs(&self) -> (usize, usize) {
        let upgradable = PackageSort::default().upgradable();
        let upgradable_but_held = PackageSort::default().upgradable().hold_installed();

        (
            self.cache.packages(&upgradable).count(),
            self.cache.packages(&upgradable_but_held).count(),
        )
    }

    /// Get autoremovable packages count
    pub fn count_pending_autoremovable_pkgs(&self) -> usize {
        let sort = PackageSort::default().auto_removable();

        self.cache.packages(&sort).count()
    }

    pub fn count_installed_packages(&self) -> usize {
        let sort = PackageSort::default().installed();

        self.cache.packages(&sort).count()
    }

    pub fn init_dbus_status(&mut self) -> OmaAptResult<()> {
        let tokio = self.get_or_init_async_runtime()?;

        self.conn
            .get_or_try_init(|| -> Result<_, _> { tokio.block_on(create_session()) })
            .inspect_err(|e| debug!("Failed to connect zbus session: {}", e))
            .ok();

        Ok(())
    }

    pub(crate) fn get_or_init_async_runtime(&self) -> Result<&Runtime, OmaAptError> {
        self.tokio
            .get_or_try_init(|| -> Result<_, _> {
                tokio::runtime::Builder::new_multi_thread()
                    .enable_time()
                    .enable_io()
                    .build()
            })
            .map_err(OmaAptError::FailedCreateAsyncRuntime)
    }

    /// Set apt manager status as upgrade
    pub fn upgrade(&self, mode: Upgrade) -> OmaAptResult<()> {
        self.cache
            .upgrade(mode)
            .map_err(OmaAptError::SetUpgradeMode)?;

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
    pub fn download<F>(
        &self,
        client: &Client,
        pkgs: Vec<OmaPackage>,
        config: DownloadConfig<'_>,
        custom_download_message: Option<CustomDownloadMessage>,
        callback: F,
    ) -> OmaAptResult<Summary>
    where
        F: AsyncFn(Event),
    {
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

            if sha256
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
                .pkg_urls(get_package_url(&ver))
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

        if self.dry_run {
            return Ok(Summary {
                success: vec![],
                failed: vec![],
            });
        }

        let res = self.get_or_init_async_runtime()?.block_on(download_pkgs(
            client,
            &download_list,
            config,
            true,
            custom_download_message.unwrap_or(Box::new(|i: &InstallEntry| {
                format!("{} {} {}", i.name(), i.new_version(), i.arch()).into()
            })),
            callback,
        ))?;

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

        #[cfg(feature = "aosc")]
        let image_name = OnceCell::new();

        #[cfg(feature = "aosc")]
        let current_kernel_ver = OnceCell::new();

        for pkg in pkgs {
            #[cfg(feature = "aosc")]
            if pkg.name().starts_with("linux-kernel-") {
                let current_kernel_ver = current_kernel_ver
                    .get_or_init(|| sysinfo::System::kernel_version().unwrap_or_default());

                if crate::utils::pkg_is_current_kernel(
                    Path::new(&self.sysroot()),
                    &image_name,
                    pkg.name(),
                    current_kernel_ver,
                ) {
                    continue;
                }
            }

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
        install_progress_manager: InstallProgressOpt,
        op: &OmaOperation,
        client: &Client,
        config: CommitNetworkConfig,
        custom_download_message: Option<CustomDownloadMessage>,
        callback: impl AsyncFn(Event),
    ) -> OmaAptResult<()> {
        let sysroot = self.sysroot();

        if self.dry_run {
            debug!("op: {op:?}");
            return Ok(());
        }

        let commit = DoInstall::new(self, client, &sysroot, config)?;
        commit.commit(
            op,
            install_progress_manager,
            custom_download_message.unwrap_or_else(|| {
                Box::new(|i| format!("{} {} {}", i.name(), i.new_version(), i.arch()).into())
            }),
            callback,
        )?;

        Ok(())
    }

    fn sysroot(&self) -> String {
        if is_termux() {
            self.sysroot.display().to_string()
        } else {
            self.config.get("Dir").unwrap_or_else(|| "/".to_string())
        }
    }

    pub fn fix_resolver_broken(&self) {
        self.cache.fix_broken();
    }

    pub fn fix_dpkg_status(
        &self,
        need_reconfigure: bool,
        need_retriggers: bool,
    ) -> OmaAptResult<()> {
        if self.dry_run {
            debug!("In dry-run mode, no fix dpkg status");
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

    pub fn is_needs_fix_dpkg_status(&self) -> Result<(bool, bool), OmaAptError> {
        let sort = PackageSort::default().installed();
        let pkgs = self.cache.packages(&sort);
        let mut need_reconfigure = false;
        let mut need_retriggers = false;
        let dpkg_update_path = Path::new(&self.sysroot()).join("var/lib/dpkg/updates");

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
                if pkg.current_state() == PkgCurrentState::Installed {
                    continue;
                }

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

        debug!(
            "need_reconfigure: {} need_retriggers: {}",
            need_reconfigure, need_retriggers
        );

        Ok((need_reconfigure, need_retriggers))
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

    pub(crate) fn run_dpkg_configure(&self) -> OmaAptResult<()> {
        debug!("Running `dpkg --configure -a' ...");

        let cmd = Command::new("dpkg")
            .arg("--root")
            .arg(self.sysroot())
            .arg("--configure")
            .arg("-a")
            .spawn()
            .map_err(OmaAptError::DpkgFailedConfigure)?;

        dpkg_exit_status(cmd)
    }

    pub(crate) fn run_dpkg_triggers(&self) -> OmaAptResult<()> {
        debug!("Running `dpkg --triggers-only -a' ...");

        let cmd = Command::new("dpkg")
            .arg("--root")
            .arg(self.sysroot())
            .arg("--triggers-only")
            .arg("-a")
            .spawn()
            .map_err(OmaAptError::DpkgFailedConfigure)?;

        dpkg_exit_status(cmd)
    }

    /// Get apt archive dir
    pub fn get_archive_dir(&self) -> &Path {
        self.archive_dir.get_or_init(|| {
            if is_termux() {
                return PathBuf::from("/data/data/com.termux/cache/apt/archives/");
            }

            let archives_dir = self
                .config
                .get("Dir::Cache::Archives")
                .unwrap_or_else(|| "archives/".to_string());

            let cache = self
                .config
                .get("Dir::Cache")
                .unwrap_or_else(|| "var/cache/apt".to_string());

            let dir = self.config.get("Dir").unwrap_or_else(|| "/".to_string());

            let archive_dir_p = PathBuf::from(archives_dir);
            if archive_dir_p.is_absolute() {
                return archive_dir_p;
            }

            debug!("archive_dir_p is: {}", archive_dir_p.display());

            let cache_dir_p = PathBuf::from(cache);
            if cache_dir_p.is_absolute() {
                return cache_dir_p.join(archive_dir_p);
            }

            debug!("cache_dir_p is: {}", cache_dir_p.display());

            let dir_p = PathBuf::from(dir);

            debug!("dir_p is: {}", dir_p.display());

            let res = dir_p.join(cache_dir_p).join(archive_dir_p);

            debug!("get_archive_dir is: {}", res.display());

            res
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

        let res = oma_utils::dpkg::mark_version_status(pkgs, hold, dry_run, self.sysroot())?;

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
            .map_err(OmaAptError::MarkStatus)?;

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
        let changes = self.cache.get_changes(sort.names);

        let mut suggest = HashSet::with_hasher(ahash::RandomState::new());
        let mut recommend = HashSet::with_hasher(ahash::RandomState::new());

        for pkg in changes {
            if pkg.marked_new_install() {
                let cand = pkg
                    .candidate()
                    .ok_or_else(|| OmaAptError::PkgNoCandidate(pkg.fullname(true)))?;

                let uri = get_package_url(&cand);

                let version = cand.version();
                let size = cand.installed_size();

                let (sha256, md5, sha512) = get_package_checksums(&pkg, &cand)?;

                collect_recommends_and_suggests(&self.cache, &mut suggest, &mut recommend, &cand);

                let entry = InstallEntry::builder()
                    .name(pkg.fullname(true))
                    .name_without_arch(pkg.name().to_string())
                    .new_version(version.to_string())
                    .new_size(size)
                    .pkg_urls(uri)
                    .arch(cand.arch().to_string())
                    .download_size(cand.size())
                    .op(InstallOperation::Install)
                    .automatic(!self.select_pkgs.contains(&pkg.index()))
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
                let install_entry = pkg_delta(
                    &pkg,
                    InstallOperation::Upgrade,
                    &self.cache,
                    &mut suggest,
                    &mut recommend,
                )?;

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

                let (sha256, md5, sha512) = get_package_checksums(&pkg, &version)?;

                collect_recommends_and_suggests(
                    &self.cache,
                    &mut suggest,
                    &mut recommend,
                    &version,
                );

                let entry = InstallEntry::builder()
                    .name(pkg.fullname(true))
                    .name_without_arch(pkg.name().to_string())
                    .new_version(version.version().to_string())
                    .old_size(version.installed_size())
                    .new_size(version.installed_size())
                    .pkg_urls(get_package_url(&version))
                    .arch(version.arch().to_string())
                    .download_size(version.size())
                    .op(InstallOperation::ReInstall)
                    .automatic(!self.select_pkgs.contains(&pkg.index()))
                    .maybe_sha256(sha256)
                    .maybe_sha512(sha512)
                    .maybe_md5(md5)
                    .index(pkg.index())
                    .build();

                install.push(entry);
            }

            if pkg.marked_downgrade() {
                let install_entry = pkg_delta(
                    &pkg,
                    InstallOperation::Downgrade,
                    &self.cache,
                    &mut suggest,
                    &mut recommend,
                )?;

                install.push(install_entry);
            }
        }

        for pkg in self.filter_pkgs(&[FilterMode::AutoRemovable])? {
            if !pkg.marked_delete() {
                let ver = pkg.installed().unwrap_or_else(|| {
                    // 有可能存在操作之后立即是孤包的情况
                    // 我们需要拿取将要安装的版本
                    // 也许，搜寻自动删除的逻辑应该在操作完成之后？
                    pkg.candidate().unwrap()
                });
                autoremovable.0 += 1;
                autoremovable.1 += ver.installed_size();
            }
        }

        let disk_size = self.cache.depcache().disk_size();

        let disk_size_delta = match disk_size {
            DiskSpace::Require(n) => n as i64,
            DiskSpace::Free(n) => 0 - n as i64,
        };

        let total_download_size = self.cache.depcache().download_size();

        if sort.operation {
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
            disk_size_delta,
            total_download_size,
            autoremovable,
            suggest: suggest.into_iter().collect(),
            recommend: recommend.into_iter().collect(),
        })
    }

    /// Check available disk space
    pub fn check_disk_size(&self, op: &OmaOperation) -> OmaAptResult<()> {
        let download_size = op.total_download_size as i64;

        let need_space = download_size + op.disk_size_delta;

        let available_disk_size = fs4::available_space(self.sysroot())
            .map_err(OmaAptError::FailedGetAvailableSpace)?
            as i64;

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
    ) -> OmaAptResult<impl Iterator<Item = Package<'_>>> {
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
                FilterMode::Hold => sort.hold_installed(),
                _ => sort,
            };
        }

        let pkgs = self.cache.packages(&sort);

        Ok(pkgs)
    }
}

fn dpkg_exit_status(mut cmd: std::process::Child) -> Result<(), OmaAptError> {
    match cmd.wait() {
        Ok(status) => match status.code() {
            Some(0) => Ok(()),
            Some(x) => Err(OmaAptError::DpkgFailedConfigure(io::Error::other(format!(
                "dpkg returned non-zero code: {x}",
            )))),
            None => Err(OmaAptError::DpkgFailedConfigure(io::Error::other(
                "Could not get dpkg exit status",
            ))),
        },
        Err(e) => Err(OmaAptError::DpkgFailedConfigure(io::Error::other(e))),
    }
}

fn get_package_url(cand: &Version<'_>) -> Vec<PackageUrl> {
    cand.version_files()
        .filter_map(|v| {
            let pkg_file = v.package_file();
            if !pkg_file.is_downloadable() {
                return None;
            }

            Some(PackageUrl {
                download_url: pkg_file.index_file().archive_uri(&v.lookup().filename()),
                index_url: pkg_file.index_file().archive_uri(""),
            })
        })
        .collect::<Vec<_>>()
}

fn collect_recommends_and_suggests(
    cache: &Cache,
    suggest: &mut HashSet<(String, String)>,
    recommend: &mut HashSet<(String, String)>,
    version: &Version<'_>,
) {
    if let Some(s) = version.depends_map().get(&oma_apt::DepType::Suggests) {
        collect_suggest(cache, suggest, s);
    }

    if let Some(r) = version.depends_map().get(&oma_apt::DepType::Recommends) {
        collect_suggest(cache, recommend, r);
    }
}

fn collect_suggest(
    cache: &Cache,
    suggest: &mut HashSet<(String, String)>,
    packages: &[Dependency<'_>],
) {
    for deps in OmaDependency::map_deps(packages).inner() {
        if deps.len() == 1 {
            let suggest_package = deps.first().unwrap();
            let name = &suggest_package.name;
            let pkg = cache.get(name);

            let Some(pkg) = pkg else {
                continue;
            };

            if !pkg.marked_install() && !pkg.is_installed() {
                let Some(cand) = pkg.candidate() else {
                    continue;
                };

                let Some(desc) = cand.summary() else {
                    continue;
                };

                suggest.insert((pkg.fullname(true), desc));
            }
        } else {
            let pkgs = deps
                .iter()
                .map(|x| &x.name)
                .flat_map(|pkg| cache.get(pkg))
                .collect::<Vec<_>>();

            let all_not_marked_install_and_is_installed = pkgs
                .iter()
                .all(|pkg| !pkg.marked_install() && !pkg.is_installed());

            if all_not_marked_install_and_is_installed {
                for pkg in pkgs {
                    let Some(cand) = pkg.candidate() else {
                        continue;
                    };

                    let Some(desc) = cand.summary() else {
                        continue;
                    };

                    suggest.insert((pkg.fullname(true), desc));
                }
            }
        }
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

fn pkg_delta(
    new_pkg: &Package,
    op: InstallOperation,
    cache: &Cache,
    suggest: &mut HashSet<(String, String)>,
    recommend: &mut HashSet<(String, String)>,
) -> OmaAptResult<InstallEntry> {
    let cand = new_pkg
        .candidate()
        .ok_or_else(|| OmaAptError::PkgNoCandidate(new_pkg.fullname(true)))?;

    let new_version = cand.version();
    // 如果一个包有版本修改，则肯定之前已经安装
    // 所以请求已安装版本应该直接 unwrap
    let installed = new_pkg.installed().unwrap();
    let old_version = installed.version();

    let (sha256, md5, sha512) = get_package_checksums(new_pkg, &cand)?;

    collect_recommends_and_suggests(cache, suggest, recommend, &cand);

    let install_entry = InstallEntry::builder()
        .name(new_pkg.fullname(true))
        .name_without_arch(new_pkg.name().to_string())
        .old_version(old_version.to_string())
        .new_version(new_version.to_owned())
        .old_size(installed.installed_size())
        .new_size(cand.installed_size())
        .pkg_urls(get_package_url(&cand))
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

type PkgChecksums = (Option<String>, Option<String>, Option<String>);

fn get_package_checksums(
    new_pkg: &Package<'_>,
    cand: &Version<'_>,
) -> Result<PkgChecksums, OmaAptError> {
    let not_local_source = cand.uris().all(|x| !x.starts_with("file:"));

    let sha256 = cand.get_record(RecordField::SHA256);
    let md5 = cand.get_record(RecordField::MD5sum);
    let sha512 = cand.sha512();
    if not_local_source
        && sha256
            .as_ref()
            .or(md5.as_ref())
            .or(sha512.as_ref())
            .is_none()
    {
        return Err(OmaAptError::PkgNoChecksum(new_pkg.to_string()));
    }

    Ok((sha256, md5, sha512))
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
        .ok_or(OmaAptError::PtrIsNone(PtrIsNone))?;
    let version = unsafe { pkginfo.version_raw.unique() }
        .make_safe()
        .ok_or(OmaAptError::PtrIsNone(PtrIsNone))?;
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

    debug!("pkg: {}", pkg.fullname(true));
    debug!("marked_new_install: {}", pkg.marked_new_install());
    debug!("marked_install: {}", pkg.marked_install());
    debug!("marked_downgrade: {}", pkg.marked_downgrade());
    debug!("marked_upgrade: {}", pkg.marked_upgrade());
    debug!("marked_keep: {}", pkg.marked_keep());

    Ok(pkg.marked_install())
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

/// Broken package, struct contains its name and cause for broken status.
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
