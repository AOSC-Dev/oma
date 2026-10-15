use std::{
    cell::RefCell,
    cmp::Ordering,
    fmt,
    io::{self},
    path::{Path, PathBuf},
    process::Command,
    sync::Arc,
};

use ahash::{HashMap, HashSet};
use bon::Builder;
pub use oma_apt::cache::Upgrade;
use once_cell::sync::OnceCell;
use reqwest_middleware::ClientWithMiddleware;
use tokio::runtime::{Handle, Runtime};

use oma_apt::{
    BaseDep, DepFlags, DepType, Dependency, Package, PkgCurrentState, Version,
    cache::{Cache, PackageSort},
    error::AptErrors,
    new_cache,
    progress::{AcquireProgress, InstallProgress},
    raw::{IntoRawIter, config as apt_config},
    records::RecordField,
    util::{DiskSpace, cmp_versions},
};

use oma_fetch::{Event, Summary, checksum::ChecksumError};
use oma_utils::{GetLockError, dpkg::DpkgError, human_bytes::HumanBytes, is_termux};

pub use oma_apt::config::Config as AptConfig;
use oma_logger::debug;

#[cfg(feature = "aosc")]
use oma_logger::warn;

pub use oma_pm_operation_type::*;
use oma_utils::zbus::Connection;

use crate::{
    commit::{CommitConfig, CustomDownloadMessage, DoInstall},
    dbus::create_session,
    download::download_pkgs,
    lock::AptLockGuard,
    matches::MatcherError,
    pkginfo::{OmaDependency, OmaPackage, OmaPackageWithoutVersion, PtrIsNone},
    progress::InstallProgressManager,
    sort::SummarySort,
    utils::run_task_with_pump,
};

pub enum InstallProgressOpt {
    TermLike(Box<dyn InstallProgressManager>),
    Fd(i32),
}

#[derive(Debug, Clone, Builder)]
pub struct OmaAptArgs<'a> {
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
    another_apt_options: &'a [String],
}

pub struct OmaApt {
    /// Summary of all the apt operations in rust-apt.
    pub cache: Cache,
    /// The set of packages to be autoremoved.
    autoremove: HashSet<usize>,
    /// Toggle for dry-run mode.
    pub dry_run: bool,
    /// The set of selected packages.
    select_pkgs: HashSet<usize>,
    /// The recommendations a reinstall marked and protected: they are roots of
    /// the dependency walk, so what they need stays with them.
    recommend_roots: HashSet<usize>,
    /// A set of lists containing broken packages that have unmet dependencies.
    /// Each inner vector represents a group of packages with unmet dependencies.
    unmet: Vec<Vec<BrokenPackage>>,
    /// The path for archive.
    archive_dir: OnceCell<PathBuf>,
    pub(crate) async_handler: OnceCell<Handle>,
    pub(crate) async_runtime: OnceCell<Runtime>,
    pub(crate) conn: OnceCell<Connection>,
    sysroot: PathBuf,
    pub(crate) apt_lock: RefCell<Option<AptLockGuard>>,
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
    DependencyIssue {
        broken_dependencies: Vec<Vec<BrokenPackage>>,
        is_solver3: bool,
        apt_errors: AptErrors,
    },
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
    #[error("Unable to find package {0} on dpkg status file")]
    DpkgStatusGetPkg(String),
    #[error("Wrong dpkg status: {0}, package is not installed?")]
    WrongDpkgStatus(String),
    #[error("Package {0} status field is missing, dpkg status is broken.")]
    DpkgStatusBroken(String),
    #[error(transparent)]
    FailedGetArchiveDirLock(#[from] GetLockError),
    #[error("recv async event error")]
    RecvError,
    #[error(transparent)]
    Anyhow(#[from] anyhow::Error),
}

pub type OmaAptResult<T> = Result<T, OmaAptError>;

#[derive(Debug, Clone)]
pub struct DownloadConfig {
    /// The number of threads to be used for downloads.
    pub network_thread: Option<usize>,
    /// Path to downloaded files/archives.
    pub download_dir: Option<Arc<Path>>,
}

pub fn apt_config_get(key: String) -> Option<String> {
    let res = apt_config::find(key, "".to_string());
    if res.is_empty() { None } else { Some(res) }
}

pub fn apt_config_set_vector(key: &str, values: &[&str]) {
    let mut vec_key = String::from(key);
    if !vec_key.ends_with("::") {
        vec_key.push_str("::");
    }

    for value in values {
        apt_config::set(vec_key.to_string(), value.to_string());
    }
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
    pub fn new(local_debs: Vec<String>, args: OmaAptArgs, dry_run: bool) -> OmaAptResult<Self> {
        for path in &local_debs {
            if !Path::new(&path).exists() {
                return Err(OmaAptError::PathNotExist(path.to_string()));
            }
        }

        let sysroot = args.sysroot.clone();
        Self::init_config(args)?;

        Ok(Self {
            cache: new_cache!(&local_debs).map_err(OmaAptError::CreateCache)?,
            autoremove: HashSet::with_hasher(ahash::RandomState::new()),
            dry_run,
            select_pkgs: HashSet::with_hasher(ahash::RandomState::new()),
            recommend_roots: HashSet::with_hasher(ahash::RandomState::new()),
            unmet: vec![],
            archive_dir: OnceCell::new(),
            async_handler: OnceCell::new(),
            async_runtime: OnceCell::new(),
            conn: OnceCell::new(),
            sysroot: sysroot.into(),
            apt_lock: RefCell::new(None),
        })
    }

    /// Init apt config (before create new apt manager)
    fn init_config(args: OmaAptArgs) -> OmaAptResult<()> {
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

            apt_config::set("Dir".to_string(), sysroot.display().to_string());
        }

        debug!("Dir is: {:?}", apt_config_get("Dir".to_string()));
        debug!(
            "Dir::State::status is: {:?}",
            apt_config_get("Dir::State::status".to_string())
        );

        let install_recommend = if install_recommends {
            true
        } else if no_install_recommends {
            false
        } else {
            apt_config::find_bool("APT::Install-Recommends".to_string(), true)
        };

        let install_suggests = if install_suggests {
            true
        } else if no_install_suggests {
            false
        } else {
            apt_config::find_bool("APT::Install-Suggests".to_string(), false)
        };

        apt_config::set(
            "APT::Install-Recommends".to_string(),
            install_recommend.to_string(),
        );
        debug!("APT::Install-Recommends is set to {install_recommend}");

        apt_config::set(
            "APT::Install-Suggests".to_string(),
            install_suggests.to_string(),
        );
        debug!("APT::Install-Suggests is set to {install_suggests}");

        if yes {
            apt_config::set("APT::Get::Assume-Yes".to_string(), "true".to_string());
            debug!("APT::Get::Assume-Yes is set to true");
        }

        let mut dpkg_args = vec![];
        let opts = apt_config_get("Dpkg::Options::".to_string());
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
            apt_config::set("APT::Get::force-yes".to_string(), "true".to_string());
            debug!("APT::Get::force-Yes is set to true");
        }

        if dpkg_force_unsafe_io {
            dpkg_args.push("--force-unsafe-io");
            debug!("Dpkg::Options:: added --force-unsafe-io");
        }

        if yes || force_yes {
            unsafe { std::env::set_var("DEBIAN_FRONTEND", "noninteractive") };
        }

        let dir = apt_config_get("Dir".to_string()).unwrap_or_else(|| "/".to_owned());
        let root_arg = format!("--root={dir}");

        if !is_termux() {
            dpkg_args.push(&root_arg);
        }

        apt_config_set_vector("Dpkg::Options::", &dpkg_args);
        debug!("dpkg args: {dpkg_args:?}");

        for kv in another_apt_options {
            let (k, v) = kv.split_once('=').unwrap_or((kv, ""));
            apt_config::set(k.to_owned(), v.to_owned());
            debug!("{k}={v} is set");
        }

        Ok(())
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
            .get_or_try_init(|| -> Result<_, _> {
                run_task_with_pump(tokio, None::<&mut fn(Event)>, None, async {
                    create_session()
                        .await
                        .map_err(|e| anyhow::anyhow!("{e}").into())
                })
            })
            .inspect_err(|e| debug!("Failed to connect zbus session: {}", e))
            .ok();

        Ok(())
    }

    pub(crate) fn get_or_init_async_runtime(&self) -> Result<&Handle, OmaAptError> {
        self.async_handler
            .get_or_try_init(|| -> Result<_, _> {
                if let Ok(h) = tokio::runtime::Handle::try_current() {
                    Ok(h)
                } else {
                    let rt = self.async_runtime.get_or_try_init(|| {
                        tokio::runtime::Builder::new_multi_thread()
                            .enable_time()
                            .enable_io()
                            .build()
                    })?;

                    Ok(rt.handle().to_owned())
                }
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

        let install_recommends = apt_config::find_bool("APT::Install-Recommends".to_string(), true);

        for pkg in pkgs {
            let marked_install = mark_install(
                &self.cache,
                pkg,
                reinstall,
                install_recommends,
                &mut self.recommend_roots,
            )?;

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
        client: &ClientWithMiddleware,
        pkgs: Vec<OmaPackage>,
        config: DownloadConfig,
        custom_download_message: Option<CustomDownloadMessage>,
        mut callback: F,
    ) -> OmaAptResult<Summary>
    where
        F: FnMut(Event) + 'static,
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

        let download_list = Arc::from(download_list.as_ref());

        let client_ptr = client.clone();
        let msg_formatter = custom_download_message.unwrap_or_else(|| {
            Box::new(|i: &InstallEntry| {
                format!("{} {} {}", i.name(), i.new_version(), i.arch()).into()
            })
        });

        let (tx, rx) = flume::unbounded();
        let res = run_task_with_pump(
            self.get_or_init_async_runtime()?,
            Some(&mut callback),
            Some(rx),
            async move {
                download_pkgs(client_ptr, download_list, config, true, msg_formatter, tx).await
            },
        )?;

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
        apt_config::get_architectures()
    }

    /// Commit changes
    pub fn commit<F>(
        self,
        install_progress_manager: InstallProgressOpt,
        op: &OmaOperation,
        client: &ClientWithMiddleware,
        config: CommitConfig,
        custom_download_message: Option<CustomDownloadMessage>,
        callback: F,
    ) -> OmaAptResult<()>
    where
        F: FnMut(Event) + 'static,
    {
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
            apt_config::find("Dir".to_string(), "/".to_string())
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
            .is_ok_and(|dir| dir.count() != 0)
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
            return Err(OmaAptError::DependencyIssue {
                broken_dependencies: self.unmet.to_vec(),
                is_solver3: apt_config_get("APT::Solver".to_string()).is_some_and(|v| v == "3.0"),
                apt_errors: e,
            });
        }

        self.drop_unneeded_install_marks();

        Ok(())
    }

    /// Drop the automatic installs the transaction does not need.
    ///
    /// What the transaction is about is taken from its result: everything that
    /// stays installed, the packages that were asked for, apt's essential
    /// packages and the recommendations a reinstall marks, together with what
    /// they need. Dependencies count the way apt counts them — one alternative
    /// per dependency group, by version, as `is_important_dep` and
    /// `MarkInstall_InstallDependencies` do.
    ///
    /// Giving up on a mark can leave the transaction broken, so, like apt's
    /// `pkgMinimizeUpgrade`, the marks are given up on one at a time and put
    /// back when the resolution no longer holds, and the walk repeats while it
    /// made a difference. A mark something protects is a root of the walk, not
    /// something to give up on. Installed automatic packages are left alone;
    /// removing those is `autoremove`'s job.
    fn drop_unneeded_install_marks(&self) {
        // The marks something protects: they stay, so what they need stays
        // with them.
        let mut protected: HashSet<usize> = HashSet::default();
        // The removals the installs given up on asked for.
        let mut removals: Vec<Package<'_>> = vec![];
        loop {
            let unneeded = self.unneeded_install_marks(&protected);
            if unneeded.is_empty() {
                break;
            }

            let broken = self.cache.depcache().broken_count();
            let mut progressed = false;
            for pkg in unneeded {
                // What the install would remove goes with it, so it is read
                // before the mark is given up on.
                let removed = self.removals_of(&pkg);

                if !pkg.mark_keep() {
                    // Something protects it — the reinstall path marks the
                    // recommendations it follows itself — so the mark did not
                    // change: it is a root, not something to give up on.
                    progressed |= protected.insert(pkg.index());
                    continue;
                }

                if self.cache.depcache().broken_count() > broken {
                    // Something needs it after all: it is what keeps the
                    // version it is installed with satisfiable.
                    pkg.mark_install(true, false);
                    continue;
                }

                debug!(
                    "Unmarking {}: nothing in the transaction needs it",
                    pkg.fullname(true)
                );
                removals.extend(removed);
                progressed = true;
            }

            if !progressed {
                break;
            }
        }

        self.restore_removals(&removals);
    }

    /// Packages an install would remove: the installed ones its version
    /// conflicts with or breaks.
    fn removals_of(&self, pkg: &Package<'_>) -> Vec<Package<'_>> {
        let Some(version) = pkg.install_version() else {
            return vec![];
        };

        self.cache
            .packages(&PackageSort::default().installed())
            .filter(|removed| {
                removed.marked_delete()
                    && [DepType::Conflicts, DepType::DpkgBreaks]
                        .iter()
                        .any(|dep_type| {
                            version
                                .depends_map()
                                .get(dep_type)
                                .into_iter()
                                .flatten()
                                .flat_map(|group| group.iter())
                                .any(|dep| removes(dep, removed))
                        })
            })
            .collect()
    }

    /// Put back the removals a discarded install asked for: without the
    /// replacement arriving, the package it would have replaced is still what
    /// the system runs. A removal another install still wants stays, and so
    /// does one that cannot come back — keeping it would leave the transaction
    /// broken again. Giving one back can be what makes another possible, so it
    /// is tried again while it made a difference.
    fn restore_removals(&self, removals: &[Package<'_>]) {
        let mut pending: Vec<&Package<'_>> = removals
            .iter()
            .filter(|removed| {
                !self.select_pkgs.contains(&removed.index()) && removed.marked_delete()
            })
            .collect();

        loop {
            let mut progressed = false;
            let mut retry = vec![];

            for removed in pending {
                if !removed.marked_delete() {
                    // Something else already put it back.
                    continue;
                }

                // A removal another install still wants stays.
                let wanted = self
                    .cache
                    .packages(&PackageSort::default())
                    .filter(|pkg| pkg.marked_install())
                    .any(|pkg| {
                        self.removals_of(&pkg)
                            .iter()
                            .any(|other| other.index() == removed.index())
                    });

                if wanted {
                    continue;
                }

                let broken = self.cache.depcache().broken_count();
                if !removed.mark_keep() {
                    continue;
                }

                // Keeping it leaves the transaction broken: the removal was
                // needed for something else too — what the package depends on
                // is not there any more, or it conflicts with what is arriving.
                if self.cache.depcache().broken_count() > broken {
                    debug!(
                        "Keeping {} removed: the transaction needs it gone",
                        removed.fullname(true)
                    );
                    removed.mark_delete(false);
                    retry.push(removed);
                    continue;
                }

                progressed = true;
            }

            if !progressed {
                break;
            }

            pending = retry;
        }
    }

    /// The install marks outside what the transaction is about (see
    /// [`OmaApt::drop_unneeded_install_marks`]).
    fn unneeded_install_marks(&self, protected: &HashSet<usize>) -> Vec<Package<'_>> {
        let marked: Vec<Package<'_>> = self
            .cache
            .packages(&PackageSort::default())
            .filter(|pkg| pkg.marked_install())
            .collect();
        // The packages whose dependencies the walk follows: the ones going in,
        // a reinstall among them — it marks its recommendations itself
        // (`also_install_recommends`), so they are its dependencies like any
        // other.
        let mut going_in: HashSet<usize> = marked.iter().map(|pkg| pkg.index()).collect();

        // What was marked for a dependency of something: a new install
        // outside what the user asked for. An upgrade is something apt keeps
        // when the resolution — not the marking — dropped what required it,
        // and this cleanup cannot tell the two apart.
        let automatic: HashSet<usize> = marked
            .iter()
            .filter(|pkg| pkg.marked_new_install() && !self.select_pkgs.contains(&pkg.index()))
            // apt's upgrade passes install essential packages that are
            // missing and the resolution scores them so they stay
            // (`Scores::AddEssential`): they are not dependencies to give up
            // on.
            .filter(|pkg| !pkg.is_essential())
            .map(|pkg| pkg.index())
            .collect();
        // Nothing was marked as a dependency of something, so there is nothing
        // to give up on either.
        if automatic.is_empty() {
            return vec![];
        }

        // The versions that decide the dependencies: the ones going in, and
        // the installed ones that stay. Which version it is matters, an
        // installed `x 1` does not satisfy `Depends: x (>= 2)`, and a version
        // that is left behind — by an upgrade or a downgrade — satisfies
        // nothing.
        let mut kept: HashSet<(usize, String)> = HashSet::default();
        let mut transaction: HashSet<(usize, String)> = HashSet::default();
        let mut staying: HashSet<usize> = HashSet::default();
        let mut versions: HashMap<usize, Version<'_>> = HashMap::default();
        for pkg in self.cache.packages(&PackageSort::default().installed()) {
            if pkg.marked_delete() {
                // What a package that goes away needed is not needed any more.
                continue;
            }

            staying.insert(pkg.index());

            if pkg.marked_reinstall() {
                going_in.insert(pkg.index());
            }

            if pkg.marked_install() {
                // The version going in decides its dependencies, the installed
                // one stays behind.
                continue;
            }

            if let Some(installed_version) = pkg.installed() {
                let version = (pkg.index(), installed_version.version().to_string());
                kept.insert(version.clone());
                transaction.insert(version);
                versions.insert(pkg.index(), installed_version);
            }
        }
        for pkg in &marked {
            if let Some(install_version) = pkg.install_version() {
                let version = (pkg.index(), install_version.version().to_string());
                transaction.insert(version.clone());
                versions.insert(pkg.index(), install_version);

                // What satisfies a dependency is the version a package goes in
                // with, so an upgraded package still satisfies the groups it
                // did before (`DepInstall`).
                if pkg.is_installed() {
                    kept.insert(version);
                }
            }
        }

        let mut roots = staying;
        roots.extend(self.select_pkgs.iter().copied());
        roots.extend(protected.iter().copied());
        roots.extend(self.recommend_roots.iter().copied());
        // An essential package nothing depends on is not a dependency that can
        // be given up on, so its own dependencies count too.
        roots.extend(
            marked
                .iter()
                .filter(|pkg| pkg.is_essential())
                .map(|pkg| pkg.index()),
        );

        let mut needs: HashMap<usize, Vec<usize>> = HashMap::default();
        for (index, version) in &versions {
            let going_in = going_in.contains(index);
            let mut groups = vec![];
            for dep_type in [
                DepType::Depends,
                DepType::PreDepends,
                DepType::Recommends,
                DepType::Suggests,
            ] {
                for group in version.depends_map().get(&dep_type).into_iter().flatten() {
                    if going_in {
                        // Which dependencies apt follows is its own policy:
                        // `APT::Install-Recommends` and
                        // `APT::Install-Recommends-Sections`, plus the
                        // suggested counterparts.
                        if !self.cache.depcache().is_important_dep(group.first()) {
                            continue;
                        }
                    } else if !matches!(dep_type, DepType::Depends | DepType::PreDepends) {
                        // apt does not backfill the soft dependencies a package
                        // that stays installed as it is never had installed
                        // (`MarkInstall_InstallDependencies` ignores old
                        // unsatisfied important dependencies).
                        continue;
                    }

                    groups.push(
                        group
                            .iter()
                            .flat_map(|base_dep| base_dep.all_targets())
                            .map(|target| (target.parent().index(), target.version().to_string()))
                            .collect::<Vec<_>>(),
                    );
                }
            }
            needs.insert(*index, needed_targets(&groups, &kept, &transaction));
        }

        unneeded_automatic_installs(roots, &automatic, |index| {
            needs.get(&index).cloned().unwrap_or_default()
        })
        .into_iter()
        .filter_map(|index| marked.iter().find(|pkg| pkg.index() == index).cloned())
        .collect()
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

            PathBuf::from(apt_config::find_dir(
                "Dir::Cache::Archives".to_string(),
                "archives/".to_string(),
            ))
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

        let res = crate::dpkg::mark_status(
            pkgs.iter().map(|s| s.as_str()),
            self.sysroot(),
            hold,
            dry_run,
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
            .map_err(OmaAptError::MarkStatus)?;

        Ok(res)
    }

    pub fn ensure_apt_frontend_locked(&self) -> OmaAptResult<()> {
        let mut lock_slot = self.apt_lock.borrow_mut();
        if lock_slot.is_none() {
            let guard = AptLockGuard::acquire()?;
            *lock_slot = Some(guard);
        }
        Ok(())
    }

    /// Build transaction
    pub fn build_transaction(
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

        for pkg in self
            .cache
            .packages(&PackageSort::default().auto_removable())
        {
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
    let not_local_source = cand.uris().iter().all(|x| !x.starts_with("file:"));

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
    recommend_roots: &mut HashSet<usize>,
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
                also_install_recommends(&ver, cache, recommend_roots);
            }

            return Ok(is_marked);
        }
    }

    // apt-private/private-install.cc `TryToInstall`: the requested package is
    // cleared from the problem resolver's lists and protected there, so
    // resolution keeps it and gives up on what cannot be installed instead of
    // removing the conflicting package.
    cache.resolver().clear(&pkg);
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

/// Whether the `Conflicts`/`Breaks` alternative `dep` is about `removed`,
/// asked the way apt asks it (`CheckDep`): the version the package is
/// installed with — or the version it provides, for a virtual package — is
/// compared against what the dependency asks for. Versions matter, a
/// `Conflicts: d (<< 1)` is not about `d` installed in version 1.
fn removes(dep: &BaseDep<'_>, removed: &Package<'_>) -> bool {
    let Some(installed) = removed.installed() else {
        return false;
    };

    if dep.target_package().index() == removed.index() {
        return in_range(Some(installed.version()), dep.comp_type(), dep.version());
    }

    // A conflict against a virtual package is about the version the package
    // provides it with.
    installed.provides().any(|provider| {
        provider.name() == dep.name()
            && in_range(provider.version_str().ok(), dep.comp_type(), dep.version())
    })
}

/// Whether the version `candidate` answers a dependency asking for `want` with
/// the operator `comp_type`, as apt's `CheckDep` answers it: a dependency
/// without a version asks for nothing, a package — or a provides — without one
/// cannot answer it. `comp_type` is apt's, where `<` and `>` are already
/// strict.
fn in_range(candidate: Option<&str>, comp_type: Option<&str>, want: Option<&str>) -> bool {
    let Some(want) = want else {
        return true;
    };

    let Some(candidate) = candidate else {
        return false;
    };

    let Ok(order) = cmp_versions(candidate, want) else {
        return false;
    };

    match comp_type {
        Some("<") => order == Ordering::Less,
        Some("<=") => order != Ordering::Greater,
        Some("=") => order == Ordering::Equal,
        Some(">=") => order != Ordering::Less,
        Some(">") => order == Ordering::Greater,
        Some("!=") => order != Ordering::Equal,
        _ => false,
    }
}

/// What apt satisfies the dependency groups `groups` with, so that everything
/// reachable from them is needed: `groups` maps each dependency group to the
/// packages *and versions* that satisfy each of its alternatives, in
/// preference order — an installed `x 1` does not satisfy a `Depends: x
/// (>= 2)`, so which version it is matters.
///
/// apt resolves a dependency group to a single alternative
/// (`MarkInstall_InstallDependencies`): one that is already satisfied by
/// something that stays installed needs nothing new, and otherwise apt
/// installs the first alternative the transaction can. The other alternatives
/// of the group — one that only a dropped recommendation could satisfy, for
/// one — are not needed.
fn needed_targets(
    groups: &[Vec<(usize, String)>],
    kept: &HashSet<(usize, String)>,
    transaction: &HashSet<(usize, String)>,
) -> Vec<usize> {
    let mut targets = vec![];
    for group in groups {
        if group.iter().any(|version| kept.contains(version)) {
            continue;
        }

        if let Some(target) = group.iter().find(|version| transaction.contains(*version)) {
            targets.push(target.0);
        }
    }

    targets
}

/// The automatic installs that are outside the dependency closure of `roots`:
/// they were only marked for install for something that the resolution dropped
/// (see [`OmaApt::drop_unneeded_install_marks`]).
///
/// The roots themselves are needed, and so is what they require.
fn unneeded_automatic_installs(
    roots: impl IntoIterator<Item = usize>,
    automatic: &HashSet<usize>,
    needs: impl Fn(usize) -> Vec<usize>,
) -> Vec<usize> {
    let mut needed: HashSet<usize> = HashSet::default();
    let mut queue: Vec<usize> = Vec::new();
    for root in roots {
        if needed.insert(root) {
            queue.push(root);
        }
    }
    while let Some(index) = queue.pop() {
        for target in needs(index) {
            if needed.insert(target) {
                queue.push(target);
            }
        }
    }

    automatic
        .iter()
        .filter(|index| !needed.contains(index))
        .copied()
        .collect()
}

fn mark_install_inner(pkg: &Package) -> bool {
    // Like apt's `TryToInstall` + `doAutoInstall`: mark the request itself
    // first, then run the auto-install pass that pulls in what it needs —
    // dependencies, and with `APT::Install-Recommends` recommended packages —
    // as automatic installs.
    //
    // The reverse order (marking the request as an auto-install first, as
    // PackageKit's apt backend does) makes apt mark a recommended package's
    // whole dependency chain even when the recommended package itself cannot
    // be installed: only the recommendation is dropped later, its
    // dependencies stay marked and get installed as unneeded packages
    // (#847).
    pkg.mark_install(false, true);
    pkg.mark_install(true, true)
}

#[cfg(feature = "aosc")]
fn also_install_recommends(ver: &Version, cache: &Cache, recommend_roots: &mut HashSet<usize>) {
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
                        recommend_roots.insert(pkg.index());
                    }
                    continue;
                }
                warn!("Recommend {} does not exist.", dep.name);
            }
        }
    }
}

#[cfg(not(feature = "aosc"))]
fn also_install_recommends(_ver: &Version, _cache: &Cache, _recommend_roots: &mut HashSet<usize>) {}

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

#[cfg(test)]
mod needed_targets_tests {
    use super::needed_targets;
    use ahash::HashSet;

    fn set(versions: &[(usize, &str)]) -> HashSet<(usize, String)> {
        versions
            .iter()
            .map(|(index, version)| (*index, version.to_string()))
            .collect()
    }

    fn group(targets: &[(usize, &str)]) -> Vec<(usize, String)> {
        targets
            .iter()
            .map(|(index, version)| (*index, version.to_string()))
            .collect()
    }

    /// An alternative that stays installed needs nothing new, and the rest of
    /// the group is not needed for it.
    #[test]
    fn a_kept_alternative_satisfies_the_group() {
        // `1 | 2` with 1 installed: the leftover 2 does not become needed.
        assert!(
            needed_targets(
                &[group(&[(1, "1"), (2, "1")])],
                &set(&[(1, "1")]),
                &set(&[(1, "1"), (2, "1")]),
            )
            .is_empty()
        );
    }

    /// Otherwise apt installs the first alternative it can.
    #[test]
    fn the_first_alternative_of_an_or_group_wins() {
        // `1 | 2`, neither installed, both marked: apt installs 1, so 2 is
        // only kept when something else needs it.
        assert_eq!(
            needed_targets(
                &[group(&[(1, "1"), (2, "1")])],
                &set(&[]),
                &set(&[(1, "1"), (2, "1")]),
            ),
            vec![1]
        );
    }

    /// A group nothing in the transaction can satisfy adds nothing.
    #[test]
    fn unsatisfied_groups_are_skipped() {
        assert!(needed_targets(&[group(&[])], &set(&[]), &set(&[(1, "1")])).is_empty());
    }

    /// Every group contributes at most one alternative.
    #[test]
    fn each_group_contributes_one_target() {
        let targets = needed_targets(
            &[
                group(&[(1, "1")]),
                group(&[(3, "1"), (4, "1")]),
                group(&[(5, "1")]),
                group(&[(6, "1")]),
            ],
            &set(&[(6, "1")]),
            &set(&[(1, "1"), (3, "1"), (5, "1"), (6, "1")]),
        );

        assert_eq!(targets, vec![1, 3, 5]);
    }

    /// The version that is installed has to be the one that satisfies the
    /// dependency: with `x 1` installed and no `x 2` in the transaction,
    /// `Depends: x (>= 2) | y` is only satisfied by `y`.
    #[test]
    fn a_version_the_transaction_does_not_install_does_not_satisfy() {
        let targets = needed_targets(
            &[group(&[(1, "2"), (2, "1")])],
            &set(&[(1, "1")]),
            &set(&[(1, "1"), (2, "1")]),
        );

        assert_eq!(targets, vec![2]);
    }
}

#[cfg(test)]
mod unneeded_automatic_installs_tests {
    use super::unneeded_automatic_installs;
    use ahash::HashSet;

    fn set(indices: &[usize]) -> HashSet<usize> {
        indices.iter().copied().collect()
    }

    /// A dependency of a selected package stays; a package pulled in for
    /// something the resolution dropped goes (#847).
    #[test]
    fn drops_what_nothing_needs() {
        // 1 → 2 → 3 is the closure of the selected package; 4 was pulled in
        // as a dependency of a recommended package that is not installed any
        // more, so nothing that stays depends on it (it contributes no edge:
        // the recommended package is not marked).
        let needs = |index| match index {
            1 => vec![2],
            2 => vec![3],
            _ => vec![],
        };

        assert_eq!(
            unneeded_automatic_installs([1], &set(&[2, 3, 4]), needs),
            vec![4]
        );
    }

    /// With the recommendation installed, its dependency is needed as well.
    #[test]
    fn keeps_the_closure_of_every_root() {
        let needs = |index| match index {
            1 => vec![4],
            4 => vec![5],
            _ => vec![],
        };

        assert!(unneeded_automatic_installs([1], &set(&[4, 5]), needs).is_empty());
    }

    /// A package the user asked for is never dropped, auto or not.
    #[test]
    fn roots_are_kept() {
        let needs = |_| vec![];

        assert!(unneeded_automatic_installs([1, 2], &set(&[]), needs).is_empty());
    }

    /// A root is needed even when it is one of the candidates: what a
    /// reinstall marks itself stays.
    #[test]
    fn roots_are_needed_among_candidates() {
        let needs = |_| vec![];

        assert!(unneeded_automatic_installs([1, 2], &set(&[2]), needs).is_empty());
    }

    /// Cycles do not hang the walk.
    #[test]
    fn cycles_terminate() {
        let needs = |index| match index {
            1 => vec![2],
            2 => vec![1],
            _ => vec![],
        };

        assert!(unneeded_automatic_installs([1], &set(&[2]), needs).is_empty());
    }
}

#[cfg(test)]
mod version_range_tests {
    use super::in_range;

    /// The operators are the ones `comp_type()` gives apt's cache, where `<`
    /// and `>` are already strict.
    #[test]
    fn apt_operators() {
        assert!(in_range(Some("1"), Some("<"), Some("2")));
        assert!(!in_range(Some("1"), Some("<"), Some("1")));

        assert!(in_range(Some("1"), Some("<="), Some("1")));
        assert!(!in_range(Some("1"), Some("<="), Some("0")));

        assert!(in_range(Some("1"), Some("="), Some("1")));
        assert!(!in_range(Some("1"), Some("="), Some("2")));

        assert!(in_range(Some("2"), Some(">="), Some("1")));
        assert!(!in_range(Some("1"), Some(">="), Some("2")));

        assert!(in_range(Some("2"), Some(">"), Some("1")));
        assert!(!in_range(Some("1"), Some(">"), Some("1")));

        assert!(in_range(Some("1"), Some("!="), Some("2")));
        assert!(!in_range(Some("1"), Some("!="), Some("1")));
    }

    /// A dependency without a version asks for nothing, a package or provides
    /// without one cannot answer a versioned dependency.
    #[test]
    fn missing_versions() {
        assert!(in_range(Some("1"), None, None));
        assert!(in_range(None, None, None));
        assert!(!in_range(None, Some(">="), Some("1")));
        assert!(!in_range(Some(""), Some(">="), Some("1")));
    }

    /// Versions are compared the way dpkg compares them.
    #[test]
    fn debian_versions() {
        assert!(in_range(Some("1.0~rc1"), Some("<"), Some("1.0")));
        assert!(in_range(Some("1:1.0"), Some(">"), Some("9.9")));
        assert!(in_range(Some("1.0-2"), Some(">="), Some("1.0-1")));
    }
}
