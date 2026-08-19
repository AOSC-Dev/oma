use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::process::Command;

use flume::Receiver;
use oma_fetch::{
    DownloadEntry, DownloadManager, DownloadSource, DownloadSourceType, Event, checksum::Checksum,
    download::BuilderError,
};
use once_cell::sync::OnceCell;
use reqwest_middleware::ClientWithMiddleware;
use thiserror::Error;
use tokio::runtime::{Handle, Runtime};

use crate::{AptConfig, AptDb};

use super::{DpkgOp, LockError, LockGuard, Transaction};

/// Errors from executing a [`DpkgPlan`] (download or dpkg).
#[derive(Debug, Error)]
pub enum ExecutorError {
    /// The plan references a package/version with no entry in the index.
    #[error("no index entry for {package} {version:?}")]
    EntryNotFound {
        package: String,
        version: Option<String>,
    },
    /// The package/version has no source list entry to derive a URL from.
    #[error("no source list entry for {package} {version:?}")]
    NoSource {
        package: String,
        version: Option<String>,
    },
    /// Failed to build the download manager.
    #[error("failed to build download manager: {0}")]
    Manager(#[from] BuilderError),
    /// Some packages failed to download.
    #[error("download failed for: {0:?}")]
    DownloadFailed(Vec<String>),
    /// Failed to spawn dpkg.
    #[error("failed to run `{command}`: {err}")]
    Spawn {
        command: String,
        err: std::io::Error,
    },
    /// dpkg exited non-zero.
    #[error("`{command}` failed with {status}")]
    DpkgFailed {
        command: String,
        status: std::process::ExitStatus,
    },
    /// Failed to record which packages were auto-installed.
    #[error("failed to record auto-installed packages: {0}")]
    AutoInstalled(#[from] crate::ExtendedStatesError),
    /// Failed to create the internal async runtime that drives downloads.
    #[error("failed to create async runtime: {0}")]
    FailedCreateAsyncRuntime(String),
    /// Failed to receive the download task's result.
    #[error("failed to receive download result")]
    PumpRecv,
    /// Failed to acquire the apt/dpkg lock (another package manager holds it).
    #[error("failed to acquire lock: {0}")]
    Lock(#[from] LockError),
}

/// The download flow's input, prepared once from a [`DpkgPlan`]: every `.deb`
/// to fetch (with its URL, filename and checksum) plus where each file lands
/// in the archive, so the download and install flows carry the information
/// forward instead of re-deriving it from the index.
pub struct DownloadList {
    /// The `.deb` files to download, in plan order.
    pub entries: Vec<DownloadEntry>,
    /// package → downloaded `.deb` path (what `dpkg --unpack` reads).
    pub deb_paths: HashMap<String, PathBuf>,
}

/// apt's two locks for one operation — the dpkg frontend lock and the
/// archive lock — held until dropped. Acquire with [`Executor::lock`]
/// *before* showing the review plan, then pass to [`Executor::execute`], so
/// no other package manager interferes between the prompt and the commit —
/// like apt, which takes its locks at cache open, before the y/n prompt.
pub struct ExecutorLocks {
    _frontend: LockGuard,
    _archive: LockGuard,
}

/// Runs the two execution flows over a [`DpkgPlan`]: downloading the required
/// `.deb` files with `oma-fetch`, and applying the dpkg operations in apt's
/// list order (removals first, then unpacks in dependency order, then
/// configures).
///
/// The flows are deliberately separate — the download flow
/// ([`Self::download`]) takes the pre-built [`DownloadList`] (see
/// [`build_download_list`]); the install flow ([`Self::apply_dpkg`]) takes
/// the transaction (deriving the dpkg plan from it) and the downloaded deb
/// paths. Neither re-looks-up package information from the index.
pub struct Executor {
    /// `dpkg --root <sysroot>` — the filesystem dpkg operates on.
    sysroot: PathBuf,
    /// Directory the downloaded `.deb` files land in (`dpkg --unpack` reads
    /// them from here).
    archive_dir: PathBuf,
    /// HTTP client handed to `oma-fetch`.
    client: ClientWithMiddleware,
    /// Concurrent download threads.
    threads: usize,
    /// Where `Auto-Installed: 1` records land after a successful install
    /// (relative to `sysroot`; default `var/lib/apt/extended_states`).
    extended_states: PathBuf,
    /// Lazily-created tokio runtime driving `oma-fetch` downloads, so the
    /// public API stays synchronous (like oma-pm): the async work runs on
    /// this runtime, progress is pumped back to the caller's thread.
    async_runtime: OnceCell<Runtime>,
    /// The handle of the runtime actually used — the caller's ambient
    /// runtime when one is active, otherwise the one we created.
    async_handler: OnceCell<Handle>,
}

impl Executor {
    /// Create an executor over `sysroot` (dpkg root), `archive_dir` (where
    /// `.deb`s are downloaded) and the HTTP `client`.
    pub fn new(
        sysroot: impl Into<PathBuf>,
        archive_dir: impl Into<PathBuf>,
        client: ClientWithMiddleware,
    ) -> Self {
        let sysroot = sysroot.into();
        Self {
            extended_states: sysroot.join("var/lib/apt/extended_states"),
            sysroot,
            archive_dir: archive_dir.into(),
            client,
            threads: 4,
            async_runtime: OnceCell::new(),
            async_handler: OnceCell::new(),
        }
    }

    /// Override where auto-installed packages are recorded — the APT
    /// `Dir::State::extended_states` path (default `var/lib/apt/extended_states`
    /// under the sysroot). Useful when the real path is configured
    /// differently.
    pub fn with_extended_states(mut self, path: impl Into<PathBuf>) -> Self {
        self.extended_states = path.into();
        self
    }

    /// Create an executor from apt configuration: the dpkg root from
    /// `RootDir` (default `/`), the download directory from
    /// `Dir::Cache::archives`, and the extended-states path from
    /// `Dir::State::extended_states`.
    pub fn from_config(cfg: &AptConfig, client: ClientWithMiddleware) -> Self {
        let root = cfg.get("RootDir", "");
        let sysroot = if root.is_empty() {
            PathBuf::from("/")
        } else {
            PathBuf::from(root)
        };
        let archive_dir = cfg.get_file("Dir::Cache::archives", "var/cache/apt/archives");
        let extended_states =
            cfg.get_file("Dir::State::extended_states", "var/lib/apt/extended_states");
        Self::new(sysroot, archive_dir, client).with_extended_states(extended_states)
    }

    /// Acquire apt's locks for one operation — the dpkg frontend lock
    /// (`/var/lib/dpkg/lock-frontend`) and the archive lock
    /// (`{archive_dir}/lock`) — held until the returned guard is dropped.
    /// Call this *before* showing the review plan (like apt, which locks at
    /// cache open, before the y/n prompt), then pass the guard to
    /// [`Self::execute`] so the locks span the prompt and the whole install.
    pub fn lock(&self) -> Result<ExecutorLocks, ExecutorError> {
        Ok(ExecutorLocks {
            _frontend: self.lock_frontend()?,
            _archive: self.lock_archive()?,
        })
    }

    /// The tokio runtime handle driving downloads — the caller's ambient
    /// runtime when one is active, otherwise a lazily-created multi-thread
    /// runtime owned by this executor (like oma-pm's
    /// `get_or_init_async_runtime`). Keeps the public API synchronous.
    fn get_or_init_async_runtime(&self) -> Result<&Handle, ExecutorError> {
        self.async_handler
            .get_or_try_init(|| -> Result<Handle, ExecutorError> {
                if let Ok(handle) = tokio::runtime::Handle::try_current() {
                    return Ok(handle);
                }
                let rt = self.async_runtime.get_or_try_init(|| {
                    tokio::runtime::Builder::new_multi_thread()
                        .enable_time()
                        .enable_io()
                        .build()
                        .map_err(|e| ExecutorError::FailedCreateAsyncRuntime(e.to_string()))
                })?;
                Ok(rt.handle().to_owned())
            })
    }

    /// Download flow: fetch the given `.deb` entries with `oma-fetch`.
    ///
    /// Takes the pre-built entries (see [`build_download_list`]) that carry
    /// each version's URL, filename and checksum — this is the download flow
    /// and does not touch the plan or the index.
    ///
    /// `callback` receives each download [`Event`] (progress, retries, ...)
    /// on the calling thread while the download runs on an internal tokio
    /// runtime (see [`Executor::get_or_init_async_runtime`]); this method
    /// blocks until the download finishes. Synchronous, like oma-pm.
    pub fn download<F>(&self, entries: Vec<DownloadEntry>, callback: F) -> Result<(), ExecutorError>
    where
        F: FnMut(Event),
    {
        if entries.is_empty() {
            return Ok(());
        }

        let handle = self.get_or_init_async_runtime()?;
        let (tx, rx) = flume::unbounded();
        let client = self.client.clone();
        let threads = self.threads;
        let mut callback = callback;
        run_task_with_pump(handle, Some(&mut callback), Some(rx), async move {
            let manager = DownloadManager::builder()
                .client(client)
                .download_list(entries.into())
                .maybe_threads(Some(threads))
                .build();

            let summary = manager
                .start_download(move |event| {
                    let tx = tx.clone();
                    async move {
                        let _ = tx.send_async(event).await;
                    }
                })
                .await?;
            if !summary.is_download_success() {
                return Err(ExecutorError::DownloadFailed(summary.failed));
            }
            Ok(())
        })
    }

    /// Install flow: run the dpkg operations in plan order — removals,
    /// unpacks (on the downloaded `.debs`), configures.
    ///
    /// Everything the dpkg run needs is derived from the transaction: the
    /// operation plan ([`Transaction::to_dpkg_plan`]) and the auto-installed
    /// set ([`Transaction::auto_installed_names`]) — the packages the
    /// resolver pulled in as dependencies rather than explicit requests.
    /// `deb_paths` (from [`build_download_list`]) maps each unpacked package
    /// to its downloaded `.deb`; the install flow does not touch the index.
    ///
    /// Once the whole dpkg run succeeds, the auto-installed packages are
    /// recorded with `Auto-Installed: 1` in the extended states file, so
    /// they can be autoremoved later — exactly like apt.
    pub fn apply_dpkg(
        &self,
        txn: Transaction,
        deb_paths: &HashMap<String, PathBuf>,
    ) -> Result<(), ExecutorError> {
        // The plan and the auto-installed set borrow from `txn`, which this
        // function owns — nothing is cloned.
        let plan = txn.to_dpkg_plan();
        for op in plan.ops() {
            match op {
                DpkgOp::Remove { .. } => self.run_dpkg(op, None)?,
                DpkgOp::Unpack { package, .. } => {
                    let deb = deb_paths.get(*package).expect(
                        "build_download_list records a deb path for every unpacked package",
                    );
                    self.run_dpkg(op, Some(deb))?;
                }
                DpkgOp::Configure { .. } => self.run_dpkg(op, None)?,
            }
        }
        // Only after the whole run succeeded: the packages really are
        // installed now, so their auto-installed flag can be recorded.
        let mut ext = crate::AptExtendedStates::from_file(&self.extended_states)?;
        ext.mark_auto(txn.auto_installed_names())?;
        ext.to_file(&self.extended_states)?;
        Ok(())
    }

    /// Full execution flow — the whole install in one call: build the
    /// download list from the transaction, download every `.deb` with
    /// `oma-fetch`, run the dpkg plan, and record which packages were
    /// auto-installed (the resolver's dependencies) once dpkg has succeeded.
    ///
    /// Takes apt's locks first and holds them for the whole operation — the
    /// dpkg frontend lock (`/var/lib/dpkg/lock-frontend`) and the archive
    /// lock (`{archive_dir}/lock`) — so no other package manager runs dpkg
    /// or downloads into the cache concurrently; a second manager gets a
    /// [`ExecutorError::Lock`] instead of racing. Like apt's `DoInstall`,
    /// the archive lock is acquired before any download starts.
    ///
    /// Pass `locks` (from [`Self::lock`], acquired before showing the review
    /// plan) to reuse them instead of taking the locks here, so they span
    /// the confirmation prompt too — like apt, which locks at cache open,
    /// before the y/n prompt. `None` takes the locks right here.
    ///
    /// A convenience that ties [`Self::build_download_list`],
    /// [`Self::download`] and [`Self::apply_dpkg`] together — the dpkg plan
    /// and the auto-installed set are derived from the transaction inside
    /// [`Self::apply_dpkg`]. The granular methods stay available for callers
    /// that need to split the flow — e.g. render the download list before
    /// downloading, or run dpkg as a separate step.
    pub fn execute<F>(
        &self,
        index: &AptDb,
        txn: Transaction,
        locks: Option<&ExecutorLocks>,
        callback: F,
    ) -> Result<(), ExecutorError>
    where
        F: FnMut(Event),
    {
        // apt's locks, held for the whole operation: the dpkg frontend lock
        // (so no other package manager runs dpkg while we work) and the
        // archive lock (so no other manager downloads into this cache). When
        // `locks` is given (acquired before the review prompt via
        // [`Self::lock`]) they are already held; otherwise take them now,
        // like apt's `DoInstall`, before any download starts. The guards live
        // until this function returns, so the locks span the whole operation.
        let _held_locks = match locks {
            Some(_) => None,
            None => Some(self.lock()?),
        };

        let list = self.build_download_list(index, &txn)?;
        self.download(list.entries, callback)?;
        self.apply_dpkg(txn, &list.deb_paths)
    }

    /// Prepare the download flow's input from the *resolution* output — the
    /// chosen versions in a [`Transaction`] — not from the dpkg plan (which
    /// is the install flow's concern). For every install-side change it
    /// derives the `.deb` URL, filename and checksum from the index once, and
    /// records where the file lands (in `self.archive_dir`).
    ///
    /// The result feeds [`Self::download`] (the download flow) and
    /// [`Self::apply_dpkg`] (the install flow), so neither re-derives package
    /// information from the index. Each URL is
    /// built from the package's stored [`IndexSource`] base URL — see
    /// [`download_url`]. The target is the archive dir (via a `partial/`
    /// staging dir so a failed download doesn't leave a torn file).
    pub fn build_download_list(
        &self,
        index: &AptDb,
        txn: &Transaction,
    ) -> Result<DownloadList, ExecutorError> {
        build_download_list_impl(index, &self.archive_dir, txn)
    }

    /// The dpkg frontend lock (`/var/lib/dpkg/lock-frontend` under the
    /// sysroot) — the lock package managers take to serialize dpkg runs.
    /// Held for the whole [`Self::execute`].
    fn lock_frontend(&self) -> Result<LockGuard, ExecutorError> {
        LockGuard::acquire(self.sysroot.join("var/lib/dpkg/lock-frontend"))
            .map_err(ExecutorError::Lock)
    }

    /// The archive lock (`{archive_dir}/lock`) — the lock package managers
    /// take to serialize downloads into the cache. Taken before any download
    /// starts and held for the whole [`Self::execute`] (like apt's
    /// `DoInstall`); the archive directory is created first.
    fn lock_archive(&self) -> Result<LockGuard, ExecutorError> {
        std::fs::create_dir_all(&self.archive_dir).map_err(|e| {
            ExecutorError::Lock(LockError::Failed {
                path: self.archive_dir.display().to_string(),
                err: e.to_string(),
            })
        })?;
        LockGuard::acquire(self.archive_dir.join("lock")).map_err(ExecutorError::Lock)
    }

    /// Spawn one `dpkg` invocation for an operation.
    fn run_dpkg(&self, op: &DpkgOp, deb: Option<&Path>) -> Result<(), ExecutorError> {
        let args = dpkg_args(&self.sysroot, op, deb);
        let command = args.join(" ");
        let status =
            Command::new("dpkg")
                .args(&args)
                .status()
                .map_err(|err| ExecutorError::Spawn {
                    command: command.clone(),
                    err,
                })?;
        if !status.success() {
            return Err(ExecutorError::DpkgFailed { command, status });
        }
        Ok(())
    }
}

/// Run `task` on the runtime's `handle`, pumping download [`Event`]s from
/// `rx` into `callback` on the calling thread while it runs, and blocking
/// until the task's result arrives. Mirrors oma-pm's `run_task_with_pump`,
/// which keeps the public API synchronous: the async work runs on the
/// runtime's threads, progress is rendered on the caller's thread.
fn run_task_with_pump<Fut, T, F>(
    handle: &Handle,
    callback: Option<&mut F>,
    rx: Option<Receiver<Event>>,
    task: Fut,
) -> Result<T, ExecutorError>
where
    Fut: std::future::Future<Output = Result<T, ExecutorError>> + Send + 'static,
    T: Send + 'static,
    F: FnMut(Event),
{
    let (result_tx, result_rx) = flume::bounded(1);
    handle.spawn(async move {
        let res = task.await;
        let _ = result_tx.send(res);
    });

    if let Some(callback) = callback
        && let Some(rx) = rx
    {
        while let Ok(msg) = rx.recv() {
            callback(msg);
        }
    }

    // The channel carries the task's own `Result<T, ExecutorError>`; `?`
    // only propagates a channel failure (the task panicked / was dropped).
    result_rx.recv().map_err(|_| ExecutorError::PumpRecv)?
}

/// Implementation of [`Executor::build_download_list`] — kept as a plain
/// function so the method can borrow `self.archive_dir`.
fn build_download_list_impl(
    index: &AptDb,
    archive_dir: &Path,
    txn: &Transaction,
) -> Result<DownloadList, ExecutorError> {
    let mut entries = Vec::new();
    let mut deb_paths = HashMap::new();
    for change in &txn.changes {
        // Only install-side changes download something — removals have no
        // `to_version`.
        let Some(version) = change.to_version.as_deref() else {
            continue;
        };
        let package = &change.package;
        let entry = lookup_entry(index, package, Some(version))?;
        let Some(filename) = entry.filename.as_deref() else {
            return Err(ExecutorError::EntryNotFound {
                package: package.clone(),
                version: Some(version.to_string()),
            });
        };

        let sources = download_sources(index, package, version, filename)?;
        let deb_name = filename.rsplit('/').next().unwrap_or(filename).to_string();
        let download_entry = DownloadEntry::builder()
            .source(sources)
            .filename(deb_name.clone())
            .allow_resume(true)
            .maybe_hash(
                entry
                    .sha256
                    .as_deref()
                    .and_then(|s| Checksum::from_sha256_str(s).ok()),
            )
            .dir(archive_dir.join("partial"))
            .final_dir(archive_dir.to_path_buf())
            .build();
        deb_paths.insert(package.clone(), archive_dir.join(&deb_name));
        entries.push(download_entry);
    }
    Ok(DownloadList { entries, deb_paths })
}

/// Build a package's download sources from the [`IndexSource`]s its version
/// records: one `{base_url}/{filename}` per recorded source, so a package
/// available from several mirrors/suites gets every URL and oma-fetch can
/// fall back across them.
///
/// The base URLs were resolved from `sources.list` when the database was
/// built, so they already carry the real scheme (http/https) — no
/// `sources.list` re-resolution is needed here.
fn download_sources(
    index: &AptDb,
    package: &str,
    version: &str,
    filename: &str,
) -> Result<Vec<DownloadSource>, ExecutorError> {
    let sources = index
        .get_version(package, version)
        .map(|v| v.sources.to_vec())
        .ok_or_else(|| ExecutorError::NoSource {
            package: package.to_string(),
            version: Some(version.to_string()),
        })?;

    if sources.is_empty() {
        return Err(ExecutorError::NoSource {
            package: package.to_string(),
            version: Some(version.to_string()),
        });
    }

    Ok(sources
        .into_iter()
        .map(|source| DownloadSource {
            url: format!(
                "{}/{}",
                source.base_url.trim_end_matches('/'),
                filename.trim_start_matches('/')
            ),
            source_type: DownloadSourceType::Http,
        })
        .collect())
}

/// Look up a package's index entry, mapping a miss to [`ExecutorError`].
fn lookup_entry(
    index: &AptDb,
    package: &str,
    version: Option<&str>,
) -> Result<crate::PackageEntry, ExecutorError> {
    let version = version.unwrap_or_default();
    index
        .get_version(package, version)
        .map(|c| c.into_owned().entry)
        .ok_or_else(|| ExecutorError::EntryNotFound {
            package: package.to_string(),
            version: (!version.is_empty()).then(|| version.to_string()),
        })
}

/// The full `dpkg` argument vector for one operation, e.g.
/// `["--root", "/", "--remove", "pkg"]`. `deb` is only needed for `Unpack`.
fn dpkg_args(sysroot: &Path, op: &DpkgOp, deb: Option<&Path>) -> Vec<String> {
    let mut args = vec!["--root".to_string(), sysroot.display().to_string()];
    match op {
        DpkgOp::Remove { package, .. } => {
            args.push("--remove".to_string());
            args.push(package.to_string());
        }
        DpkgOp::Unpack { .. } => {
            args.push("--unpack".to_string());
            args.push(
                deb.expect("unpack operation requires a downloaded deb path")
                    .display()
                    .to_string(),
            );
        }
        DpkgOp::Configure { package, .. } => {
            args.push("--configure".to_string());
            args.push(package.to_string());
        }
    }
    args
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        AptDb,
        apt_provider::{Change, ChangeKind, ChangeSet, Transaction},
    };

    /// Build an index whose entries all report the given source.
    fn db_with_source(
        entries: Vec<crate::PackageEntry>,
        source: crate::apt_lists::IndexSource,
    ) -> AptDb {
        let sources = vec![source; entries.len()];
        crate::AptDb::from_entries_with_sources("", entries, sources)
    }

    /// A simple repository source (base URL + suite, `main`/`amd64`).
    fn index_source(base: &str, suite: &str) -> crate::apt_lists::IndexSource {
        crate::apt_lists::IndexSource {
            base_url: base.to_string(),
            suite: suite.to_string(),
            component: Some("main".to_string()),
            arch: Some("amd64".to_string()),
        }
    }

    fn deb_entry(name: &str, version: &str, filename: &str) -> crate::PackageEntry {
        crate::PackageEntry {
            package: name.to_string(),
            version: Some(version.to_string()),
            filename: Some(filename.to_string()),
            sha256: Some("0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef".into()),
            ..crate::PackageEntry {
                package: String::new(),
                version: None,
                architecture: None,
                description: None,
                description_md5: None,
                maintainer: None,
                installed_size: None,
                depends: None,
                pre_depends: None,
                recommends: None,
                suggests: None,
                breaks: None,
                conflicts: None,
                replaces: None,
                provides: None,
                section: None,
                priority: None,
                homepage: None,
                multi_arch: None,
                filename: None,
                size: None,
                sha256: None,
                essential: None,
                protected: None,
            }
        }
    }

    fn changeset_with(packages: &[&str]) -> Transaction {
        let changes: Vec<Change> = packages
            .iter()
            .map(|p| Change {
                kind: ChangeKind::Install,
                package: p.to_string(),
                from_version: None,
                to_version: Some("1.0".into()),
                old_size: None,
                new_size: None,
                download_size: None,
                depends_on: Vec::new(),
                auto_installed: false,
            })
            .collect();
        ChangeSet { changes }.into_transaction()
    }

    #[test]
    fn dpkg_args_match_apt_operations() {
        let sysroot = Path::new("/");
        assert_eq!(
            dpkg_args(
                sysroot,
                &DpkgOp::Remove {
                    package: "old",
                    version: Some("1.0"),
                },
                None
            ),
            vec!["--root", "/", "--remove", "old"]
        );
        assert_eq!(
            dpkg_args(
                sysroot,
                &DpkgOp::Unpack {
                    package: "app",
                    version: Some("1.0"),
                },
                Some(Path::new("/var/cache/app_1.0_amd64.deb"))
            ),
            vec!["--root", "/", "--unpack", "/var/cache/app_1.0_amd64.deb"]
        );
        assert_eq!(
            dpkg_args(
                sysroot,
                &DpkgOp::Configure {
                    package: "app",
                    version: Some("1.0"),
                },
                None
            ),
            vec!["--root", "/", "--configure", "app"]
        );
    }

    #[test]
    fn download_entries_derive_url_from_source() {
        // The stored base URL carries the real scheme from `sources.list`.
        let index = db_with_source(
            vec![deb_entry("app", "1.0", "pool/main/a/app_1.0_amd64.deb")],
            index_source("https://mirrors.example.com/debian", "bookworm"),
        );
        let txn = changeset_with(&["app"]);
        let list =
            build_download_list_impl(&index, Path::new("/var/cache/apt/archives"), &txn).unwrap();
        assert_eq!(list.entries.len(), 1);
        let e = &list.entries[0];
        assert_eq!(
            e.source[0].url,
            "https://mirrors.example.com/debian/pool/main/a/app_1.0_amd64.deb"
        );
        assert_eq!(e.filename, "app_1.0_amd64.deb");
        // The deb path for the install flow is carried, not re-derived.
        assert_eq!(
            list.deb_paths["app"],
            Path::new("/var/cache/apt/archives/app_1.0_amd64.deb")
        );
    }

    #[test]
    fn download_sources_keeps_http_scheme() {
        // The scheme comes from the stored source, not an https assumption.
        let index = db_with_source(
            vec![deb_entry("app", "1.0", "pool/main/a/app_1.0_amd64.deb")],
            index_source("http://mirror.local/debian", "bookworm"),
        );
        let sources =
            download_sources(&index, "app", "1.0", "pool/main/a/app_1.0_amd64.deb").unwrap();
        assert_eq!(sources.len(), 1);
        assert_eq!(
            sources[0].url,
            "http://mirror.local/debian/pool/main/a/app_1.0_amd64.deb"
        );
    }

    #[test]
    fn download_sources_include_every_recorded_source() {
        // A version available from two sources gets both URLs (same version
        // merged via insert_with_source, which grows the source list).
        let mut db = AptDb::from_entries("", Vec::new());
        let entry = deb_entry("app", "1.0", "pool/main/a/app_1.0_amd64.deb");
        db.insert_with_source(
            entry.clone(),
            index_source("https://mirrors.example.com/debian", "bookworm"),
        );
        db.insert_with_source(
            entry,
            index_source("https://mirror.local/debian", "bookworm"),
        );

        let sources = download_sources(&db, "app", "1.0", "pool/main/a/app_1.0_amd64.deb").unwrap();
        assert_eq!(sources.len(), 2);
        assert_eq!(
            sources[0].url,
            "https://mirrors.example.com/debian/pool/main/a/app_1.0_amd64.deb"
        );
        assert_eq!(
            sources[1].url,
            "https://mirror.local/debian/pool/main/a/app_1.0_amd64.deb"
        );
    }

    #[test]
    fn plan_ops_are_downloaded_once() {
        // One install-side package → one Unpack in the plan → one download entry.
        let index = db_with_source(
            vec![deb_entry("app", "1.0", "pool/main/a/app_1.0_amd64.deb")],
            index_source("https://mirrors.example.com/debian", "bookworm"),
        );
        let txn = changeset_with(&["app"]);
        let list = build_download_list_impl(&index, Path::new("/archives"), &txn).unwrap();
        assert_eq!(list.entries.len(), 1);
    }
}
