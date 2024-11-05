use std::env;
use std::ffi::CString;
use std::io::{self, stderr, stdin, IsTerminal};
use std::path::PathBuf;

use std::process::{exit, Command};
use std::sync::{LazyLock, OnceLock};
use std::time::Duration;

mod args;
mod config;
mod error;
mod lang;
mod pb;
mod subcommand;
mod table;
mod tui;
mod utils;

#[cfg(feature = "egg")]
mod egg;

use anyhow::anyhow;

use clap::ArgMatches;
use error::OutputError;
use i18n_embed::DesktopLanguageRequester;
use lang::LANGUAGE_LOADER;
use list::ListFlags;
use oma_console::print::{termbg, OmaColorFormat};
use oma_console::writer::{writeln_inner, MessageType, Writer};
use oma_console::WRITER;
use oma_console::{due_to, OmaLayer};
use oma_utils::dbus::{create_dbus_connection, get_another_oma_status, OmaDbusError};
use oma_utils::oma::{terminal_ring, unlock_oma};
use oma_utils::OsRelease;
use reqwest::Client;
use rustix::stdio::stdout;
use subcommand::utils::LockError;
use tokio::runtime::Runtime;
use tracing::{debug, error, info, warn};
use tracing_subscriber::filter::LevelFilter;
use tracing_subscriber::prelude::__tracing_subscriber_SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;
use tracing_subscriber::{fmt, EnvFilter, Layer};
use tui::TuiArgs;
use utils::is_ssh_from_loginctl;

use std::sync::atomic::{AtomicBool, Ordering};

use oma_console::console;

use crate::config::{Config, GeneralConfig};
#[cfg(feature = "egg")]
use crate::egg::ailurus;
use crate::error::Chain;
#[cfg(feature = "aosc")]
use crate::subcommand::topics::TopicArgs;
use crate::subcommand::*;

static ALLOWCTRLC: AtomicBool = AtomicBool::new(false);
static LOCKED: AtomicBool = AtomicBool::new(false);
static DEBUG: AtomicBool = AtomicBool::new(false);
static SPAWN_NEW_OMA: AtomicBool = AtomicBool::new(false);
static APP_USER_AGENT: &str = concat!("oma/", env!("CARGO_PKG_VERSION"));
static COLOR_FORMATTER: OnceLock<OmaColorFormat> = OnceLock::new();
static RT: LazyLock<Runtime> = LazyLock::new(|| {
    tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .expect("Failed to init async runtime")
});
static HTTP_CLIENT: LazyLock<Client> = LazyLock::new(|| {
    Client::builder()
        .user_agent(APP_USER_AGENT)
        .build()
        .unwrap()
});

#[derive(Debug, Default)]
pub struct InstallArgs {
    no_refresh: bool,
    install_dbg: bool,
    reinstall: bool,
    no_fixbroken: bool,
    yes: bool,
    force_yes: bool,
    force_confnew: bool,
    install_recommends: bool,
    install_suggests: bool,
    no_install_recommends: bool,
    no_install_suggests: bool,
    sysroot: String,
    no_refresh_topic: bool,
    force_unsafe_io: bool,
}

#[derive(Debug, Default)]
pub struct UpgradeArgs {
    yes: bool,
    force_yes: bool,
    force_confnew: bool,
    sysroot: String,
    no_refresh_topcs: bool,
    autoremove: bool,
    force_unsafe_io: bool,
}

#[derive(Debug, Default)]
pub struct RemoveArgs {
    yes: bool,
    remove_config: bool,
    no_autoremove: bool,
    force_yes: bool,
    sysroot: String,
    fix_broken: bool,
    force_unsafe_io: bool,
}

pub struct OmaArgs {
    dry_run: bool,
    network_thread: usize,
    no_progress: bool,
    no_check_dbus: bool,
    protect_essentials: bool,
    another_apt_options: Vec<String>,
}

fn main() {
    let localizer = crate::lang::localizer();
    let requested_languages = DesktopLanguageRequester::requested_languages();

    if let Err(error) = localizer.select(&requested_languages) {
        eprintln!("Error while loading languages for library_fluent {}", error);
    }

    // Windows Terminal doesn't support bidirectional (BiDi) text, and renders the isolate characters incorrectly.
    // This is a temporary workaround for https://github.com/microsoft/terminal/issues/16574
    // TODO: this might break BiDi text, though we don't support any writing system depends on that.
    LANGUAGE_LOADER.set_use_isolating(false);

    ctrlc::set_handler(single_handler).expect(
        "Oma could not initialize SIGINT handler.\n\nPlease restart your installation environment.",
    );

    let mut cmd = args::command_builder();
    let matches = cmd.get_matches_mut();

    let dry_run = matches!(
        matches
            .subcommand()
            .map(|(_, x)| x.try_get_one::<bool>("dry_run")),
        Some(Ok(Some(true)))
    );

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

    // --no-progress
    let no_progress = matches.get_flag("no_progress")
        || matches!(
            matches
                .subcommand()
                .map(|(_, x)| x.try_get_one("no_progress")),
            Some(Ok(Some(true)))
        )
        || debug;

    #[cfg(feature = "tokio-console")]
    console_subscriber::init();

    #[cfg(not(feature = "tokio-console"))]
    if !debug {
        let no_i18n_embd_info: EnvFilter = "i18n_embed=off,info".parse().unwrap();

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
                .with(
                    fmt::layer()
                        .event_format(
                            tracing_subscriber::fmt::format()
                                .with_file(true)
                                .with_line_number(true),
                        )
                        .with_filter(filter),
                )
                .init();
        } else {
            let debug_filter: EnvFilter = "hyper=off,rustls=off,debug".parse().unwrap();
            tracing_subscriber::registry()
                .with(
                    fmt::layer()
                        .event_format(
                            tracing_subscriber::fmt::format()
                                .with_file(true)
                                .with_line_number(true),
                        )
                        .with_filter(debug_filter),
                )
                .init();
        }
    }

    let code = match run_subcmd(matches, dry_run, no_progress) {
        Ok(exit_code) => {
            unlock_oma().ok();
            exit_code
        }
        Err(e) => {
            match display_error_and_can_unlock(e) {
                Ok(true) => {
                    unlock_oma().ok();
                }
                Ok(false) => {}
                Err(e) => {
                    eprintln!("Failed to display error, kind: {e}");
                }
            }

            1
        }
    };

    terminal_ring();

    exit(code);
}

fn run_subcmd(matches: ArgMatches, dry_run: bool, no_progress: bool) -> Result<i32, OutputError> {
    // Egg
    #[cfg(feature = "egg")]
    {
        let a = matches.get_count("ailurus");
        if a != 0 {
            ailurus()?;
            if a == 3 {
                AILURUS.store(true, Ordering::Relaxed);
            } else {
                return Ok(3);
            }
        }
    }

    // 使系统错误使用系统 locale 语言输出
    unsafe {
        let s = CString::new("").unwrap();
        libc::setlocale(libc::LC_ALL, s.as_ptr());
    }

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

    let mut follow_term_color = config.follow_terminal_color()
        || matches.get_flag("follow_terminal_color")
        || matches!(
            matches
                .subcommand()
                .map(|(_, x)| x.try_get_one("follow_terminal_color")),
            Some(Ok(Some(true)))
        );

    let mut no_color = false;

    // --no-color option
    if matches.get_flag("no_color")
        || matches!(
            matches.subcommand().map(|(_, x)| x.try_get_one("no_color")),
            Some(Ok(Some(true)))
        )
    {
        std::env::set_var("NO_COLOR", "");
        no_color = true;
    }

    COLOR_FORMATTER.get_or_init(|| {
        // FIXME: Marking latency limits for oma's terminal color queries (via
        // termbg). On slower terminals - i.e., SSH and unaccelerated
        // graphical environments, any colored interfaces in oma may return a
        // terminal color query string in the returned shell, confusing users.
        //
        //   (ssh)root@LoongUnion1 [ ~ ] ? 11;rgb:2323/2626/2727
        //
        // Following advice from termbg here. Add latency limits to avoid this
        // strange output on slower terminals.
        //
        // For further investigation, we have some remaining questions:
        //
        // 1. Why 100ms? We see that the termbg-based procs project using the
        //    same latency limit to workaround the aforementioned issue.
        //    It should be noted that this is nothing more than a "magic
        //    number" that we have tested to work.
        // 2. The true cause or reproducing conditions for this issue is not
        //    yet clear, we found the same issue on a slower machine (Loongson
        //    3B4000) in a nearby datacenter (~50ms) with a faster one
        //    (Loongson 3C5000), which does not exhibit the issue; as well as
        //    on a faster machine (AMD EPYC 7H12) with high latency (~450ms).
        //
        // Ref: https://github.com/dalance/procs/issues/221
        // Ref: https://github.com/dalance/procs/commit/83305be6fb431695a070524328b66c7107ce98f3
        let timeout = Duration::from_millis(100);

        if !stdout().is_terminal() || !stderr().is_terminal() || !stdin().is_terminal() || no_color
        {
            follow_term_color = true;
        } else if env::var("SSH_CONNECTION").is_ok() || is_ssh_from_loginctl()  {
            debug!(
                "You are running oma in an SSH session, using default terminal colors to avoid latency."
            );
            follow_term_color = true;
        } else if env::var("TERM").is_err() || termbg::terminal() != termbg::Terminal::XtermCompatible {
            debug!("Your terminal is: {:?}", termbg::terminal());
            debug!(
                "Unknown or unsupported terminal ($TERM is empty or unsupported) detected, using default terminal colors to avoid latency."
            );
            follow_term_color = true;
        } else if let Ok(latency) = termbg::latency(Duration::from_millis(1000)) {
            debug!("latency: {:?}", latency);
            if latency * 2 > timeout {
                debug!(
                    "Terminal latency is too long, falling back to default terminal colors, latency: {:?}.",
                    latency
                );
                follow_term_color = true;
            }
        } else {
            debug!("Terminal latency is too long, falling back to default terminal colors.");
            follow_term_color = true;
        }

        OmaColorFormat::new(follow_term_color, timeout)
    });

    let no_check_dbus = if matches.get_flag("no_check_dbus") {
        true
    } else {
        config.no_check_dbus()
    };

    let apt_options = matches
        .try_get_many::<String>("apt_options")
        .ok()
        .flatten()
        .or_else(|| {
            matches
                .subcommand()
                .and_then(|(_, x)| x.try_get_many("apt_options").ok())
                .flatten()
        })
        .unwrap_or_default();

    let oma_args = OmaArgs {
        dry_run,
        network_thread: config.network_thread(),
        no_progress,
        no_check_dbus,
        protect_essentials: config
            .general
            .as_ref()
            .map(|x| x.protect_essentials)
            .unwrap_or_else(GeneralConfig::default_protect_essentials),
        another_apt_options: apt_options.map(|x| x.to_string()).collect::<Vec<_>>(),
    };

    let exit_code = match matches.subcommand() {
        Some(("install", args)) => {
            let input = pkgs_getter(args).unwrap_or_default();

            let install_args = InstallArgs {
                no_refresh: args.get_flag("no_refresh"),
                install_dbg: args.get_flag("install_dbg"),
                reinstall: args.get_flag("reinstall"),
                no_fixbroken: !args.get_flag("fix_broken"),
                yes: args.get_flag("yes"),
                force_yes: args.get_flag("force_yes"),
                force_confnew: args.get_flag("force_confnew"),
                install_recommends: args.get_flag("install_recommends"),
                install_suggests: args.get_flag("install_suggests"),
                no_install_recommends: args.get_flag("no_install_recommends"),
                no_install_suggests: args.get_flag("no_install_recommends"),
                no_refresh_topic: no_refresh_topics(&config, args),
                force_unsafe_io: args.get_flag("force_unsafe_io"),
                sysroot,
            };

            install::execute(input, install_args, oma_args)?
        }
        Some(("upgrade", args)) => {
            let pkgs_unparse = pkgs_getter(args).unwrap_or_default();

            let args = UpgradeArgs {
                yes: args.get_flag("yes"),
                force_yes: args.get_flag("force_yes"),
                force_confnew: args.get_flag("force_confnew"),
                sysroot,
                no_refresh_topcs: no_refresh_topics(&config, args),
                autoremove: args.get_flag("autoremove"),
                force_unsafe_io: args.get_flag("force_unsafe_io"),
            };

            upgrade::execute(pkgs_unparse, args, oma_args)?
        }
        Some(("download", args)) => {
            let keyword = pkgs_getter(args).unwrap_or_default();
            let keyword = keyword.iter().map(|x| x.as_str()).collect::<Vec<_>>();

            let path = args
                .get_one::<String>("path")
                .cloned()
                .map(|x| PathBuf::from(&x));

            download::execute(keyword, path, oma_args)?
        }
        Some((x, args)) if x == "remove" || x == "purge" => {
            let input = pkgs_getter(args).unwrap_or_default();
            let input = input.iter().map(|x| x.as_str()).collect::<Vec<_>>();

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
                fix_broken: args.get_flag("fix_broken"),
                force_unsafe_io: args.get_flag("force_unsafe_io"),
            };

            remove::execute(input, args, oma_args)?
        }
        Some(("refresh", args)) => {
            refresh::execute(oma_args, sysroot, no_refresh_topics(&config, args))?
        }
        Some(("show", args)) => {
            let input = pkgs_getter(args).unwrap_or_default();
            let input = input.iter().map(|x| x.as_str()).collect::<Vec<_>>();
            let all = args.get_flag("all");
            let json = args.get_flag("json");

            show::execute(
                all,
                input,
                sysroot,
                json,
                oma_args.another_apt_options,
                no_progress,
            )?
        }
        Some(("search", args)) => {
            let patterns = args
                .get_many::<String>("pattern")
                .map(|x| x.map(|x| x.to_owned()).collect::<Vec<_>>())
                .unwrap();

            let no_pager = !stdout().is_terminal()
                || !stderr().is_terminal()
                || !stdin().is_terminal()
                || args.get_flag("no_pager");

            let json = args.get_flag("json");

            let engine = config.search_engine();

            search::execute(
                &patterns,
                no_progress,
                sysroot,
                engine,
                no_pager,
                json,
                oma_args.another_apt_options,
            )?
        }
        Some((x, args)) if x == "files" || x == "provides" => {
            let arg = if x == "files" { "package" } else { "pattern" };
            let pkg = args.get_one::<String>(arg).unwrap();
            let is_bin = args.get_flag("bin");
            let println = config.search_contents_println()
                || !stdout().is_terminal()
                || !stderr().is_terminal()
                || !stdin().is_terminal()
                || args.get_flag("no_pager");

            contents_find::execute(x, is_bin, pkg, no_progress, sysroot, println)?
        }
        Some(("fix-broken", _)) => fix_broken::execute(oma_args, sysroot)?,
        Some(("pick", args)) => {
            let pkg_str = args.get_one::<String>("package").unwrap();

            pick::execute(
                pkg_str,
                args.get_flag("no_refresh"),
                oma_args,
                sysroot,
                no_refresh_topics(&config, args),
            )?
        }
        Some(("mark", args)) => {
            let op = args.get_one::<String>("action").unwrap();

            let pkgs = pkgs_getter(args).unwrap();
            let dry_run = args.get_flag("dry_run");

            mark::execute(
                op,
                pkgs,
                dry_run,
                sysroot,
                oma_args.another_apt_options,
                no_progress,
            )?
        }
        Some(("command-not-found", args)) => {
            command_not_found::execute(args.get_one::<String>("package").unwrap())?
        }
        Some(("list", args)) => {
            let pkgs = pkgs_getter(args).unwrap_or_default();
            let all = args.get_flag("all");
            let installed = args.get_flag("installed");
            let upgradable = args.get_flag("upgradable");
            let manual = args.get_flag("manually-installed");
            let auto = args.get_flag("automatic");
            let json = args.get_flag("json");
            let autoremovable = args.get_flag("autoremovable");

            let flags = ListFlags {
                all,
                installed,
                upgradable,
                manual,
                auto,
                autoremovable,
            };

            list::execute(flags, pkgs, sysroot, json, oma_args.another_apt_options)?
        }
        Some(("depends", args)) => {
            let pkgs = pkgs_getter(args).unwrap();
            let json = args.get_flag("json");

            depends::execute(
                pkgs,
                sysroot,
                json,
                oma_args.another_apt_options,
                no_progress,
            )?
        }
        Some(("rdepends", args)) => {
            let pkgs = pkgs_getter(args).unwrap();
            let json = args.get_flag("json");

            rdepends::execute(
                pkgs,
                sysroot,
                json,
                oma_args.another_apt_options,
                no_progress,
            )?
        }
        Some(("clean", _)) => clean::execute(no_progress, sysroot, oma_args.another_apt_options)?,
        Some(("history", _)) => subcommand::history::execute_history(sysroot)?,
        Some(("undo", _)) => history::execute_undo(oma_args, sysroot)?,
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
                no_check_dbus,
                sysroot,
            };

            let client = Client::builder()
                .user_agent(APP_USER_AGENT)
                .build()
                .unwrap();

            topics::execute(args, client, oma_args)?
        }
        Some(("pkgnames", args)) => {
            let keyword = args.get_one::<String>("keyword").map(|x| x.as_str());
            let filter_installed = args.get_flag("installed");

            pkgnames::execute(keyword, sysroot, filter_installed)?
        }
        Some(("tui", _)) | None => tui::execute(TuiArgs {
            sysroot,
            no_progress,
            dry_run,
            network_thread: oma_args.network_thread,
            no_check_dbus,
            another_apt_options: oma_args.another_apt_options,
        })?,
        #[cfg(feature = "aosc")]
        Some(("mirror", args)) => {
            let subcmd = args.subcommand();

            match subcmd {
                None => mirror::tui(
                    no_progress,
                    !args.get_flag("no_refresh_topics"),
                    oma_args.network_thread,
                    args.get_flag("no_refresh"),
                )?,
                Some(("sort-mirrors", _)) => mirror::set_order(
                    no_progress,
                    !args.get_flag("no_refresh_topics"),
                    oma_args.network_thread,
                    args.get_flag("no_refresh"),
                )?,
                Some(("set", sub_args)) => {
                    let names = sub_args
                        .get_many::<String>("names")
                        .unwrap()
                        .map(|x| x.as_str())
                        .collect::<Vec<_>>();

                    mirror::operate(
                        no_progress,
                        !args.get_flag("no_refresh_topics"),
                        oma_args.network_thread,
                        args.get_flag("no_refresh"),
                        names,
                        mirror::Operate::Set,
                    )?
                }
                Some(("add", sub_args)) => {
                    let names = sub_args
                        .get_many::<String>("names")
                        .unwrap()
                        .map(|x| x.as_str())
                        .collect::<Vec<_>>();

                    mirror::operate(
                        no_progress,
                        !args.get_flag("no_refresh_topics"),
                        oma_args.network_thread,
                        args.get_flag("no_refresh"),
                        names,
                        mirror::Operate::Add,
                    )?
                }
                Some(("remove", sub_args)) => {
                    let names = sub_args
                        .get_many::<String>("names")
                        .unwrap()
                        .map(|x| x.as_str())
                        .collect::<Vec<_>>();

                    mirror::operate(
                        no_progress,
                        !args.get_flag("no_refresh_topics"),
                        oma_args.network_thread,
                        args.get_flag("no_refresh"),
                        names,
                        mirror::Operate::Remove,
                    )?
                }
                Some(("speedtest", sub_args)) => mirror::speedtest(
                    no_progress,
                    sub_args.get_flag("set_fastest"),
                    !args.get_flag("no_refresh_topics"),
                    oma_args.network_thread,
                    args.get_flag("no_refresh"),
                )?,
                _ => unreachable!(),
            }
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
    };

    Ok(exit_code)
}

fn no_refresh_topics(config: &Config, args: &ArgMatches) -> bool {
    if !cfg!(feature = "aosc") {
        return true;
    }

    config.no_refresh_topics() || args.get_flag("no_refresh_topics")
}

#[inline]
fn color_formatter() -> &'static OmaColorFormat {
    COLOR_FORMATTER.get().unwrap()
}

fn display_error_and_can_unlock(e: OutputError) -> io::Result<bool> {
    let mut unlock = true;
    if !e.description.is_empty() {
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
                        cause_writer.get_prefix_len() + WRITER.get_prefix_len(),
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
    } else {
        // 单独处理例外情况的错误
        let errs = Chain::new(&e);
        for e in errs {
            if let Some(e) = e.downcast_ref::<OmaDbusError>() {
                match e {
                    OmaDbusError::FailedConnectDbus(e) => {
                        error!("{}", fl!("failed-check-dbus"));
                        due_to!("{e}");
                        warn!("{}", fl!("failed-check-dbus-tips-1"));
                        info!("{}", fl!("failed-check-dbus-tips-2"));
                        info!("{}", fl!("failed-check-dbus-tips-3"));
                    }
                    _ => return Ok(true),
                }
            }

            if e.downcast_ref::<LockError>().is_some() {
                unlock = false;
                if find_another_oma().is_err() {
                    error!("{}", fl!("failed-to-lock-oma"));
                }
            }
        }
    }

    Ok(unlock)
}

fn find_another_oma() -> Result<(), OutputError> {
    RT.block_on(async { find_another_oma_inner().await })
}

async fn find_another_oma_inner() -> Result<(), OutputError> {
    let conn = create_dbus_connection().await?;
    let status = get_another_oma_status(&conn).await?;
    error!("{}", fl!("another-oma-is-running", s = status));

    Ok(())
}

fn single_handler() {
    if SPAWN_NEW_OMA.load(Ordering::Relaxed) {
        return;
    }

    let allow_ctrlc = ALLOWCTRLC.load(Ordering::Relaxed);

    // Dealing with lock
    if LOCKED.load(Ordering::Relaxed) {
        unlock_oma().expect("Failed to unlock instance.");
    }

    // Show cursor before exiting.
    // This is not a big deal so we won't panic on this.
    let _ = WRITER.show_cursor();

    if !allow_ctrlc {
        info!("{}", fl!("user-aborted-op"));
    }

    std::process::exit(130);
}
