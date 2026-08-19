//! Scan the available package set for "broken" packages — those whose
//! dependencies cannot be satisfied by any consistent version set (the
//! resolvo solver's notion, not apt's).
//!
//! This is the diagnostic tool that used to ship as `oma_apt_pkg`'s
//! `BrokenScanner` / `find_broken` public API; it now lives here as an
//! example so the library keeps a minimal public surface. The full check
//! runs one solver pass per package (the library's public `solve_packages`
//! entry rebuilds the intern pool per call), so a whole-archive scan is
//! slow — use `--limit` for a quick check, or `--shallow` for an apt-speed
//! approximation that only looks at direct dependencies.
//!
//! usage:
//!   cargo run -p oma-apt-pkg --example broken -- [--limit N] [--shallow] [--all-versions] [--name PKG]

mod scanner;

use std::collections::HashSet;
use std::time::Instant;

use clap::Parser;
use oma_apt_pkg::{AptConfig, AptDb};

use scanner::BrokenScanner;

#[derive(Parser)]
#[command(about = "Report packages whose dependencies cannot be satisfied")]
struct Args {
    /// Only scan the first N packages (sorted by name).
    #[arg(long)]
    limit: Option<usize>,

    /// Use the fast shallow (direct-dependency) check instead of the full
    /// solver. Roughly apt's speed, but misses transitive breakage.
    #[arg(long)]
    shallow: bool,

    /// With --shallow, flag a package if *any* of its versions is broken by
    /// direct dependencies, instead of only the candidate (the version apt
    /// would install).
    #[arg(long)]
    all_versions: bool,

    /// Check a single package by name and print the verdict.
    #[arg(long)]
    name: Option<String>,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse();

    let mut cfg = AptConfig::new();
    cfg.init_defaults()?;
    cfg.load_system()?;

    // Reuse resolver_check/broken_compare's cache so a shared run doesn't
    // rebuild it.
    let cache_path = std::env::temp_dir().join("oma-aptdb-resolver.bincode");
    cfg.set("Dir::Cache::oma-aptdb", &cache_path.to_string_lossy());
    let apt_db = AptDb::load_or_build(&cfg)?;

    if let Some(name) = &args.name {
        if !apt_db.has_package(name) {
            println!("{name}: not in the index");
            return Ok(());
        }
        let mut scanner = BrokenScanner::new(&apt_db);
        if scanner.is_broken(name) {
            println!("{name}: BROKEN (no consistent version set exists)");
        } else {
            println!("{name}: ok");
        }
        return Ok(());
    }

    let mut names: Vec<&str> = apt_db.packages().collect();
    // Sort so `--limit` scans a stable window (packages() order is not
    // deterministic), making runs reproducible and comparable.
    names.sort_unstable();
    if let Some(limit) = args.limit {
        names.truncate(limit);
    }

    let mode = if args.shallow {
        if args.all_versions {
            "shallow (all versions)"
        } else {
            "shallow (candidate)"
        }
    } else {
        "full solver"
    };
    println!("=== {mode}: scanning {} packages ===", names.len());

    let start = Instant::now();
    let mut scanner = BrokenScanner::new(&apt_db);
    let mut broken = Vec::new();
    // A successful solve proves its whole closure is not broken, so those
    // packages are skipped instead of being re-solved.
    let mut known_ok: HashSet<String> = HashSet::new();
    let mut solved = 0usize;
    for (i, name) in names.iter().enumerate() {
        let is_broken = if args.shallow {
            if args.all_versions {
                scanner.shallow_is_broken_any_version(name)
            } else {
                scanner.shallow_is_broken(name)
            }
        } else {
            if known_ok.contains(*name) {
                continue;
            }
            solved += 1;
            if scanner.direct_deps_unsatisfiable(name) {
                true
            } else {
                match scanner.check(name) {
                    Ok(closure) => {
                        known_ok.extend(closure);
                        false
                    }
                    Err(()) => true,
                }
            }
        };
        if is_broken {
            broken.push(name.to_string());
        }
        if (i + 1) % 1000 == 0 || i + 1 == names.len() {
            println!(
                "  checked {}/{} in {:?} ({} broken so far)",
                i + 1,
                names.len(),
                start.elapsed(),
                broken.len()
            );
        }
    }
    println!(
        "{} broken in {:?} ({} solves)",
        broken.len(),
        start.elapsed(),
        solved
    );
    for name in broken.iter().take(20) {
        println!("  {name}");
    }
    if broken.len() > 20 {
        println!("  ... and {} more", broken.len() - 20);
    }

    Ok(())
}
