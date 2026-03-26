use std::env::{self, args};
use std::ffi::CString;
use std::io::{self, IsTerminal, stderr, stdin};
use std::path::{Path, PathBuf};

use std::process::{Command, exit};
use std::sync::{LazyLock, OnceLock};
use std::time::Duration;

mod args;
mod completions;
mod config;
mod config_file;
mod core;
mod dbus;
mod error;
mod exit_handle;
mod install_progress;
mod lang;
mod logger;
mod menu;
mod pb;
mod root;
mod subcommand;
mod table;
mod tui;
mod utils;

use args::{CliExecuter, OhManagerAilurus};
use clap::builder::FalseyValueParser;
use clap::{ArgAction, ArgMatches, Args, ColorChoice, CommandFactory, FromArgMatches};
use clap_complete::CompleteEnv;
use clap_i18n_richformatter::CommandI18nExt;
use dbus::is_ssh_from_loginctl;
use error::OutputError;
use i18n_embed::{DesktopLanguageRequester, Localizer};
use lang::LANGUAGE_LOADER;
use oma_console::{
    print::{OmaColorFormat, termbg},
    terminal::wrap_content,
    writer::Writer,
};
use oma_utils::{OsRelease, is_termux};
use rustix::stdio::stdout;
use spdlog::{debug, info, prelude::error as log_error, warn};
use tokio::runtime::Runtime;
use tui::Tui;

use std::sync::atomic::{AtomicBool, Ordering};

use oma_console::console;

use crate::config::OmaConfig;
use crate::config_file::ConfigFile;
use crate::error::Chain;
use crate::exit_handle::ExitHandle;
#[cfg(not(feature = "tokio-console"))]
use crate::logger::init_logger;
use crate::logger::remove_old_log_file_impl;
use crate::subcommand::*;

static NOT_DISPLAY_ABORT: AtomicBool = AtomicBool::new(false);
static NOT_ALLOW_CTRLC: AtomicBool = AtomicBool::new(false);
static DEFAULT_USER_AGENT: &str = concat!("oma/", env!("CARGO_PKG_VERSION"));
static COLOR_FORMATTER: OnceLock<OmaColorFormat> = OnceLock::new();
static RT: LazyLock<Runtime> = LazyLock::new(|| {
    tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .expect("Failed to init async runtime")
});

static WRITER: LazyLock<Writer> = LazyLock::new(Writer::default);
static LOCK: OnceLock<PathBuf> = OnceLock::new();
static NO_COLOR: AtomicBool = AtomicBool::new(false);

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
        help = fl!("clap-no-progress-help")
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
    #[arg(short, long, action = ArgAction::Version, help = fl!("clap-version-help"))]
    version: Option<bool>,
    /// Set sysroot target directory
    #[arg(long, global = true, default_value = sysroot_default_value(), env = "OMA_SYSROOT", help = fl!("clap-sysroot-help"))]
    sysroot: PathBuf,
    /// Set apt options
    #[arg(long, short = 'o', global = true, action = ArgAction::Append, help = fl!("clap-apt-options-help"))]
    apt_options: Vec<String>,
    /// Don't ring if oma completes the transactionf
    #[arg(long, global = true, env = "OMA_NO_BELL", help = fl!("clap-no-bell-help"), value_parser = FalseyValueParser::new()
)]
    no_bell: bool,
    /// Setup download threads (default as 4)
    #[arg(long, short = 't', global = true, env = "OMA_DOWNLOAD_THREADS", help = fl!("clap-download-threads-help"))]
    download_threads: Option<usize>,
    /// Print help
    #[arg(long, short, global = true, action = ArgAction::Help, help = fl!("clap-help"))]
    help: Option<bool>,
    /// No use config
    #[arg(long, global = true, env = "OMA_NO_CONFIG", help = fl!("clap-no-config-help"), value_parser = FalseyValueParser::new())]
    no_config: bool,
    /// User agent string to use for HTTP requests.
    #[arg(long, global = true, env = "OMA_USER_AGENT", help = fl!("clap-user-agent-help"))]
    user_agent: Option<String>,
}

fn main() {
    // 使系统错误使用系统 locale 语言输出
    unsafe {
        let s = CString::new("").unwrap();
        libc::setlocale(libc::LC_ALL, s.as_ptr());
    }

    init_localizer();

    // 补全
    CompleteEnv::with_factory(OhManagerAilurus::command)
        .completer("oma")
        .complete();

    ctrlc::set_handler(exit_handle::signal_handler)
        .expect("oma could not initialize SIGINT handler.");

    // 要适配额外的插件子命令，所以这里要保留 matches
    let (matches, oma) = parse_args();

    #[cfg(feature = "tokio-console")]
    console_subscriber::init();

    #[cfg(not(feature = "tokio-console"))]
    let file = init_logger(&oma);

    debug!(
        "Run oma with args: {} (pid: {})",
        args().collect::<Vec<_>>().join(" "),
        std::process::id()
    );
    debug!("oma version: {}", env!("CARGO_PKG_VERSION"));

    let config_ctx = read_config_from_file_and_cli(oma);

    debug!("OS: {:?}", OsRelease::new());
    if config_ctx.sysroot.to_string_lossy() != "/" {
        debug!(
            "--sysroot OS: {:?}",
            OsRelease::new_from(config_ctx.sysroot.join("etc/os-release"))
        );
    }

    let _remove_old_log_worker = remove_old_log_file_impl(file, &config_ctx);

    config_ctx.init_apt_config();

    let no_bell = config_ctx.no_bell;

    match try_main(config_ctx, matches) {
        Ok(exit_code) => exit_code.handle(!no_bell),
        Err(e) => {
            if let Err(e) = display_error(e) {
                eprintln!("Failed to display error: {e}");
            }

            if !no_bell {
                exit_handle::terminal_ring();
            }

            exit(1)
        }
    }
}

fn read_config_from_file_and_cli(oma: OhManagerAilurus) -> OmaConfig {
    // Init config file
    let config_file = if oma.global.no_config {
        warn!("{}", fl!("no-config-warning"));
        ConfigFile::default()
    } else {
        ConfigFile::read()
    };

    let mut config_ctx = OmaConfig::from_config_file(config_file);
    debug!("Config file: {:#?}", config_ctx);
    config_ctx.update_from_global_cli_opts(oma);
    debug!("Config: {:#?}", config_ctx);

    config_ctx
}

fn parse_args() -> (ArgMatches, OhManagerAilurus) {
    let matches = OhManagerAilurus::command().get_matches_i18n();

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

fn try_main(mut config: OmaConfig, matches: ArgMatches) -> Result<ExitHandle, OutputError> {
    init_color_formatter(&config);
    let subcmd = config.take_subcmd();

    match subcmd {
        Some(subcmd) => subcmd.execute(config),
        None => {
            if let Some((subcommand, args)) = matches.subcommand() {
                let mut plugin = Path::new("/usr/local/libexec").join(format!("oma-{subcommand}"));
                let plugin_fallback = Path::new("/usr/libexec").join(format!("oma-{subcommand}"));

                if !plugin.is_file() {
                    plugin = plugin_fallback;
                    if !plugin.is_file() {
                        log_error!("{}", fl!("custom-command-unknown", subcmd = subcommand));
                        return Ok(ExitHandle::default()
                            .ring(true)
                            .status(exit_handle::ExitStatus::Fail));
                    }
                }

                info!("{}", fl!("custom-command-applet-exec", subcmd = subcommand));
                let mut process = &mut Command::new(plugin);
                if let Some(args) = args.get_many::<String>("COMMANDS") {
                    process = process.args(args);
                }

                let status = process.status().unwrap().code().unwrap();
                if status != 0 {
                    log_error!("{}", fl!("custom-command-applet-exception", s = status));
                }

                Ok(ExitHandle::default()
                    .ring(true)
                    .status(exit_handle::ExitStatus::Other(status)))
            } else {
                Tui::default().execute(config)
            }
        }
    }
}

fn init_color_formatter(config: &OmaConfig) {
    let mut follow_term_color = config.follow_terminal_color;
    let no_color = config.color == ColorChoice::Never;

    if no_color {
        unsafe { env::set_var("NO_COLOR", "1") };
        NO_COLOR.store(true, Ordering::Relaxed);
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

fn display_error(e: OutputError) -> io::Result<()> {
    if !e.description.is_empty() {
        log_error!("{e}");

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

                    let res = wrap_content(
                        "",
                        &c.to_string(),
                        cause_writer.get_max_len(),
                        cause_writer.get_prefix_len() + WRITER.get_prefix_len(),
                    )
                    .into_iter()
                    .map(|(_, s)| s)
                    .collect::<Vec<_>>();

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

#[inline]
pub fn get_lock(sysroot: &Path) -> &Path {
    LOCK.get_or_init(|| sysroot.join("run/lock/oma.lock"))
}

fn sysroot_default_value() -> &'static str {
    if is_termux() {
        "/data/data/com.termux/files/usr/"
    } else {
        "/"
    }
}
