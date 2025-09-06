use std::env::{self, args};
use std::ffi::CString;
use std::fs::{create_dir_all, read_dir, remove_file};
use std::io::{self, IsTerminal, stderr, stdin};
use std::path::{Path, PathBuf};

use std::process::{Command, exit};
use std::sync::{LazyLock, OnceLock};
use std::thread;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

mod args;
mod config;
mod error;
mod install_progress;
mod lang;
mod path_completions;
mod pb;
mod subcommand;
mod table;
mod tui;
mod utils;

use args::{CliExecuter, OhManagerAilurus, print_version};
use clap::builder::FalseyValueParser;
use clap::{ArgAction, ArgMatches, Args, ColorChoice, CommandFactory, FromArgMatches, arg};
use clap_complete::CompleteEnv;
use clap_i18n_richformatter::{ClapI18nRichFormatter, init_clap_rich_formatter_localizer};
use error::OutputError;
use i18n_embed::{DesktopLanguageRequester, Localizer};
use lang::LANGUAGE_LOADER;
use oma_console::OmaLayer;
use oma_console::print::{OmaColorFormat, termbg};
use oma_console::writer::{MessageType, Writer, writeln_inner};
use oma_utils::OsRelease;
use oma_utils::dbus::{create_dbus_connection, get_another_oma_status};
use oma_utils::oma::{terminal_ring, unlock_oma};
use reqwest::Client;
use rustix::stdio::stdout;
use subcommand::utils::{LockError, is_terminal};
use tokio::runtime::Runtime;
use tracing::{debug, error, info, warn};
use tracing_appender::non_blocking::WorkerGuard;
use tracing_subscriber::prelude::__tracing_subscriber_SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;
use tracing_subscriber::{EnvFilter, Layer, fmt};
use tui::Tui;
use utils::{is_root, is_ssh_from_loginctl};

use std::sync::atomic::{AtomicBool, Ordering};

use oma_console::console;

use crate::config::Config;
use crate::error::Chain;
use crate::install_progress::osc94_progress;
use crate::subcommand::*;

static NOT_DISPLAY_ABORT: AtomicBool = AtomicBool::new(false);
static LOCKED: AtomicBool = AtomicBool::new(false);
static NOT_ALLOW_CTRLC: AtomicBool = AtomicBool::new(false);
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
static WRITER: LazyLock<Writer> = LazyLock::new(Writer::default);

#[derive(Debug, Args)]
pub struct GlobalOptions {
    /// Run oma in "dry-run" mode
    #[arg(
        long,
        global = true,
        help = fl!("clap-dry-run-help"),
        long_help = fl!("clap-dry-run-long-help"),
        env = "OMA_DRY_RUN",
        value_parser = FalseyValueParser::new()
    )]
    dry_run: bool,
    /// Run oma with debug output
    #[arg(
        long,
        global = true,
        help = fl!("clap-debug-help"),
        long_help = fl!("clap-debug-long-help"),
        env = "OMA_DEBUG",
        value_parser = FalseyValueParser::new()
    )]
    debug: bool,
    /// Represents the color preferences for program output
    #[arg(long, global = true, default_value = "auto", help = fl!("clap-color-help"))]
    color: ColorChoice,
    /// Output result with terminal theme color
    #[arg(
        long,
        global = true,
        env = "OMA_FOLLOW_TERMINAL_COLOR",
        value_parser = FalseyValueParser::new(),
        help = fl!("clap-follow-terminal-color-help")
    )]
    follow_terminal_color: bool,
    /// Do not display progress bar
    #[arg(
        long,
        global = true,
        env = "OMA_NO_PROGRESS",
        value_parser = FalseyValueParser::new(),
    )]
    no_progress: bool,
    /// Run oma do not check dbus
    #[arg(
        long,
        global = true,
        env = "OMA_NO_CHECK_DBUS",
        help = fl!("clap-no-check-dbus-help"),
        value_parser = FalseyValueParser::new()
    )]
    no_check_dbus: bool,
    /// Run oma do not check battery status
    #[arg(long, global = true, env = "OMA_NO_CHECK_BATTERY", help = fl!("clap-no-check-battery-help"), value_parser = FalseyValueParser::new()
)]
    no_check_battery: bool,
    /// Run oma do not take wake lock
    #[arg(long, global = true, env = "OMA_NO_TAKE_WAKE_LOCK", help = fl!("clap-no-take-wake-lock-help"), value_parser = FalseyValueParser::new()
)]
    no_take_wake_lock: bool,
    /// Print version
    // FIXME: ArgAcrion::Version buggy
    #[arg(short, long, help = fl!("clap-version-help"))]
    version: bool,
    /// Set sysroot target directory
    #[arg(long, global = true, default_value = "/", env = "OMA_SYSROOT", help = fl!("clap-sysroot-help"))]
    sysroot: PathBuf,
    /// Set apt options
    #[arg(long, global = true, action = ArgAction::Append, help = fl!("clap-apt-options-help"))]
    apt_options: Vec<String>,
    /// Don't ring if oma completes the transaction
    #[arg(long, global = true, env = "OMA_NO_BELL", help = fl!("clap-no-bell-help"), value_parser = FalseyValueParser::new()
)]
    no_bell: bool,
    /// Setup download threads (default as 4)
    #[arg(long, short = 't', global = true, env = "OMA_DOWNLOAD_THREADS", help = fl!("clap-download-threads-help"))]
    download_threads: Option<usize>,
    /// Print help
    #[arg(long, short, global = true, action = ArgAction::HelpLong, help = fl!("clap-help"))]
    help: Option<bool>,
}

fn main() {
    // 补全
    CompleteEnv::with_factory(OhManagerAilurus::command)
        .completer("oma")
        .complete();

    // 使系统错误使用系统 locale 语言输出
    unsafe {
        let s = CString::new("").unwrap();
        libc::setlocale(libc::LC_ALL, s.as_ptr());
    }

    init_clap_rich_formatter_localizer();
    init_localizer();

    ctrlc::set_handler(single_handler).expect("oma could not initialize SIGINT handler.");

    // 要适配额外的插件子命令，所以这里要保留 matches
    let (matches, oma) = parse_args();

    if oma.global.version {
        print_version();
        exit(0);
    }

    // Init config file
    let config = Config::read();

    #[cfg(feature = "tokio-console")]
    console_subscriber::init();

    #[cfg(not(feature = "tokio-console"))]
    let (_guard, file) = init_logger(&oma, &config);

    debug!(
        "Run oma with args: {} (pid: {})",
        args().collect::<Vec<_>>().join(" "),
        std::process::id()
    );
    debug!("oma version: {}", env!("CARGO_PKG_VERSION"));
    debug!("OS: {:?}", OsRelease::new());

    match file {
        Ok(file) => {
            debug!("Log file: {}", file);
        }
        Err(e) => {
            warn!("Failed to write log to file: {e}");
        }
    }

    let code = match try_main(oma, config, matches) {
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

    exit(code);
}

fn parse_args() -> (ArgMatches, OhManagerAilurus) {
    let matches = match OhManagerAilurus::command().try_get_matches() {
        Ok(m) => m,
        Err(e) => {
            let e = e.apply::<ClapI18nRichFormatter>();
            e.exit();
        }
    };

    let oma = match OhManagerAilurus::from_arg_matches(&matches).map_err(|e| {
        let mut cmd = OhManagerAilurus::command();
        e.format(&mut cmd)
    }) {
        Ok(oma) => oma,
        Err(e) => e.exit(),
    };

    (matches, oma)
}

fn init_localizer() {
    let localizer = crate::lang::localizer();
    let requested_languages = DesktopLanguageRequester::requested_languages();

    if let Err(error) = localizer.select(&requested_languages) {
        eprintln!("Error while loading languages for library_fluent {error}");
    }

    // Windows Terminal doesn't support bidirectional (BiDi) text, and renders the isolate characters incorrectly.
    // This is a temporary workaround for https://github.com/microsoft/terminal/issues/16574
    // TODO: this might break BiDi text, though we don't support any writing system depends on that.
    LANGUAGE_LOADER.set_use_isolating(false);
}

macro_rules! init_logger_inner {
    ($context:ident, $non_blocking:ident) => {{
        let debug_filter: EnvFilter = "hyper=off,rustls=off,debug".parse().unwrap();
        $context
            .with(
                fmt::layer()
                    .with_file(true)
                    .with_writer($non_blocking)
                    .with_filter(debug_filter),
            )
            .init()
    }};
    ($context:ident) => {{ $context.init() }};
}

fn init_logger(
    oma: &OhManagerAilurus,
    config: &Config,
) -> (Option<WorkerGuard>, anyhow::Result<String>) {
    let debug = oma.global.debug;
    let dry_run = oma.global.dry_run;

    let log_dir = if is_root() {
        PathBuf::from("/var/log/oma")
    } else {
        dirs::state_dir()
            .expect("Failed to get state dir")
            .join("oma")
    };

    let log_file = create_log_file(&log_dir);
    let mut log_guard = None;

    if !debug && !dry_run {
        let no_i18n_embd: EnvFilter = "i18n_embed=off,info".parse().unwrap();

        let context = tracing_subscriber::registry().with(
            OmaLayer::new()
                .with_ansi(enable_ansi(oma))
                .with_filter(no_i18n_embd),
        );

        if let Ok(log_file) = &log_file {
            let file_appender = tracing_appender::rolling::never(&log_dir, log_file);
            let (non_blocking, guard) = tracing_appender::non_blocking(file_appender);
            init_logger_inner!(context, non_blocking);
            log_guard = Some(guard);
        } else {
            init_logger_inner!(context);
        }
    } else {
        let filter = EnvFilter::try_from_default_env()
            .unwrap_or_else(|_| "hyper=off,rustls=off,debug".parse().unwrap());

        let context = tracing_subscriber::registry().with(
            fmt::layer()
                .event_format(
                    tracing_subscriber::fmt::format()
                        .with_file(true)
                        .with_line_number(true)
                        .with_ansi(enable_ansi(oma)),
                )
                .with_filter(filter),
        );

        if let Ok(log_file) = &log_file {
            let file_appender = tracing_appender::rolling::never(&log_dir, log_file);
            let (non_blocking, guard) = tracing_appender::non_blocking(file_appender);
            init_logger_inner!(context, non_blocking);
            log_guard = Some(guard);
        } else {
            init_logger_inner!(context);
        }
    }

    // 日志文件创建成功再去遍历文件
    if log_file.is_ok() {
        thread::scope(|s| {
            s.spawn(|| {
                let mut v = vec![];
                let dirs = read_dir(&log_dir)
                    .expect("Failed to read log dir")
                    .collect::<Vec<_>>();

                for p in &dirs {
                    let Ok(p) = p else {
                        continue;
                    };

                    let file_name = p.file_name();
                    let file_name = file_name.to_string_lossy();
                    let Some((prefix, timestamp)) = file_name.rsplit_once('.') else {
                        continue;
                    };

                    if prefix != "oma.log" {
                        continue;
                    }

                    let Ok(timestamp) = timestamp.parse::<usize>() else {
                        continue;
                    };

                    v.push(timestamp);
                }

                if v.len() > config.save_log_count() {
                    v.sort_unstable_by(|a, b| b.cmp(a));

                    for _ in 1..=(v.len() - config.save_log_count()) {
                        let Some(pop) = v.pop() else {
                            break;
                        };

                        let log_path = log_dir.join(format!("oma.log.{pop}"));
                        if let Err(e) = remove_file(&log_path) {
                            debug!("Failed to remove file {}: {}", log_path.display(), e);
                        }
                    }
                }
            });
        });
    }

    (log_guard, log_file)
}

fn create_log_file(log_dir: &Path) -> anyhow::Result<String> {
    create_dir_all(log_dir)?;

    let log_file = format!(
        "oma.log.{}",
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs()
    );

    let log_file = log_dir.join(log_file).to_string_lossy().to_string();

    Ok(log_file)
}

#[inline]
fn enable_ansi(oma: &OhManagerAilurus) -> bool {
    (oma.global.color != ColorChoice::Never && stderr().is_terminal())
        || oma.global.color == ColorChoice::Always
}

fn try_main(
    oma: OhManagerAilurus,
    config: Config,
    matches: ArgMatches,
) -> Result<i32, OutputError> {
    init_color_formatter(&oma, &config);

    let no_progress =
        oma.global.no_progress || !is_terminal() || oma.global.debug || oma.global.dry_run;

    let code = match oma.subcmd {
        Some(subcmd) => subcmd.execute(&config, no_progress),
        None => {
            if let Some((subcommand, args)) = matches.subcommand() {
                let mut plugin = Path::new("/usr/local/libexec").join(format!("oma-{subcommand}"));
                let plugin_fallback = Path::new("/usr/libexec").join(format!("oma-{subcommand}"));

                if !plugin.is_file() {
                    plugin = plugin_fallback;
                    if !plugin.is_file() {
                        error!("{}", fl!("custom-command-unknown", subcmd = subcommand));
                        return Ok(1);
                    }
                }

                info!("{}", fl!("custom-command-applet-exec", subcmd = subcommand));
                let mut process = &mut Command::new(plugin);
                if let Some(args) = args.get_many::<String>("COMMANDS") {
                    process = process.args(args);
                }

                let status = process.status().unwrap().code().unwrap();
                if status != 0 {
                    error!("{}", fl!("custom-command-applet-exception", s = status));
                }

                Ok(status)
            } else {
                Tui::from(&oma.global).execute(&config, no_progress)
            }
        }
    };

    if !oma.global.no_bell && config.bell() {
        terminal_ring();
    }

    code
}

fn init_color_formatter(oma: &OhManagerAilurus, config: &Config) {
    let mut follow_term_color = oma.global.follow_terminal_color || config.follow_terminal_color();
    let no_color = oma.global.color == ColorChoice::Never;

    if no_color {
        unsafe { env::set_var("NO_COLOR", "1") };
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

    let status = match status.as_str() {
        "Pending" => fl!("status-pending"),
        "Downloading" => fl!("status-downloading"),
        pkg => fl!("status-package", pkg = pkg),
    };

    error!("{}", fl!("another-oma-is-running", s = status));

    Ok(())
}

fn single_handler() {
    if NOT_ALLOW_CTRLC.load(Ordering::Relaxed) {
        return;
    }

    // Force drop osc94 progress
    osc94_progress(0.0, true);

    let not_display_abort = NOT_DISPLAY_ABORT.load(Ordering::Relaxed);

    // Dealing with lock
    if LOCKED.load(Ordering::Relaxed) {
        unlock_oma().expect("Failed to unlock instance.");
    }

    // Show cursor before exiting.
    // This is not a big deal so we won't panic on this.
    let _ = WRITER.show_cursor();

    if !not_display_abort {
        info!("{}", fl!("user-aborted-op"));
    }

    std::process::exit(130);
}
