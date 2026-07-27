use bon::Builder;
use oma_pm::apt::OmaApt;
use spdlog::warn;
use zbus::zvariant::OwnedFd;

use crate::{
    config::OmaConfig, dbus::dbus_check, error::OutputError, exit_handle::ExitHandle, fl,
    root::root, subcommand::utils::lock_oma,
};

use super::{commit_changes::CommitChanges, refresh::Refresh};

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Guard that holds file descriptors alive for the duration of the operation.
///
/// Dropping this releases the APT lock file and the DBus sleep-inhibit lock,
/// so it must outlive the entire subcommand execution.
struct PrerequisitesGuard {
    /// oma lock file descriptor (rustix::fd::OwnedFd)
    _lock_fd: Option<std::os::fd::OwnedFd>,
    /// DBus sleep-inhibit file descriptor from logind (zbus::zvariant::OwnedFd)
    _inhibit_fd: Option<OwnedFd>,
}

// ---------------------------------------------------------------------------
// Commit options (maps to `CommitChanges` builder fields)
// ---------------------------------------------------------------------------

/// Accumulated configuration for the final `CommitChanges` call.
#[derive(Clone, Debug)]
struct CommitOpts {
    no_fixbroken: bool,
    fix_dpkg_status: bool,
    remove_config: bool,
    autoremove: bool,
    no_clean: bool,
    download_only: bool,
    is_upgrade: bool,
    is_fixbroken: bool,
    is_undo: bool,
    check_tum: bool,
}

impl Default for CommitOpts {
    fn default() -> Self {
        Self {
            no_fixbroken: true,
            fix_dpkg_status: true,
            remove_config: false,
            autoremove: false,
            no_clean: false,
            download_only: false,
            is_upgrade: false,
            is_fixbroken: false,
            is_undo: false,
            check_tum: false,
        }
    }
}

// ---------------------------------------------------------------------------
// AptContext — passed into the subcommand body
// ---------------------------------------------------------------------------

/// Context passed into the subcommand body after all prerequisites are done.
pub struct AptContext<'a> {
    pub config: &'a OmaConfig,
    yes: bool,
    opts: CommitOpts,
}

impl AptContext<'_> {
    /// Build and run `CommitChanges` with the accumulated configuration, then
    /// return the exit handle.
    ///
    /// This is the canonical way to finish a write operation.  Call it at the
    /// end of your subcommand body, passing the mutated `OmaApt`.
    pub fn commit(self, apt: OmaApt) -> Result<ExitHandle, OutputError> {
        CommitChanges::builder()
            .apt(apt)
            .no_fixbroken(self.opts.no_fixbroken)
            .fix_dpkg_status(self.opts.fix_dpkg_status)
            .yes(self.yes)
            .remove_config(self.opts.remove_config)
            .autoremove(self.opts.autoremove)
            .download_only(self.opts.download_only)
            .is_upgrade(self.opts.is_upgrade)
            .is_fixbroken(self.opts.is_fixbroken)
            .is_undo(self.opts.is_undo)
            .check_tum(self.opts.check_tum)
            .config(self.config)
            .no_clean(self.opts.no_clean)
            .build()
            .run()
    }
}

// ---------------------------------------------------------------------------
// Pipeline (bon)
// ---------------------------------------------------------------------------

/// Pipeline builder for the common subcommand execution flow.
///
/// Merges three phases that every "write" subcommand needs:
///
/// 1. **Prerequisites** — root check, oma lock, DBus sleep-inhibit
/// 2. **Refresh** — repository metadata (optional)
/// 3. **Commit** — via [`AptContext::commit`] at the end of the body
///
/// # Example
///
/// ```ignore
/// Pipeline::builder()
///     .config(&config)
///     .yes(true)
///     .need_refresh(true)
///     .no_fixbroken(false)
///     .autoremove(true)
///     .no_clean(false)
///     .build()
///     .run(|ctx| {
///         let apt = …;
///         // … mutate apt …
///         ctx.commit(apt)
///     })
/// ```
#[derive(Builder)]
pub struct Pipeline<'a> {
    config: &'a OmaConfig,

    // -- Prerequisites options ------------------------------------------------

    /// Bypass confirmation prompts.
    #[builder(default)]
    yes: bool,

    /// Refresh repository metadata before the operation.
    #[builder(default)]
    need_refresh: bool,

    /// Always perform privilege escalation (root/lock/dbus), even in dry-run
    /// mode. Needed for subcommands like `fix-broken`.
    #[builder(default)]
    always_escalate: bool,

    // -- Commit changes options ------------------------------------------------

    /// Do not attempt to fix broken APT dependencies.
    #[builder(default = true)]
    no_fixbroken: bool,

    /// Fix dpkg broken status.
    #[builder(default = true)]
    fix_dpkg_status: bool,

    /// Remove configuration files for removed packages (like `apt purge`).
    #[builder(default)]
    remove_config: bool,

    /// Automatically remove unused packages.
    #[builder(default)]
    autoremove: bool,

    /// Do not clean the local package cache after the operation.
    #[builder(default)]
    no_clean: bool,

    /// Only download packages, do not install them.
    #[builder(default)]
    download_only: bool,

    /// Mark this as an upgrade operation.
    #[builder(default)]
    is_upgrade: bool,

    /// Mark this as a fix-broken operation.
    #[builder(default)]
    is_fixbroken: bool,

    /// Mark this as an undo operation.
    #[builder(default)]
    is_undo: bool,

    /// Enable TUM (Topic Update Manager) checking.
    #[builder(default)]
    check_tum: bool,
}

impl Pipeline<'_> {
    /// Run the pipeline and delegate subcommand-specific logic to `body`.
    ///
    /// The `body` closure receives an [`AptContext`] that provides:
    /// - `ctx.config` — the `OmaConfig`
    /// - `ctx.commit(apt)` — convenience method to commit changes
    pub fn run(
        self,
        body: impl FnOnce(AptContext<'_>) -> Result<ExitHandle, OutputError>,
    ) -> Result<ExitHandle, OutputError> {
        let Self {
            config,
            yes,
            need_refresh,
            always_escalate,
            no_fixbroken,
            fix_dpkg_status,
            remove_config,
            autoremove,
            no_clean,
            download_only,
            is_upgrade,
            is_fixbroken,
            is_undo,
            check_tum,
        } = self;

        // ── Phase 1: Prerequisites ────────────────────────────────────
        let need_root = always_escalate || !config.dry_run;

        let _guard: Option<PrerequisitesGuard> = if need_root {
            root()?;
            Some(PrerequisitesGuard {
                _lock_fd: Some(lock_oma(&config.sysroot)?),
                _inhibit_fd: dbus_check(yes, config)?,
            })
        } else {
            None
        };

        // ── Phase 2: Repository refresh (optional) ────────────────────
        if need_refresh {
            Refresh::builder().config(config).build().run()?;
        }

        // ── Phase 3: Automatic mode warning ───────────────────────────
        if yes {
            warn!("{}", fl!("automatic-mode-warn"));
        }

        // ── Phase 4+: Delegate to subcommand ──────────────────────────
        let ctx = AptContext {
            config,
            yes,
            opts: CommitOpts {
                no_fixbroken,
                fix_dpkg_status,
                remove_config,
                autoremove,
                no_clean,
                download_only,
                is_upgrade,
                is_fixbroken,
                is_undo,
                check_tum,
            },
        };
        body(ctx)
    }
}
