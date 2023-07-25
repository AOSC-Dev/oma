use std::path::PathBuf;
use std::sync::Arc;
use std::{path::Path, process::exit};

mod args;
mod lang;

use anyhow::Result;

use clap::ArgMatches;
use nix::sys::signal;
use oma_console::{console::style, info, writer::Writer};
use oma_console::{due_to, error};
use oma_pm::apt::{AptArgs, OmaApt};
use oma_refresh::db::OmaRefresh;
use oma_utils::{unlock_oma, OsRelease};
use once_cell::sync::{Lazy, OnceCell};
use std::sync::atomic::{AtomicBool, AtomicI32, Ordering};
use tracing::metadata::LevelFilter;
use tracing_subscriber::{
    fmt, prelude::__tracing_subscriber_SubscriberExt, util::SubscriberInitExt, Layer,
};

use oma_console::console;

use lang::LANGUAGE_LOADER;

static SUBPROCESS: AtomicI32 = AtomicI32::new(-1);
static ALLOWCTRLC: AtomicBool = AtomicBool::new(false);
static LOCKED: AtomicBool = AtomicBool::new(false);
static AILURUS: AtomicBool = AtomicBool::new(false);

fn main() {
    ctrlc::set_handler(single_handler).expect(
        "Oma could not initialize SIGINT handler.\n\nPlease restart your installation environment.",
    );

    let writer = Writer::default();

    let code = match try_main() {
        Ok(exit_code) => exit_code,
        Err(e) => {
            if !e.to_string().is_empty() {
                error!(writer, "{e}");
            }
            e.chain().skip(1).for_each(|cause| {
                due_to!(writer, "{}", cause);
            });
            1
        }
    };

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

    // Init dry-run flag
    let dry_run = if let Some(Ok(Some(true))) = matches
        .subcommand()
        .map(|(_, x)| x.try_get_one::<bool>("dry_run"))
    {
        tracing_subscriber::registry()
            .with(
                fmt::layer()
                    .with_writer(std::io::stdout)
                    .without_time()
                    .with_target(false)
                    .with_filter(if matches.get_flag("debug") {
                        LevelFilter::DEBUG
                    } else {
                        LevelFilter::INFO
                    }),
            )
            .try_init()
            .expect("Can not setup dry_run logger");

        tracing::info!("Running in Dry-run mode");

        true
    } else {
        false
    };

    // Init debug flag
    let debug = if matches.get_flag("debug") {
        tracing_subscriber::registry()
            .with(
                fmt::layer()
                    .with_writer(std::io::stdout)
                    .without_time()
                    .with_target(false)
                    .with_filter(LevelFilter::DEBUG),
            )
            .try_init()?;

        true
    } else {
        false
    };

    tracing::debug!(
        "oma version: {}\n OS: {:#?}",
        env!("CARGO_PKG_VERSION"),
        OsRelease::new()
    );

    let pkgs_getter = |args: &ArgMatches| {
        args.get_many::<String>("packages")
            .map(|x| x.map(|x| x.to_owned()).collect::<Vec<_>>())
    };

    let exit_code = match matches.subcommand() {
        Some(("install", args)) => {
            refresh()?;
            let apt = OmaApt::new()?;
            let pkgs_unparse = pkgs_getter(args).unwrap_or_default();
            let pkgs_unparse = pkgs_unparse.iter().map(|x| x.as_str()).collect::<Vec<_>>();

            let pkgs = apt.select_pkg(pkgs_unparse)?;

            apt.install(pkgs, args.get_flag("reinstall"))?;
            // TODO: network thread
            apt.commit(None, AptArgs::default())?;

            0
        }
        // OmaCommand::Install(
        //     InstallOptions {
        //     packages: pkgs_getter(args),
        //     install_dbg: args.get_flag("install_dbg"),
        //     reinstall: args.get_flag("reinstall"),
        //     no_fixbroken: args.get_flag("no_fix_broken"),
        //     no_refresh: args.get_flag("no_refresh"),
        //     yes: args.get_flag("yes"),
        //     force_yes: args.get_flag("force_yes"),
        //     force_confnew: args.get_flag("force_confnew"),
        //     dry_run: args.get_flag("dry_run"),
        //     dpkg_force_all: args.get_flag("dpkg_force_all"),
        //     install_recommends: args.get_flag("install_recommends"),
        //     install_suggests: args.get_flag("install_suggests"),
        //     no_install_recommends: args.get_flag("no_install_recommends"),
        //     no_install_suggests: args.get_flag("no_install_suggests"),
        // }
        Some(("upgrade", args)) => {
            refresh()?;
            let apt = OmaApt::new()?;
            let pkgs = apt.select_pkg(
                pkgs_getter(args)
                    .unwrap_or_default()
                    .iter()
                    .map(|x| x.as_str())
                    .collect::<Vec<_>>(),
            )?;

            apt.upgrade()?;
            apt.install(pkgs, false)?;
            apt.commit(None, AptArgs::default())?;

            0
        }
        // OmaCommand::Upgrade(UpgradeOptions {
        //     packages: pkgs_getter(args),
        //     yes: args.get_flag("yes"),
        //     force_yes: args.get_flag("force_yes"),
        //     force_confnew: args.get_flag("force_confnew"),
        //     dry_run: args.get_flag("dry_run"),
        //     dpkg_force_all: args.get_flag("dpkg_force_all"),
        //     no_autoremove: args.get_flag("no_autoremove"),
        // }),
        Some(("download", args)) => {
            let apt = OmaApt::new()?;
            let pkgs = apt.select_pkg(
                pkgs_getter(args)
                    .unwrap_or_default()
                    .iter()
                    .map(|x| x.as_str())
                    .collect::<Vec<_>>(),
            )?;

            let path = args
                .get_one::<String>("path")
                .cloned()
                .map(|x| PathBuf::from(&x));

            apt.download(pkgs, None, path.as_deref())?;

            0
        }
        // OmaCommand::Download(Download {
        //     packages: pkgs_getter(args).unwrap(),
        //     path: args.get_one::<String>("path").cloned(),
        //     with_deps: args.get_flag("with_deps"),
        // }),
        Some(("remove", args)) => {
            let apt = OmaApt::new()?;
            let pkgs = apt.select_pkg(
                pkgs_getter(args)
                    .unwrap_or_default()
                    .iter()
                    .map(|x| x.as_str())
                    .collect::<Vec<_>>(),
            )?;

            // TODO: protect
            apt.remove(pkgs, !args.get_flag("keep_config"), true, true)?;

            apt.commit(None, AptArgs::default())?;

            0
        }
        // OmaCommand::Remove(RemoveOptions {
        //     packages: pkgs_getter(args).unwrap(),
        //     yes: args.get_flag("yes"),
        //     force_yes: args.get_flag("force_yes"),
        //     keep_config: args.get_flag("keep_config"),
        //     dry_run: args.get_flag("dry_run"),
        //     no_autoremove: args.get_flag("no_autoremove"),
        // }),
        Some(("refresh", _)) => {
            // TODO: limit
            refresh()?;

            0
        }
        Some(("show", args)) => todo!(),
        // OmaCommand::Show(Show {
        //     packages: pkgs_getter(args).unwrap(),
        //     is_all: args.get_flag("all"),
        // }),
        Some(("search", args)) => todo!(),
        // OmaCommand::Search(Search {
        //     keyword: args
        //         .get_many::<String>("pattern")
        //         .map(|x| x.map(|x| x.to_owned()).collect::<Vec<_>>())
        //         .unwrap(),
        // }),
        Some(("files", args)) => todo!(),
        // OmaCommand::ListFiles(ListFiles {
        //     package: args.get_one::<String>("package").unwrap().to_string(),
        //     bin: args.get_flag("bin"),
        // }),
        Some(("provides", args)) => todo!(),
        // OmaCommand::Provides(Provides {
        //     kw: args.get_one::<String>("pattern").unwrap().to_string(),
        //     bin: args.get_flag("bin"),
        // }),
        Some(("fix-broken", args)) => todo!(),
        // OmaCommand::FixBroken(FixBroken {
        //     dry_run: args.get_flag("dry_run"),
        // }),
        Some(("pick", args)) => todo!(),
        // OmaCommand::Pick(PickOptions {
        //     package: args.get_one::<String>("package").unwrap().to_string(),
        //     no_fixbroken: args.get_flag("no_fix_broken"),
        //     no_refresh: args.get_flag("no_refresh"),
        //     dry_run: args.get_flag("dry_run"),
        // }),
        Some(("mark", args)) => todo!(),
        // OmaCommand::Mark(Mark {
        //     action: match args.get_one::<String>("action").map(|x| x.as_str()) {
        //         Some("hold") => MarkAction::Hold(MarkActionArgs {
        //             pkgs: pkgs_getter(args).unwrap(),
        //         }),
        //         Some("unhold") => MarkAction::Unhold(MarkActionArgs {
        //             pkgs: pkgs_getter(args).unwrap(),
        //         }),
        //         Some("auto") => MarkAction::Auto(MarkActionArgs {
        //             pkgs: pkgs_getter(args).unwrap(),
        //         }),
        //         Some("manual") => MarkAction::Manual(MarkActionArgs {
        //             pkgs: pkgs_getter(args).unwrap(),
        //         }),
        //         _ => unreachable!(),
        //     },
        //     dry_run: args.get_flag("dry_run"),
        // }),
        Some(("command-not-found", args)) => todo!(),
        // OmaCommand::CommandNotFound(CommandNotFound {
        //     kw: args.get_one::<String>("package").unwrap().to_string(),
        // }),
        Some(("list", args)) => todo!(),
        // OmaCommand::List(ListOptions {
        //     packages: pkgs_getter(args),
        //     all: args.get_flag("all"),
        //     installed: args.get_flag("installed"),
        //     upgradable: args.get_flag("upgradable"),
        // }),
        Some(("depends", args)) => todo!(),
        // OmaCommand::Depends(Dep {
        //     pkgs: pkgs_getter(args).unwrap(),
        // }),
        Some(("rdepends", args)) => todo!(),
        // OmaCommand::Rdepends(Dep {
        //     pkgs: pkgs_getter(args).unwrap(),
        // }),
        Some(("clean", _)) => todo!(),
        Some(("history", args)) => todo!(),
        // OmaCommand::History(History {
        //     action: match args.get_one::<String>("action").map(|x| x.as_str()) {
        //         Some("undo") => HistoryAction::Undo(args.get_one::<usize>("index").copied()),
        //         Some("redo") => HistoryAction::Redo(args.get_one::<usize>("index").copied()),
        //         _ => unimplemented!(),
        //     },
        // }),
        #[cfg(feature = "aosc")]
        Some(("topics", v)) => todo!(),
        // OmaCommand::Topics(Topics {
        //     opt_in: v
        //         .get_many::<String>("opt_in")
        //         .map(|x| x.map(|x| x.to_owned()).collect::<Vec<_>>()),
        //     opt_out: v
        //         .get_many::<String>("opt_out")
        //         .map(|x| x.map(|x| x.to_owned()).collect::<Vec<_>>()),
        // }),
        Some(("pkgnames", v)) => todo!(),
        // {
        //     OmaCommand::Pkgnames(v.get_one::<String>("keyword").map(|x| x.to_owned()))
        // }
        _ => unreachable!(),
    };

    Ok(exit_code)
}

fn refresh() -> Result<()> {
    let refresh = OmaRefresh::scan(None)?;
    let tokio = tokio::runtime::Builder::new_multi_thread()
        .enable_io()
        .enable_time()
        .build()?;

    tokio.block_on(async move { refresh.start().await })?;

    Ok(())
}

fn single_handler() {
    let writer = Writer::default();
    // Kill subprocess
    let subprocess_pid = SUBPROCESS.load(Ordering::Relaxed);
    let allow_ctrlc = ALLOWCTRLC.load(Ordering::Relaxed);
    if subprocess_pid > 0 {
        let pid = nix::unistd::Pid::from_raw(subprocess_pid);
        signal::kill(pid, signal::SIGTERM).expect("Failed to kill child process.");
        if !allow_ctrlc {
            info!(writer, "{}", fl!("user-aborted-op"));
        }
    }

    // Dealing with lock
    if LOCKED.load(Ordering::Relaxed) {
        unlock_oma().expect("Failed to unlock instance.");
    }

    // Show cursor before exiting.
    // This is not a big deal so we won't panic on this.
    let _ = writer.show_cursor();

    std::process::exit(2);
}
