use std::path::PathBuf;

use std::process::exit;

mod args;
mod command;
mod error;
mod history;
mod lang;
mod table;

use anyhow::Result;

use clap::ArgMatches;
use nix::sys::signal;
use oma_console::{console::style, info};
use oma_console::{debug, due_to, error, DEBUG, WRITER};
use oma_utils::oma::{terminal_ring, unlock_oma};
use oma_utils::OsRelease;
use once_cell::sync::Lazy;
use time::{UtcOffset, OffsetDateTime};
use time::macros::offset;

use std::sync::atomic::{AtomicBool, Ordering};

use oma_console::console;
use oma_console::pager::SUBPROCESS;

static ALLOWCTRLC: AtomicBool = AtomicBool::new(false);
static LOCKED: AtomicBool = AtomicBool::new(false);
static AILURUS: AtomicBool = AtomicBool::new(false);

static TIME_OFFSET: Lazy<UtcOffset> =
    Lazy::new(|| UtcOffset::local_offset_at(OffsetDateTime::UNIX_EPOCH).unwrap_or(offset!(UTC)));

#[derive(Debug, Default)]
pub struct InstallArgs {
    no_refresh: bool,
    install_dbg: bool,
    reinstall: bool,
    no_fixbroken: bool,
    yes: bool,
    force_yes: bool,
    force_confnew: bool,
    dpkg_force_all: bool,
    install_recommends: bool,
    install_suggests: bool,
    no_install_recommends: bool,
    no_install_suggests: bool,
}

#[derive(Debug, Default)]
pub struct UpgradeArgs {
    yes: bool,
    force_yes: bool,
    force_confnew: bool,
    dpkg_force_all: bool,
}

#[derive(Debug, Default)]
pub struct RemoveArgs {
    yes: bool,
    keep_config: bool,
    no_autoremove: bool,
    force_yes: bool,
}

fn main() {
    // FIXME: 多线程环境下无法获取 time offset
    let _ = &*TIME_OFFSET;

    ctrlc::set_handler(single_handler).expect(
        "Oma could not initialize SIGINT handler.\n\nPlease restart your installation environment.",
    );

    let code = match try_main() {
        Ok(exit_code) => exit_code,
        Err(e) => {
            if !e.to_string().is_empty() {
                error!("{e}");
            }
            e.chain().skip(1).for_each(|cause| {
                due_to!("{cause}");
            });
            1
        }
    };

    terminal_ring();
    unlock_oma().ok();

    exit(code);
}

fn try_main() -> Result<i32> {
    let cmd = args::command_builder();
    let matches = cmd.get_matches();

    // Egg
    if matches.get_count("ailurus") == 3 {
        AILURUS.store(true, Ordering::Relaxed);
    } else if matches.get_count("ailurus") != 0 {
        println!(
            "{} unexpected argument '{}' found\n",
            style("error:").red().bold(),
            style("\x1b[33m--ailurus\x1b[0m").bold()
        );
        println!("{}: oma <COMMAND>\n", style("Usage").bold().underlined());
        println!("For more information, try '{}'.", style("--help").bold());

        return Ok(3);
    }

    let dry_run = matches!(
        matches
            .subcommand()
            .map(|(_, x)| x.try_get_one::<bool>("dry_run")),
        Some(Ok(Some(true)))
    );

    // Init debug flag
    if matches.get_flag("debug") || dry_run {
        DEBUG.store(true, Ordering::Relaxed);
    }

    debug!("oma version: {}", env!("CARGO_PKG_VERSION"));
    debug!("OS: {:?}", OsRelease::new());

    let pkgs_getter = |args: &ArgMatches| {
        args.get_many::<String>("packages")
            .map(|x| x.map(|x| x.to_owned()).collect::<Vec<_>>())
    };

    let exit_code = match matches.subcommand() {
        Some(("install", args)) => {
            let pkgs_unparse = pkgs_getter(args).unwrap_or_default();

            let args = InstallArgs {
                no_refresh: args.get_flag("no_refresh"),
                install_dbg: args.get_flag("install_dbg"),
                reinstall: args.get_flag("reinstall"),
                no_fixbroken: args.get_flag("no_fix_broken"),
                yes: args.get_flag("yes"),
                force_yes: args.get_flag("force_yes"),
                force_confnew: args.get_flag("force_confnew"),
                dpkg_force_all: args.get_flag("dpkg_force_all"),
                install_recommends: args.get_flag("install_recommends"),
                install_suggests: args.get_flag("install_suggests"),
                no_install_recommends: args.get_flag("no_install_recommends"),
                no_install_suggests: args.get_flag("no_install_recommends"),
            };

            command::install(pkgs_unparse, args, dry_run)?
        }
        Some(("upgrade", args)) => {
            let pkgs_unparse = pkgs_getter(args).unwrap_or_default();

            let args = UpgradeArgs {
                yes: args.get_flag("yes"),
                force_yes: args.get_flag("force_yes"),
                force_confnew: args.get_flag("force_confnew"),
                dpkg_force_all: args.get_flag("dpkg_force_all"),
            };

            command::upgrade(pkgs_unparse, args, dry_run)?
        }
        Some(("download", args)) => {
            let keyword = pkgs_getter(args).unwrap_or_default();
            let keyword = keyword.iter().map(|x| x.as_str()).collect::<Vec<_>>();

            let path = args
                .get_one::<String>("path")
                .cloned()
                .map(|x| PathBuf::from(&x));

            command::download(keyword, path, dry_run)?
        }
        Some(("remove", args)) => {
            let pkgs_unparse = pkgs_getter(args).unwrap();
            let pkgs_unparse = pkgs_unparse.iter().map(|x| x.as_str()).collect::<Vec<_>>();

            let args = RemoveArgs {
                yes: args.get_flag("yes"),
                keep_config: args.get_flag("keep_config"),
                no_autoremove: args.get_flag("no_autoremove"),
                force_yes: args.get_flag("force_yes"),
            };

            command::remove(pkgs_unparse, args, dry_run)?
        }
        Some(("refresh", _)) => command::command_refresh()?,
        Some(("show", args)) => {
            let pkgs_unparse = pkgs_getter(args).unwrap_or_default();
            let pkgs_unparse = pkgs_unparse.iter().map(|x| x.as_str()).collect::<Vec<_>>();
            let all = args.get_flag("all");

            command::show(all, pkgs_unparse)?
        }
        Some(("search", args)) => {
            let args = args
                .get_many::<String>("pattern")
                .map(|x| x.map(|x| x.to_owned()).collect::<Vec<_>>())
                .unwrap();

            command::search(&args)?
        }
        Some((x, args)) if x == "files" || x == "provides" => {
            let arg = if x == "files" { "package" } else { "pattern" };
            let pkg = args.get_one::<String>(arg).unwrap().to_string();
            let is_bin = args.get_flag("bin");

            command::find(x, is_bin, pkg)?
        }
        Some(("fix-broken", _)) => command::fix_broken(dry_run)?,
        Some(("pick", args)) => {
            let pkg_str = args.get_one::<String>("package").unwrap().to_string();

            command::pick(pkg_str, args.get_flag("no_refresh"), dry_run)?
        }
        Some(("mark", args)) => {
            let op = args
                .get_one::<String>("action")
                .map(|x| x.as_str())
                .unwrap();
            let pkgs = pkgs_getter(args).unwrap();
            let dry_run = args.get_flag("dry_run");

            command::mark(op, pkgs, dry_run)?
        }
        Some(("command-not-found", args)) => {
            command::command_not_found(args.get_one::<String>("package").unwrap().to_string())?
        }
        Some(("list", args)) => {
            let pkgs = pkgs_getter(args).unwrap_or_default();
            let all = args.get_flag("all");
            let installed = args.get_flag("installed");
            let upgradable = args.get_flag("upgradable");

            command::list(all, installed, upgradable, pkgs)?
        }
        Some(("depends", args)) => {
            let pkgs = pkgs_getter(args).unwrap();

            command::depends(pkgs)?
        }
        Some(("rdepends", args)) => {
            let pkgs = pkgs_getter(args).unwrap();

            command::rdepends(pkgs)?
        }
        Some(("clean", _)) => command::clean()?,
        Some(("history", _)) => command::hisotry()?,
        Some(("undo", _)) => command::undo()?,
        #[cfg(feature = "aosc")]
        Some(("topics", args)) => {
            let opt_in = args
                .get_many::<String>("opt_in")
                .map(|x| x.map(|x| x.to_owned()).collect::<Vec<_>>())
                .unwrap_or_default();

            let opt_out = args
                .get_many::<String>("opt_out")
                .map(|x| x.map(|x| x.to_owned()).collect::<Vec<_>>())
                .unwrap_or_default();

            command::topics(opt_in, opt_out, dry_run)?
        }
        Some(("pkgnames", args)) => {
            let keyword = args.get_one::<String>("keyword").map(|x| x.to_owned());

            command::pkgnames(keyword)?
        }
        _ => unreachable!(),
    };

    Ok(exit_code)
}

fn single_handler() {
    // Kill subprocess
    let subprocess_pid = SUBPROCESS.load(Ordering::Relaxed);
    let allow_ctrlc = ALLOWCTRLC.load(Ordering::Relaxed);
    if subprocess_pid > 0 {
        let pid = nix::unistd::Pid::from_raw(subprocess_pid);
        signal::kill(pid, signal::SIGTERM).expect("Failed to kill child process.");
        if !allow_ctrlc {
            info!("{}", fl!("user-aborted-op"));
        }
    }

    // Dealing with lock
    if LOCKED.load(Ordering::Relaxed) {
        unlock_oma().expect("Failed to unlock instance.");
    }

    // Show cursor before exiting.
    // This is not a big deal so we won't panic on this.
    let _ = WRITER.show_cursor();

    std::process::exit(2);
}
