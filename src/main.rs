use std::env;
use std::ffi::CString;
use std::io::{self, stderr, stdin, IsTerminal};
use std::path::PathBuf;

use std::process::exit;
use std::sync::{LazyLock, OnceLock};
use std::time::Duration;

mod args;
mod config;
mod error;
mod install_progress;
mod lang;
mod pb;
mod subcommand;
mod table;
mod tui;
mod utils;

#[cfg(feature = "egg")]
mod egg;

use args::{CliExecuter, OhManagerAilurus};
use clap::{crate_name, crate_version, ArgAction, Args, ColorChoice, Parser};
use error::OutputError;
use i18n_embed::DesktopLanguageRequester;
use lang::LANGUAGE_LOADER;
use oma_console::print::{termbg, OmaColorFormat};
use oma_console::writer::{writeln_inner, MessageType, Writer};
use oma_console::WRITER;
use oma_console::{due_to, OmaLayer};
use oma_utils::dbus::{create_dbus_connection, get_another_oma_status, OmaDbusError};
use oma_utils::oma::{terminal_ring, unlock_oma};
use oma_utils::OsRelease;
use reqwest::Client;
use rustix::stdio::stdout;
use subcommand::utils::{is_terminal, LockError};
use tokio::runtime::Runtime;
use tracing::{debug, error, info, warn};
use tracing_subscriber::filter::LevelFilter;
use tracing_subscriber::prelude::__tracing_subscriber_SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;
use tracing_subscriber::{fmt, EnvFilter, Layer};
use tui::Tui;
use utils::is_ssh_from_loginctl;

use std::sync::atomic::{AtomicBool, Ordering};

use oma_console::console;

use crate::config::Config;
#[cfg(feature = "egg")]
use crate::egg::ailurus;
use crate::error::Chain;
use crate::subcommand::*;

static ALLOWCTRLC: AtomicBool = AtomicBool::new(false);
static LOCKED: AtomicBool = AtomicBool::new(false);
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

#[derive(Debug, Args)]
pub struct GlobalOptions {
    /// Run oma in “dry-run” mode. Useful for testing changes and operations without making changes to the system
    #[arg(long, global = true)]
    dry_run: bool,
    /// Run oma with debug output
    #[arg(
        long,
        global = true,
        long_help = "Run oma with debug output, including details on program parameters and data. Useful for developers and administrators to investigate and report bugs and issues"
    )]
    debug: bool,
    /// Represents the color preferences for program output
    #[arg(long, global = true, default_value = "auto")]
    color: ColorChoice,
    /// Output result with terminal theme color
    #[arg(long, global = true)]
    follow_terminal_color: bool,
    /// Do not display progress bar
    #[arg(long, global = true)]
    no_progress: bool,
    /// Run oma do not check dbus
    #[arg(long, global = true)]
    no_check_dbus: bool,
    /// Print version
    // FIXME: ArgAcrion::Version buggy
    #[arg(short, long)]
    version: bool,
    /// Set sysroot target directory
    #[arg(long, global = true, default_value = "/")]
    sysroot: PathBuf,
    /// Set apt options
    #[arg(long, global = true, action = ArgAction::Append)]
    apt_options: Vec<String>,
}

fn main() {
    init_localizer();

    ctrlc::set_handler(single_handler).expect(
        "Oma could not initialize SIGINT handler. Please restart your installation environment.",
    );

    let oma = OhManagerAilurus::parse();

    if oma.global.version {
        println!("{} {}", crate_name!(), crate_version!());
        exit(0);
    }

    #[cfg(feature = "tokio-console")]
    console_subscriber::init();

    #[cfg(not(feature = "tokio-console"))]
    init_logger(&oma);

    let code = match run_subcmd(oma) {
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

fn init_localizer() {
    let localizer = crate::lang::localizer();
    let requested_languages = DesktopLanguageRequester::requested_languages();

    if let Err(error) = localizer.select(&requested_languages) {
        eprintln!("Error while loading languages for library_fluent {}", error);
    }

    // Windows Terminal doesn't support bidirectional (BiDi) text, and renders the isolate characters incorrectly.
    // This is a temporary workaround for https://github.com/microsoft/terminal/issues/16574
    // TODO: this might break BiDi text, though we don't support any writing system depends on that.
    LANGUAGE_LOADER.set_use_isolating(false);
}

fn init_logger(oma: &OhManagerAilurus) {
    let debug = oma.global.debug;
    if !debug {
        let no_i18n_embd_info: EnvFilter = "i18n_embed=off,info".parse().unwrap();

        tracing_subscriber::registry()
            .with(
                OmaLayer::new()
                    .with_ansi(oma.global.color != ColorChoice::Never && stderr().is_terminal())
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
                                .with_line_number(true)
                                .with_ansi(
                                    oma.global.color != ColorChoice::Never
                                        && stderr().is_terminal(),
                                ),
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
                                .with_line_number(true)
                                .with_ansi(
                                    oma.global.color != ColorChoice::Never
                                        && stderr().is_terminal(),
                                ),
                        )
                        .with_filter(debug_filter),
                )
                .init();
        }
    }
}

fn run_subcmd(oma: OhManagerAilurus) -> Result<i32, OutputError> {
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

    debug!("oma version: {}", env!("CARGO_PKG_VERSION"));
    debug!("OS: {:?}", OsRelease::new());

    // Init config file
    let config = Config::read()?;
    let mut follow_term_color = oma.global.follow_terminal_color || config.follow_terminal_color();
    let no_color = oma.global.color == ColorChoice::Never;
    let no_progress = oma.global.no_progress || !is_terminal() || oma.global.debug;

    if no_color {
        env::set_var("NO_COLOR", "1");
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

    match oma.subcmd {
        Some(subcmd) => subcmd.execute(&config, no_progress),
        None => Tui::default().execute(&config, no_progress),
    }
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
                if let Err(e) = find_another_oma() {
                    debug!("{e}");
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
