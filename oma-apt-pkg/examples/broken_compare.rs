//! Compare packages our resolvo solver considers broken (no consistent
//! version set exists for them) with what `apt-cache unmet` reports.
//!
//! The two notions are *not* identical:
//! - resolvo: a package is broken when its `Pre-Depends`/`Depends`/`Recommends`
//!   cannot be satisfied by any *consistent* version set (transitive, version
//!   and conflict aware).
//! - `apt-cache unmet`: per-package, reports deps (incl. `Suggests`) with no
//!   satisfying candidate — not version-consistent, not transitive.
//!
//! So expect differences: `apt-cache unmet` flags `Suggests` (which resolvo
//! ignores) and packages whose only problem is an inconsistent *combination*
//! appear only on our side.
//!
//! The full resolvo scan runs one SAT solve per package and takes a few
//! minutes; use `--limit` for a quick check.
//!
//! usage:
//!   cargo run -p oma-apt-pkg --example broken_compare -- [--limit N] [--no-apt]

use std::collections::{HashMap, HashSet};
use std::process::Command;
use std::time::Instant;

#[path = "broken/scanner.rs"]
mod scanner;

use clap::Parser;
use oma_apt_pkg::{AptConfig, AptDb};
use scanner::BrokenScanner;

#[derive(Parser)]
#[command(about = "Compare broken packages: resolvo solver vs apt-cache unmet")]
struct Args {
    /// Only scan the first N packages with the resolvo solver.
    #[arg(long)]
    limit: Option<usize>,

    /// Use only the fast shallow (direct-dependency) check instead of the
    /// full transitive solver. Roughly apt's speed, but misses transitive
    /// breakage — good for comparing against `apt-cache unmet`.
    #[arg(long)]
    shallow: bool,

    /// With --shallow, flag a package if *any* of its versions is broken by
    /// direct dependencies, matching `apt-cache unmet`'s notion — instead of
    /// only checking the candidate (the version apt would install).
    #[arg(long)]
    all_versions: bool,

    /// Skip running `apt-cache unmet`.
    #[arg(long)]
    no_apt: bool,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse();

    let mut cfg = AptConfig::new();
    cfg.init_defaults()?;
    cfg.load_system()?;

    // Reuse resolver_check's cache so a shared run doesn't rebuild.
    let cache_path = std::env::temp_dir().join("oma-aptdb-resolver.bincode");
    cfg.set("Dir::Cache::oma-aptdb", &cache_path.to_string_lossy());
    let apt_db = AptDb::load_or_build(&cfg)?;

    let mut names: Vec<&str> = apt_db.packages().collect();
    // Sort so `--limit` scans a stable window (packages() order is not
    // deterministic), making runs reproducible and comparable.
    names.sort_unstable();
    if let Some(limit) = args.limit {
        names.truncate(limit);
    }

    // --- our view ---
    let mut broken_versions: HashMap<String, Vec<String>> = HashMap::new();
    let ours: Vec<String> = if args.shallow {
        let mode = if args.all_versions {
            "shallow (all versions)"
        } else {
            "shallow (candidate)"
        };
        println!(
            "=== resolvo solver ({mode}): scanning {} packages ===",
            names.len()
        );
        let start = Instant::now();
        let scanner = BrokenScanner::new(&apt_db);
        let mut ours = Vec::new();
        for (i, name) in names.iter().enumerate() {
            let broken = if args.all_versions {
                let versions = scanner.shallow_broken_versions(name);
                if !versions.is_empty() {
                    broken_versions.insert((*name).to_string(), versions);
                    true
                } else {
                    false
                }
            } else {
                scanner.shallow_is_broken(name)
            };
            if broken {
                ours.push((*name).to_string());
            }
            if (i + 1) % 1000 == 0 || i + 1 == names.len() {
                println!(
                    "  checked {}/{} in {:?} ({} broken so far)",
                    i + 1,
                    names.len(),
                    start.elapsed(),
                    ours.len()
                );
            }
        }
        let elapsed = start.elapsed();
        println!("resolvo ({mode}): {} broken in {elapsed:?}", ours.len());
        ours
    } else {
        println!("=== resolvo solver: scanning {} packages ===", names.len());
        let start = Instant::now();
        let mut scanner = BrokenScanner::new(&apt_db);
        // A successful solve proves its whole closure is not broken, so those
        // packages are skipped instead of being re-solved.
        let mut known_ok: HashSet<String> = HashSet::new();
        let mut solved = 0usize;
        let mut ours = Vec::new();
        for (i, name) in names.iter().enumerate() {
            if known_ok.contains(*name) {
                continue;
            }
            solved += 1;
            if scanner.direct_deps_unsatisfiable(name) {
                ours.push((*name).to_string());
            } else {
                match scanner.check(name) {
                    Ok(closure) => known_ok.extend(closure),
                    Err(()) => ours.push((*name).to_string()),
                }
            }
            if (i + 1) % 500 == 0 || i + 1 == names.len() {
                println!(
                    "  checked {}/{} (solved {solved}) in {:?} ({} broken so far)",
                    i + 1,
                    names.len(),
                    start.elapsed(),
                    ours.len()
                );
            }
        }
        let elapsed = start.elapsed();
        println!(
            "resolvo: {} broken ({} solves) in {elapsed:?}",
            ours.len(),
            solved
        );
        ours
    };

    // --- apt's view: parse `apt-cache unmet` output ---
    let apt = if args.no_apt {
        Vec::new()
    } else {
        println!("\n=== apt-cache unmet ===");
        let output = Command::new("apt-cache")
            .arg("unmet")
            .env("LC_ALL", "C")
            .output()?;
        if !output.status.success() {
            return Err(format!("apt-cache unmet failed: {}", output.status).into());
        }
        let text = String::from_utf8_lossy(&output.stdout);
        parse_apt_unmet(&text)
    };
    // `apt-cache unmet` scans the whole index; restrict it to the packages we
    // actually scanned so the comparison stays fair when `--limit` is used.
    let scanned: HashSet<&str> = names.iter().copied().collect();
    let apt: Vec<String> = apt
        .into_iter()
        .filter(|name| scanned.contains(name.as_str()))
        .collect();
    println!(
        "apt-cache unmet: {} packages (of the {} scanned)",
        apt.len(),
        names.len()
    );

    // --- comparison ---
    let ours_set: HashSet<&str> = ours.iter().map(String::as_str).collect();
    let apt_set: HashSet<&str> = apt.iter().map(String::as_str).collect();

    let both: Vec<&str> = ours
        .iter()
        .map(String::as_str)
        .filter(|n| apt_set.contains(n))
        .collect();
    let only_ours: Vec<&str> = ours
        .iter()
        .map(String::as_str)
        .filter(|n| !apt_set.contains(n))
        .collect();
    let only_apt: Vec<&str> = apt
        .iter()
        .map(String::as_str)
        .filter(|n| !ours_set.contains(n))
        .collect();

    println!("\n=== comparison ===");
    println!("in both:          {}", both.len());
    println!("only resolvo:     {}", only_ours.len());
    println!("only apt-cache:   {}", only_apt.len());

    for (title, list) in [
        ("in both", &both),
        ("only resolvo", &only_ours),
        ("only apt-cache unmet", &only_apt),
    ] {
        if list.is_empty() {
            continue;
        }
        println!("\n{title}:");
        for n in list.iter() {
            match broken_versions.get(*n) {
                Some(versions) => {
                    println!("  {n}  [broken versions: {}]", versions.join(", "))
                }
                None => println!("  {n}"),
            }
        }
    }

    Ok(())
}

/// Parse `apt-cache unmet` output (English locale) into package names.
///
/// Header lines look like: `Package <name> version <ver> has an unmet dep:`
/// followed by indented ` Depends:` / ` Recommends:` / ` Suggests:` lines.
fn parse_apt_unmet(text: &str) -> Vec<String> {
    let mut packages = Vec::new();
    for line in text.lines() {
        let line = line.trim();
        if let Some(name) = line
            .strip_prefix("Package ")
            .and_then(|rest| rest.split(" version ").next())
        {
            packages.push(name.to_string());
        }
    }
    packages.sort();
    packages.dedup();
    packages
}
