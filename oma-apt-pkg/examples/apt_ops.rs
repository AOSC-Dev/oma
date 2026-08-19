//! End-to-end install / remove / upgrade using the resolvo-based resolver
//! and the dpkg [`Executor`].
//!
//! Without `--execute` the example only prints the dpkg plan (like
//! `apt install --dry-run`); with `--execute` it downloads the `.deb` files
//! with `oma-fetch` (URLs derived from each package's source list entry) and
//! runs `dpkg` (requires root).
//!
//! usage:
//!   apt_ops install <pkg>...
//!   apt_ops remove <pkg>...
//!   apt_ops upgrade <pkg>...
//!   apt_ops fix-broken
//!   apt_ops autoremove
//!   apt_ops <action> [--execute]
//!
//! Package keywords go through the `PackageMatcher`, so `pkg`, `pkg=1.0`,
//! `pkg/stable` and globs (`fish*`) all resolve to concrete package names.
//!
//! With `--execute`, packages the resolver pulled in as dependencies are
//! recorded as `Auto-Installed: 1` in `/var/lib/apt/extended_states` after a
//! successful dpkg run (like apt), so they can be autoremoved later.
//!
//! examples:
//!   cargo run -p oma-apt-pkg --example apt_ops -- install fish
//!   cargo run -p oma-apt-pkg --example apt_ops -- install 'cmatrix*'
//!   cargo run -p oma-apt-pkg --example apt_ops -- remove cmatrix
//!   cargo run -p oma-apt-pkg --example apt_ops -- upgrade oma
//!   cargo run -p oma-apt-pkg --example apt_ops -- fix-broken
//!   cargo run -p oma-apt-pkg --example apt_ops -- autoremove
//!   sudo cargo run -p oma-apt-pkg --example apt_ops -- install fish --execute

use std::collections::HashSet;
use std::io::Write;

use clap::{Parser, Subcommand};
use oma_apt_pkg::{
    AptConfig, AptDb, AptExtendedStates, DpkgOp, DpkgPlan, DpkgState, PackageEntry, PackageMatcher,
    ResolveOptions, Transaction, TransactionPlanner, UpgradeMode,
};
use oma_fetch::Event;
use reqwest_middleware::ClientWithMiddleware;

#[derive(Parser)]
#[command(about = "Install / remove / upgrade packages via oma-apt-pkg")]
struct Args {
    #[command(subcommand)]
    action: Action,

    /// Actually download the `.deb`s (oma-fetch) and run dpkg. Without this
    /// the dpkg plan is only printed.
    #[arg(long)]
    execute: bool,
}

#[derive(Subcommand)]
enum Action {
    /// Install packages and their dependencies.
    Install { packages: Vec<String> },
    /// Remove packages (and installed packages that depend on them).
    Remove { packages: Vec<String> },
    /// Upgrade every outdated installed package to the newest version.
    /// Specified packages are additionally installed (like `apt upgrade pkg`).
    Upgrade {
        #[arg(required = false)]
        packages: Vec<String>,
    },
    /// Fix broken installed packages: packages whose installed version has
    /// unmet hard dependencies are marked for install, so the resolver adds
    /// what's missing (like `apt --fix-broken`).
    FixBroken,
    /// Remove installed packages that were auto-installed and are no longer
    /// needed (like `apt autoremove`).
    Autoremove,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse();

    // Load the repository index and the installed state, like the real oma.
    let mut cfg = AptConfig::new();
    cfg.init_defaults()?;
    cfg.load_system()?;

    let cache_path = std::env::temp_dir().join("oma-aptdb-resolver.bincode");
    cfg.set("Dir::Cache::oma-aptdb", &cache_path.to_string_lossy());
    let apt_db = AptDb::load_or_build(&cfg)?;
    println!("packages: {}", apt_db.packages().count());

    let dpkg_path = cfg.get_file("Dir::State::status", "var/lib/dpkg/status");
    let dpkg = DpkgState::from_file(&dpkg_path)?;

    let mut planner = TransactionPlanner::new(&apt_db, &dpkg, ResolveOptions::from(&cfg));

    // `autoremove` works on the installed state and needs the auto-installed
    // set recorded in the extended states file (like apt's
    // `/var/lib/apt/extended_states`).
    let auto_installed: HashSet<String> = match &args.action {
        Action::Autoremove => {
            let ext_path =
                cfg.get_file("Dir::State::extended_states", "var/lib/apt/extended_states");
            let ext = AptExtendedStates::from_file(&ext_path).unwrap_or_default();
            dpkg.installed_packages()
                .filter(|n| ext.is_auto_installed(n))
                .map(str::to_string)
                .collect()
        }
        _ => HashSet::new(),
    };

    // Resolve the user's package keywords (name, `=version`, `/branch` or
    // glob) to concrete package names via the PackageMatcher. `fix-broken`
    // and `autoremove` take no keywords — they scan the installed state
    // themselves.
    let keywords: Vec<&str> = match &args.action {
        Action::Install { packages }
        | Action::Remove { packages }
        | Action::Upgrade { packages } => packages.iter().map(String::as_str).collect(),
        Action::FixBroken | Action::Autoremove => Vec::new(),
    };
    let matcher = PackageMatcher::new(&apt_db);
    let (matched, no_result) = matcher.match_pkgs_and_versions(keywords)?;
    if !no_result.is_empty() {
        eprintln!("no packages matched: {}", no_result.join(", "));
        std::process::exit(1);
    }
    // Mark phase: record the user's intent — nothing is resolved yet (apt's
    // `MarkInstall` / `MarkDelete`, one package at a time). The matcher
    // resolves each keyword to the version to install: `pkg=1.0` pins the
    // exact version, `pkg/suite` the suite's highest; a bare name or glob
    // pins nothing, so the package's candidate is used (a policy choice,
    // like apt's `GetCandidateVer`). That entry is passed into
    // `mark_install` (apt's `TryToInstall` → `MarkInstall`).
    let targets: Vec<PackageEntry> = matched
        .iter()
        .map(|group| {
            // Each group is non-empty (the matcher drops empty matches); the
            // first version carries the package name. Install the group's
            // highest version, or the package candidate for a bare name.
            let name = group[0].entry.package.clone();
            let version = group
                .iter()
                .max_by_key(|v| v.parsed_version())
                .and_then(|v| v.entry.version.clone());
            match version {
                Some(version) => PackageEntry {
                    package: name,
                    version: Some(version),
                    ..PackageEntry::default()
                },
                None => apt_db
                    .get_candidate(&name)
                    .expect("matched package must have a candidate"),
            }
        })
        .collect();
    let mut names: Vec<String> = targets.iter().map(|e| e.package.clone()).collect();
    let action_name = match &args.action {
        Action::Install { .. } => {
            for target in targets {
                planner.mark_install(target, false);
            }
            "install"
        }
        Action::Remove { .. } => {
            for target in targets {
                planner.mark_remove(target);
            }
            "remove"
        }
        Action::Upgrade { .. } => {
            // `upgrade` always upgrades every outdated installed package;
            // any specified packages are *additionally* installed on top —
            // the same as `apt upgrade pkg`.
            planner.upgrade(UpgradeMode::FullUpgrade);
            for target in targets {
                planner.mark_install(target, false);
            }
            "upgrade"
        }
        Action::FixBroken => {
            // `fix-broken` scans the installed state itself — the keyword
            // list above is empty and `targets` is unused.
            names = planner.fix_broken()?;
            "fix-broken"
        }
        Action::Autoremove => {
            // `autoremove` scans the installed state itself — the keyword
            // list above is empty and `targets` is unused. The
            // `APT::NeverAutoRemove` patterns (kernel/firmware …) protect
            // matching packages from removal.
            let never = cfg.keys_under("APT::NeverAutoRemove");
            let never: Vec<String> = never.map(str::to_string).collect();
            names = planner.autoremove(&auto_installed, &never);
            "autoremove"
        }
    };

    let header = match &args.action {
        Action::Upgrade { packages } if packages.is_empty() => {
            "upgrade all installed packages".to_string()
        }
        _ => format!("{action_name}: {}", names.join(", ")),
    };

    // Resolve phase: the single entry point that starts dependency
    // resolution (apt's `pkgProblemResolver::Resolve`).
    println!("=== {header} ===");
    let changeset = planner.resolve()?;
    // Order phase: the single place the change set is ordered (apt's
    // `pkgOrderList`); the dpkg plan and the download list both derive from
    // the resulting transaction.
    let txn = changeset.into_transaction();
    let plan = txn.to_dpkg_plan();

    print_dpkg_plan(&plan);

    if args.execute {
        execute(&apt_db, txn, &cfg)?;
    }

    Ok(())
}

/// Print the dpkg operation list the way `apt install --dry-run` does
/// (`Remv`, then `Inst`, then `Conf`).
fn print_dpkg_plan(plan: &DpkgPlan) {
    if plan.is_empty() {
        println!("no changes");
        return;
    }
    println!("The following packages will be processed by dpkg:");
    for op in plan.ops() {
        match op {
            DpkgOp::Remove { package, version } => {
                println!("Remv {package} ({})", version.unwrap_or("?"));
            }
            DpkgOp::Unpack { package, version } => {
                println!("Inst {package} ({})", version.unwrap_or("?"));
            }
            DpkgOp::Configure { package, version } => {
                println!("Conf {package} ({})", version.unwrap_or("?"));
            }
        }
    }
}

/// Download every package with `oma-fetch`, then run the dpkg operations.
///
/// Downloads into `/var/cache/apt/archives` and operates on the real system
/// (`sysroot = /`), so this must run as root. This is the library's
/// single-call flow ([`Executor::execute`]): the download list is prepared
/// from the *resolution* output — the ordered changes in the [`Transaction`]
/// — with URLs built from each package's stored source base URL (resolved
/// from `sources.list` when the database was built, real scheme included),
/// then every `.deb` is fetched and the dpkg plan applied, recording which
/// packages were auto-installed once dpkg has succeeded.
fn execute(
    index: &AptDb,
    txn: Transaction,
    cfg: &AptConfig,
) -> Result<(), Box<dyn std::error::Error>> {
    let client = build_client()?;

    // Committing the transaction builds an executor from the apt
    // configuration (sysroot from RootDir, download dir from
    // Dir::Cache::archives) and blocks until dpkg has finished, pumping
    // download progress into our callback on this thread.
    println!("\nexecuting (download + dpkg):");
    let mut progress = DownloadProgress::default();
    txn.commit(index, cfg, client, |event| {
        handle_progress(&mut progress, event);
    })?;

    Ok(())
}

/// Tracks overall download progress from oma-fetch events.
#[derive(Default)]
struct DownloadProgress {
    downloaded: u64,
    total: u64,
    last_pct: u64,
}

/// Fold a download [`Event`] into the progress state and refresh the terminal
/// line whenever the overall percentage moves to a new integer value.
fn handle_progress(state: &mut DownloadProgress, event: Event) {
    let p = state;
    match event {
        Event::NewGlobalProgressBar(total) => p.total = total,
        Event::GlobalProgressAdd(n) => p.downloaded += n,
        Event::GlobalProgressSub(n) => p.downloaded = p.downloaded.saturating_sub(n),
        Event::AllDone => {
            println!();
            return;
        }
        _ => {}
    }

    if p.total > 0 {
        let pct = p
            .downloaded
            .checked_mul(100)
            .and_then(|v| v.checked_div(p.total))
            .unwrap_or(100);
        if pct != p.last_pct {
            p.last_pct = pct;
            print!("\r{}/{} ({}%)", p.downloaded, p.total, pct);
            let _ = std::io::stdout().flush();
        }
    }
}

/// Build an HTTPS-capable `reqwest` client for `oma-fetch`.
fn build_client() -> Result<ClientWithMiddleware, Box<dyn std::error::Error>> {
    rustls::crypto::ring::default_provider()
        .install_default()
        .map_err(|e| format!("failed to install rustls provider: {e:?}"))?;
    let client = oma_fetch::reqwest::ClientBuilder::new()
        .user_agent("oma-apt-pkg-example")
        .build()?;
    Ok(client.into())
}
