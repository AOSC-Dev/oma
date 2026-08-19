//! `oma-edsp` — an EDSP external dependency solver for apt.
//!
//! apt can delegate dependency resolution to an external binary (see the
//! apt source, `doc/external-dependency-solver-protocol.md`): the solver is
//! invoked with the request and the whole package universe on stdin and
//! answers with a solution on stdout. This binary implements that protocol
//! on top of `oma-apt-pkg`'s resolvo-based resolver.
//!
//! To let apt use it:
//!
//! 1. install (or symlink) this binary into `Dir::Bin::solvers` (default
//!    `/usr/libexec/apt/solvers/`), e.g. as `oma-edsp`
//! 2. select it with `-o APT::Solver=oma-edsp` on the command line, or set
//!    `APT::Solver "oma-edsp";` in an apt config file
//!
//! Then e.g. `apt-get -o APT::Solver=oma-edsp -s install fish` resolves
//! with oma-apt-pkg instead of apt's internal solver.
//!
//! The solver reads the system apt config (`APT::Install-Recommends` /
//! `APT::Install-Suggests` / `APT::NeverAutoRemove`) just like apt's internal
//! solver (solver3) does.

use std::io::Read;
use std::time::Instant;

use oma_apt_pkg::AptConfig;
use oma_apt_pkg::ResolveOptions;
use oma_apt_pkg::edsp::{self, EdspError};

/// Print elapsed time to stderr when `OMA_EDSP_TIMING=1` (diagnostic).
fn timed(profile: bool, name: &str, start: Instant) {
    if profile {
        eprintln!(
            "oma-edsp[{name}] {:.1}ms",
            start.elapsed().as_secs_f64() * 1000.0
        );
    }
}

fn main() {
    let profile = std::env::var_os("OMA_EDSP_TIMING").is_some();

    let mut input = String::new();
    if let Err(e) = std::io::stdin().read_to_string(&mut input) {
        eprintln!("oma-edsp: failed to read stdin: {e}");
        std::process::exit(1);
    }

    let mut out = std::io::stdout().lock();
    let t = Instant::now();
    let result = solve(&input, profile);
    timed(profile, "total", t);
    match result {
        Ok(solution) => {
            let t = Instant::now();
            if let Err(e) = edsp::write_solution(&mut out, &solution) {
                eprintln!("oma-edsp: failed to write solution: {e}");
                std::process::exit(1);
            }
            timed(profile, "write", t);
        }
        Err(e) => {
            // Send a protocol error stanza (apt shows `Message:`) and exit
            // non-zero, so apt reports the failure either way.
            let _ = edsp::write_error(&mut out, "oma-edsp", &e.to_string());
            eprintln!("oma-edsp: {e}");
            std::process::exit(1);
        }
    }
}

fn solve(input: &str, profile: bool) -> Result<edsp::EdspSolution, EdspError> {
    let t = Instant::now();
    let parsed = edsp::parse_input(input)?;
    timed(profile, "parse", t);

    // Read the same apt config the internal solver (solver3) would.
    // Best-effort — a missing/unreadable config just means apt's defaults.
    let t = Instant::now();
    let mut cfg = AptConfig::new();
    let _ = cfg.init_defaults();
    let _ = cfg.load_system();
    timed(profile, "config", t);

    // The resolver itself takes plain values — resolve the apt config here.
    let options = ResolveOptions::from(&cfg);
    let never_auto_remove: Vec<String> = cfg
        .keys_under("APT::NeverAutoRemove")
        .map(str::to_string)
        .collect();

    edsp::solve_with(&parsed, options, &never_auto_remove)
}
