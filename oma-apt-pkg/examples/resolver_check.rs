//! End-to-end smoke test for the experimental resolvo-based dependency
//! resolver: load the real APT package lists, resolve a package's dependency
//! closure, and print the resulting transaction (install/upgrade/remove).
//!
//! usage:
//!   resolver_check <package>...
//!   resolver_check <package>... --remove
//!   resolver_check <package>... --compare-apt
//!   resolver_check <package>... --compare-apt-closure
//!   resolver_check --broken [--limit N]
//!
//! examples:
//!   cargo run -p oma-apt-pkg --example resolver_check -- fish
//!   cargo run -p oma-apt-pkg --example resolver_check -- cmatrix --remove
//!   cargo run -p oma-apt-pkg --example resolver_check -- cmatrix --compare-apt
//!   cargo run -p oma-apt-pkg --example resolver_check -- cmatrix --compare-apt-closure
//!   cargo run -p oma-apt-pkg --example resolver_check -- --broken --limit 2000

use std::collections::{HashMap, HashSet};
use std::process::Command;
use std::time::Instant;

#[path = "broken/scanner.rs"]
mod scanner;

use clap::Parser;
use oma_apt_pkg::{
    AptConfig, AptDb, Change, ChangeKind, DpkgState, InstallItem, PackageEntry, ResolveOptions,
    Transaction, TransactionPlanner, resolve_install_order_with,
};
use scanner::BrokenScanner;

#[derive(Parser)]
#[command(about = "Resolve APT dependency closures and plan installs")]
struct Args {
    /// Packages to resolve; prints the install transaction.
    #[arg(required_unless_present = "broken")]
    packages: Vec<String>,

    /// Also run `apt-get -s -V install` on the roots and compare its install
    /// set with ours.
    #[arg(long)]
    compare_apt: bool,

    /// Treat `Suggests` as required (overrides `APT::Install-Suggests`).
    #[arg(long)]
    suggests: bool,

    /// Also compare the *full* dependency closure (ignoring installed state)
    /// with `apt-cache depends --recurse --important`.
    #[arg(long)]
    compare_apt_closure: bool,

    /// Instead of installing, compute and print a *remove* transaction for
    /// the given packages (including installed packages that depend on them).
    #[arg(long)]
    remove: bool,

    /// Scan packages and report the broken ones (one SAT solve each; slow).
    #[arg(long)]
    broken: bool,

    /// With --broken, only scan the first N packages.
    #[arg(long)]
    limit: Option<usize>,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse();

    let mut cfg = AptConfig::new();
    cfg.init_defaults()?;
    cfg.load_system()?;

    let cache_path = std::env::temp_dir().join("oma-aptdb-resolver.bincode");
    cfg.set("Dir::Cache::oma-aptdb", &cache_path.to_string_lossy());
    let apt_db = AptDb::load_or_build(&cfg)?;
    println!("packages: {}", apt_db.packages().count());

    let dpkg_path = cfg.get_file("Dir::State::status", "var/lib/dpkg/status");
    let dpkg = DpkgState::from_file(&dpkg_path)?;

    if args.broken {
        scan_broken(&apt_db, args.limit);
    }

    if !args.packages.is_empty() {
        let roots: Vec<&str> = args.packages.iter().map(String::as_str).collect();
        let mut resolve_options = ResolveOptions::from(&cfg);
        if args.suggests {
            resolve_options.install_suggests = true;
        }
        if args.remove {
            remove_transaction(&apt_db, &dpkg, &roots);
        } else {
            plan(
                &apt_db,
                &roots,
                &dpkg,
                resolve_options,
                args.compare_apt,
                args.compare_apt_closure,
            );
        }
    }

    Ok(())
}

/// Whether a package still needs installing: it is not installed, or it is
/// Compute and print the install transaction for `roots`.
///
/// With `compare_apt`, run `apt-get -s -V install` on `roots` and compare the
/// changed set with ours. With `compare_apt_closure`, compare the full hard
/// closure (ignoring installed state) with `apt-cache depends --recurse
/// --important`.
fn plan(
    db: &AptDb,
    roots: &[&str],
    dpkg: &DpkgState,
    options: ResolveOptions,
    compare_apt: bool,
    compare_apt_closure: bool,
) {
    println!("\n=== install transaction for: {} ===", roots.join(", "));
    // mark_install takes [`PackageEntry`] handles (like apt's MarkInstall);
    // use each root's candidate.
    let targets: Vec<PackageEntry> = roots
        .iter()
        .map(|name| db.get_candidate(name).expect("root must have a candidate"))
        .collect();
    let mut planner = TransactionPlanner::new(db, dpkg, options);
    for target in targets {
        planner.mark_install(target, false);
    }
    let start = Instant::now();
    match planner.resolve() {
        Ok(marks) => {
            let txn = marks.into_transaction();
            let elapsed = start.elapsed();
            print_transaction(&txn, elapsed);

            if compare_apt {
                compare_with_apt(roots, &txn);
            }
            if compare_apt_closure {
                // Full closure ignoring installed state: compare our hard
                // closure (no Recommends) with apt-cache's recursive one.
                match resolve_install_order_with(
                    db,
                    roots,
                    ResolveOptions {
                        install_recommends: false,
                        ..Default::default()
                    },
                ) {
                    Ok(hard) => compare_with_apt_closure(roots, &hard),
                    Err(e) => println!("failed to resolve hard closure: {e}"),
                }
            }
        }
        Err(e) => println!("failed to resolve: {e}"),
    }
}

/// Compute and print the remove transaction for `targets`.
fn remove_transaction(db: &AptDb, dpkg: &DpkgState, targets: &[&str]) {
    println!("\n=== remove transaction for: {} ===", targets.join(", "));
    let targets: Vec<PackageEntry> = targets
        .iter()
        .map(|name| {
            db.get_candidate(name)
                .expect("target must have a candidate")
        })
        .collect();
    let mut planner = TransactionPlanner::new(db, dpkg, ResolveOptions::default());
    for target in targets {
        planner.mark_remove(target);
    }
    let start = Instant::now();
    let txn = planner
        .resolve()
        .expect("removal-only resolution cannot fail")
        .into_transaction();
    let elapsed = start.elapsed();
    print_transaction(&txn, elapsed);
}

/// Print the changes in a transaction grouped by kind.
fn print_transaction(txn: &Transaction, elapsed: std::time::Duration) {
    if txn.is_empty() {
        println!("  no changes in {elapsed:?}");
        return;
    }
    println!("{} changes in {elapsed:?}:", txn.len());
    for (label, kind) in [
        ("Install", ChangeKind::Install),
        ("Upgrade", ChangeKind::Upgrade),
        ("Downgrade", ChangeKind::Downgrade),
        ("Reinstall", ChangeKind::Reinstall),
        ("Remove", ChangeKind::Remove),
    ] {
        let items: Vec<&Change> = txn.changes.iter().filter(|c| c.kind == kind).collect();
        if items.is_empty() {
            continue;
        }
        println!("\n{label}:");
        for c in items {
            match (c.from_version.as_deref(), c.to_version.as_deref()) {
                (Some(from), Some(to)) => println!("  {} {from} -> {to}", c.package),
                (None, Some(to)) => println!("  {} {to}", c.package),
                (Some(from), None) => println!("  {} {from}", c.package),
                (None, None) => println!("  {}", c.package),
            }
        }
    }
}

/// Run `apt-cache depends --recurse --important <roots>` and compare the full
/// dependency closure (ignoring installed state) with ours.
fn compare_with_apt_closure(roots: &[&str], ours: &[InstallItem]) {
    println!(
        "\n=== apt-cache depends --recurse --important: {} ===",
        roots.join(" ")
    );
    let output = match Command::new("apt-cache")
        .args(["depends", "--recurse", "--important"])
        .args(roots)
        .env("LC_ALL", "C")
        .output()
    {
        Ok(output) => output,
        Err(e) => {
            eprintln!("failed to run apt-cache: {e}");
            return;
        }
    };
    if !output.status.success() {
        eprintln!("apt-cache depends failed: {}", output.status);
        return;
    }

    let text = String::from_utf8_lossy(&output.stdout);
    // Closure members are printed at column 0; relation lines are indented.
    let apt: HashSet<String> = text
        .lines()
        .filter(|line| !line.starts_with(char::is_whitespace))
        .map(|line| line.trim().to_string())
        .collect();

    let ours_set: HashSet<&str> = ours.iter().map(|item| item.name.as_str()).collect();
    let both: Vec<&str> = ours
        .iter()
        .map(|item| item.name.as_str())
        .filter(|name| apt.contains(*name))
        .collect();
    let only_ours: Vec<&str> = ours
        .iter()
        .map(|item| item.name.as_str())
        .filter(|name| !apt.contains(*name))
        .collect();
    let only_apt: Vec<&str> = apt
        .iter()
        .filter(|name| !ours_set.contains(name.as_str()))
        .map(String::as_str)
        .collect();

    println!(
        "apt-cache: {} packages; ours (hard closure): {}",
        apt.len(),
        ours.len()
    );
    println!(
        "in both: {}, only ours: {}, only apt: {}",
        both.len(),
        only_ours.len(),
        only_apt.len()
    );
    for (title, list) in [("only ours", &only_ours), ("only apt", &only_apt)] {
        if list.is_empty() {
            continue;
        }
        println!("\n{title}:");
        for name in list.iter().take(15) {
            println!("  {name}");
        }
        if list.len() > 15 {
            println!("  ... and {} more", list.len() - 15);
        }
    }
}

/// Run `apt-get -s -V install <roots>` and compare its install set with ours.
fn compare_with_apt(roots: &[&str], txn: &Transaction) {
    println!("\n=== apt-get -s -V install: {} ===", roots.join(" "));
    let output = match Command::new("apt-get")
        .args(["-s", "-V", "install"])
        .args(roots)
        .env("LC_ALL", "C")
        .output()
    {
        Ok(output) => output,
        Err(e) => {
            eprintln!("failed to run apt-get: {e}");
            return;
        }
    };
    if !output.status.success() {
        eprintln!("apt-get -s failed: {}", output.status);
        return;
    }

    let text = String::from_utf8_lossy(&output.stdout);
    let apt: Vec<(String, String)> = text.lines().filter_map(parse_inst_line).collect();

    // Our side: everything the transaction changes (install/upgrade/downgrade),
    // keyed by name → new version.
    let ours: Vec<(&str, &str)> = txn
        .changes
        .iter()
        .filter(|c| c.kind != ChangeKind::Remove)
        .filter_map(|c| c.to_version.as_deref().map(|v| (c.package.as_str(), v)))
        .collect();
    let ours_map: HashMap<&str, &str> = ours.iter().copied().collect();
    let apt_map: HashMap<&str, &str> = apt
        .iter()
        .map(|(name, version)| (name.as_str(), version.as_str()))
        .collect();

    let both: Vec<&str> = ours
        .iter()
        .map(|(name, _)| *name)
        .filter(|name| apt_map.contains_key(name))
        .collect();
    let only_ours: Vec<&str> = ours
        .iter()
        .map(|(name, _)| *name)
        .filter(|name| !apt_map.contains_key(name))
        .collect();
    let only_apt: Vec<&str> = apt
        .iter()
        .map(|(name, _)| name.as_str())
        .filter(|name| !ours_map.contains_key(name))
        .collect();
    let version_diff: Vec<(&str, &str, &str)> = both
        .iter()
        .filter_map(|name| {
            let ours_ver = ours_map[name];
            let apt_ver = apt_map[name];
            (ours_ver != apt_ver).then_some((*name, ours_ver, apt_ver))
        })
        .collect();

    println!(
        "apt-get: {} packages; ours (to change): {}",
        apt.len(),
        ours.len()
    );
    println!(
        "in both: {}, only ours: {}, only apt: {}, version differs: {}",
        both.len(),
        only_ours.len(),
        only_apt.len(),
        version_diff.len()
    );

    for (title, list) in [("only ours", &only_ours), ("only apt", &only_apt)] {
        if list.is_empty() {
            continue;
        }
        println!("\n{title}:");
        for name in list.iter().take(15) {
            println!("  {name}");
        }
        if list.len() > 15 {
            println!("  ... and {} more", list.len() - 15);
        }
    }
    if !version_diff.is_empty() {
        println!("\nversion differs (ours -> apt):");
        for (name, ours_ver, apt_ver) in version_diff.iter().take(15) {
            println!("  {name} {ours_ver} -> {apt_ver}");
        }
    }
}

/// Parse an apt-get simulation `Inst <name> (<version> ...)` line.
fn parse_inst_line(line: &str) -> Option<(String, String)> {
    let rest = line.trim_start().strip_prefix("Inst ")?;
    let (name, version_part) = rest.split_once(' ')?;
    let version = version_part
        .trim_start_matches('(')
        .split(' ')
        .next()
        .unwrap_or("");
    Some((name.to_string(), version.to_string()))
}

/// Scan packages (optionally limited) and report the broken ones.
fn scan_broken(db: &AptDb, limit: Option<usize>) {
    let mut names: Vec<&str> = db.packages().collect();
    // Sort so `--limit` scans a stable window (packages() order is not
    // deterministic), making runs reproducible and comparable.
    names.sort_unstable();
    if let Some(limit) = limit {
        names.truncate(limit);
    }
    println!(
        "\n=== scanning {} packages for broken candidates ===",
        names.len()
    );
    let start = Instant::now();
    let mut scanner = BrokenScanner::new(db);
    let mut broken = Vec::new();
    for (i, name) in names.iter().enumerate() {
        if scanner.is_broken(name) {
            broken.push(name.to_string());
        }
        // Progress so a long scan doesn't look stuck.
        if (i + 1) % 500 == 0 || i + 1 == names.len() {
            println!(
                "  checked {}/{} in {:?} ({} broken so far)",
                i + 1,
                names.len(),
                start.elapsed(),
                broken.len()
            );
        }
    }
    println!("{} broken in {:?}:", broken.len(), start.elapsed());
    for name in broken.iter().take(20) {
        println!("  {name}");
    }
    if broken.len() > 20 {
        println!("  ... and {} more", broken.len() - 20);
    }
}
