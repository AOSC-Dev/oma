use std::ffi::CString;
use std::io;
use std::path::PathBuf;

use std::process::{exit, Command};

mod args;
mod config;
mod error;
mod history;
mod lang;
mod subcommand;
mod table;
mod utils;

#[cfg(feature = "egg")]
mod egg;

use anyhow::anyhow;

use clap::ArgMatches;
use error::OutputError;
use oma_console::writer::{writeln_inner, MessageType, Writer};
use oma_console::{due_to, OmaLayer};
use oma_console::{DEBUG, WRITER};
use oma_utils::oma::{terminal_ring, unlock_oma};
use oma_utils::OsRelease;
use rustix::process::{kill_process, Pid, Signal};
use tracing::{debug, error, info};
use tracing_subscriber::filter::LevelFilter;
use tracing_subscriber::prelude::__tracing_subscriber_SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;
use tracing_subscriber::{fmt, EnvFilter, Layer};

use std::sync::atomic::{AtomicBool, Ordering};

use oma_console::console;
use oma_console::pager::SUBPROCESS;

use crate::config::{Config, GeneralConfig};
use crate::egg::ailurus;
use crate::error::Chain;
use crate::subcommand::topics::TopicArgs;
use crate::subcommand::*;

static ALLOWCTRLC: AtomicBool = AtomicBool::new(false);
static LOCKED: AtomicBool = AtomicBool::new(false);
static AILURUS: AtomicBool = AtomicBool::new(false);

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
    sysroot: String,
}

#[derive(Debug, Default)]
pub struct UpgradeArgs {
    yes: bool,
    force_yes: bool,
    force_confnew: bool,
    dpkg_force_all: bool,
    sysroot: String,
}

#[derive(Debug, Default)]
pub struct RemoveArgs {
    yes: bool,
    remove_config: bool,
    no_autoremove: bool,
    force_yes: bool,
    sysroot: String,
}

fn main() {
    ctrlc::set_handler(single_handler).expect(
        "Oma could not initialize SIGINT handler.\n\nPlease restart your installation environment.",
    );

    let code = match try_main() {
        Ok(exit_code) => exit_code,
        Err(e) => {
            if let Err(e) = display_error(e) {
                eprintln!("Failed to display error, kind: {e}");
            }

            1
        }
    };

    terminal_ring();
    unlock_oma().ok();

    exit(code);
}

fn display_error(e: OutputError) -> io::Result<()> {
    if !e.to_string().is_empty() {
        error!("{e}");

        let cause = Chain::new(&e).skip(1).collect::<Vec<_>>();
        let last_cause = cause.last();

        if let Some(ref last) = last_cause {
            due_to!("{last}");
            let cause_writer = Writer::new(3);
            if cause.len() > 1 {
                for (i, c) in cause.iter().enumerate() {
                    if i == 0 {
                        WRITER.write_prefix(&console::style("TRACE").magenta().to_string())?;
                    } else {
                        WRITER.write_prefix("")?;
                    }

                    let mut res = vec![];
                    writeln_inner(
                        &c.to_string(),
                        "",
                        cause_writer.get_max_len().into(),
                        |t, s| match t {
                            MessageType::Msg => res.push(s.to_owned()),
                            MessageType::Prefix => (),
                        },
                    );
                    for (k, j) in res.iter().enumerate() {
                        if k == 0 {
                            cause_writer.write_prefix(&format!("{i}."))?;
                        } else {
                            WRITER.write_prefix("")?;
                            cause_writer.write_prefix("")?;
                        }
                        print!("{j}");
                    }
                }
            }
        }
    }

    Ok(())
}

fn try_main() -> Result<i32, OutputError> {
    // 使系统错误使用系统 locale 语言输出
    unsafe {
        let s = CString::new("").unwrap();
        libc::setlocale(libc::LC_ALL, s.as_ptr());
    }
    let mut cmd = args::command_builder();
    let matches = cmd.get_matches_mut();

    // Egg
    #[cfg(feature = "egg")]
    {
        let a = matches.get_count("ailurus");
        if a != 0 {
            ailurus()?;
            if a == 3 {
                AILURUS.store(true, Ordering::Relaxed);
            } else {
                return Ok(1);
            }
        }
    }

    let dry_run = matches!(
        matches
            .subcommand()
            .map(|(_, x)| x.try_get_one::<bool>("dry_run")),
        Some(Ok(Some(true)))
    );

    // --no-color option
    if matches.get_flag("no_color")
        || matches!(
            matches.subcommand().map(|(_, x)| x.try_get_one("no_color")),
            Some(Ok(Some(true)))
        )
    {
        std::env::set_var("NO_COLOR", "");
    }

    // Init debug flag
    let debug = if matches.get_flag("debug")
        || matches!(
            matches.subcommand().map(|(_, x)| x.try_get_one("debug")),
            Some(Ok(Some(true)))
        )
        || dry_run
    {
        DEBUG.store(true, Ordering::Relaxed);
        true
    } else {
        false
    };

    if !debug {
        let no_i18n_embd_info: EnvFilter = "i18n_embed=error,info"
            .parse()
            .map_err(|e| anyhow!("{e}"))?;

        tracing_subscriber::registry()
            .with(
                OmaLayer
                    .with_filter(no_i18n_embd_info)
                    .and_then(LevelFilter::INFO),
            )
            .init();
    } else {
        let env_log = EnvFilter::try_from_default_env();

        if let Ok(filter) = env_log {
            tracing_subscriber::registry()
                .with(fmt::layer().with_filter(filter))
                .init();
        } else {
            tracing_subscriber::registry().with(fmt::layer()).init();
        }
    }

    // --no-progress
    let no_progress = matches.get_flag("no_progress")
        || matches!(
            matches
                .subcommand()
                .map(|(_, x)| x.try_get_one("no_progress")),
            Some(Ok(Some(true)))
        )
        || debug;

    let sysroot = matches
        .get_one::<String>("sysroot")
        .unwrap_or(&"/".to_string())
        .to_owned();

    debug!("oma version: {}", env!("CARGO_PKG_VERSION"));
    debug!("OS: {:?}", OsRelease::new());

    // Init config file
    let config = Config::read()?;

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
                sysroot,
            };

            let network_thread = config.network_thread();

            install::execute(
                pkgs_unparse,
                args,
                dry_run,
                network_thread,
                no_progress,
                config.pure_db(),
            )?
        }
        Some(("upgrade", args)) => {
            let pkgs_unparse = pkgs_getter(args).unwrap_or_default();

            let args = UpgradeArgs {
                yes: args.get_flag("yes"),
                force_yes: args.get_flag("force_yes"),
                force_confnew: args.get_flag("force_confnew"),
                dpkg_force_all: args.get_flag("dpkg_force_all"),
                sysroot,
            };

            upgrade::execute(pkgs_unparse, args, dry_run, no_progress, config.pure_db())?
        }
        Some(("download", args)) => {
            let keyword = pkgs_getter(args).unwrap_or_default();
            let keyword = keyword.iter().map(|x| x.as_str()).collect::<Vec<_>>();

            let path = args
                .get_one::<String>("path")
                .cloned()
                .map(|x| PathBuf::from(&x));

            download::execute(keyword, path, dry_run, no_progress)?
        }
        Some((x, args)) if x == "remove" || x == "purge" => {
            let pkgs_unparse = pkgs_getter(args).unwrap();
            let pkgs_unparse = pkgs_unparse.iter().map(|x| x.as_str()).collect::<Vec<_>>();

            let args = RemoveArgs {
                yes: args.get_flag("yes"),
                remove_config: match args.try_get_one::<bool>("remove_config") {
                    Ok(Some(b)) => *b,
                    Ok(None) if x == "purge" => true,
                    Ok(None) | Err(_) => false,
                },
                no_autoremove: args.get_flag("no_autoremove"),
                force_yes: args.get_flag("force_yes"),
                sysroot,
            };

            let protect_essentials = config
                .general
                .as_ref()
                .map(|x| x.protect_essentials)
                .unwrap_or_else(GeneralConfig::default_protect_essentials);

            remove::execute(
                pkgs_unparse,
                args,
                dry_run,
                protect_essentials,
                config.network_thread(),
                no_progress,
            )?
        }
        Some(("refresh", _)) => refresh::execute(no_progress, config.pure_db(), sysroot)?,
        Some(("show", args)) => {
            let pkgs_unparse = pkgs_getter(args).unwrap_or_default();
            let pkgs_unparse = pkgs_unparse.iter().map(|x| x.as_str()).collect::<Vec<_>>();
            let all = args.get_flag("all");

            show::execute(all, pkgs_unparse, sysroot)?
        }
        Some(("search", args)) => {
            let args = args
                .get_many::<String>("pattern")
                .map(|x| x.map(|x| x.to_owned()).collect::<Vec<_>>())
                .unwrap();

            search::execute(&args, no_progress, sysroot)?
        }
        Some((x, args)) if x == "files" || x == "provides" => {
            let arg = if x == "files" { "package" } else { "pattern" };
            let pkg = args.get_one::<String>(arg).unwrap();
            let is_bin = args.get_flag("bin");

            contents_find::execute(x, is_bin, pkg, no_progress, sysroot)?
        }
        Some(("fix-broken", _)) => {
            let network_thread = config.network_thread();
            fix_broken::execute(dry_run, network_thread, no_progress, sysroot)?
        }
        Some(("pick", args)) => {
            let pkg_str = args.get_one::<String>("package").unwrap();
            let network_thread = config.network_thread();

            pick::execute(
                pkg_str,
                args.get_flag("no_refresh"),
                dry_run,
                network_thread,
                no_progress,
                config.pure_db(),
                sysroot,
            )?
        }
        Some(("mark", args)) => {
            let op = args.get_one::<String>("action").unwrap();

            let pkgs = pkgs_getter(args).unwrap();
            let dry_run = args.get_flag("dry_run");

            mark::execute(op, pkgs, dry_run, sysroot)?
        }
        Some(("command-not-found", args)) => {
            command_not_found::execute(args.get_one::<String>("package").unwrap())?
        }
        Some(("list", args)) => {
            let pkgs = pkgs_getter(args).unwrap_or_default();
            let all = args.get_flag("all");
            let installed = args.get_flag("installed");
            let upgradable = args.get_flag("upgradable");

            list::execute(all, installed, upgradable, pkgs, sysroot)?
        }
        Some(("depends", args)) => {
            let pkgs = pkgs_getter(args).unwrap();

            depends::execute(pkgs, sysroot)?
        }
        Some(("rdepends", args)) => {
            let pkgs = pkgs_getter(args).unwrap();

            rdepends::execute(pkgs, sysroot)?
        }
        Some(("clean", _)) => clean::execute(no_progress, sysroot)?,
        Some(("history", _)) => subcommand::history::execute(sysroot)?,
        Some(("undo", _)) => {
            let network_thread = config.network_thread();
            undo::execute(network_thread, no_progress, sysroot)?
        }
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

            let network_thread = config.network_thread();

            let args = TopicArgs {
                opt_in,
                opt_out,
                dry_run,
                network_thread,
                no_progress,
                download_pure_db: config.pure_db(),
                sysroot,
            };

            topics::execute(args)?
        }
        Some(("pkgnames", args)) => {
            let keyword = args.get_one::<String>("keyword").map(|x| x.as_str());

            pkgnames::execute(keyword, sysroot)?
        }
        Some((cmd, args)) => {
            let exe_dir = PathBuf::from("/usr/libexec");
            let plugin = exe_dir.join(format!("oma-{}", cmd));
            if !plugin.is_file() {
                return Err(OutputError::from(anyhow!("Unknown command: `{cmd}'.")));
            }
            info!("Executing applet oma-{cmd}");
            let mut process = &mut Command::new(plugin);
            if let Some(args) = args.get_many::<String>("COMMANDS") {
                process = process.args(args);
            }
            let status = process.status().unwrap().code().unwrap();
            if status != 0 {
                error!("Applet exited with error {status}");
            }

            return Ok(status);
        }
        None => {
            let exit_code = if AILURUS.load(Ordering::Relaxed) {
                0
            } else {
                cmd.print_help().map_err(|e| OutputError {
                    description: "Failed to print help".to_string(),
                    source: Some(Box::new(e)),
                })?;

                1
            };
            return Ok(exit_code);
        }
    };

    Ok(exit_code)
}

fn single_handler() {
    // Kill subprocess
    let subprocess_pid = SUBPROCESS.load(Ordering::Relaxed);
    let allow_ctrlc = ALLOWCTRLC.load(Ordering::Relaxed);

    if subprocess_pid > 0 {
        let pid = Pid::from_raw(subprocess_pid).expect("Pid is empty?");
        kill_process(pid, Signal::Term).expect("Failed to kill child process.");
    }

    // Dealing with lock
    if LOCKED.load(Ordering::Relaxed) {
        unlock_oma().expect("Failed to unlock instance.");
    }

    // Show cursor before exiting.
    // This is not a big deal so we won't panic on this.
    let _ = WRITER.show_cursor();

    if !allow_ctrlc {
        info!("{}", fl!("user-aborted-op"));
    } else {
        std::process::exit(0);
    }

    std::process::exit(2);
}
