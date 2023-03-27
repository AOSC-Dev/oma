use std::process::exit;

use anyhow::Result;
use clap::{ArgAction, Parser, Subcommand};

use oma::{MarkAction, Oma};
use cli::Writer;
use console::style;
use nix::sys::signal;
use once_cell::sync::Lazy;
use os_release::OsRelease;
use std::sync::atomic::{AtomicBool, AtomicI32, Ordering};
use time::macros::offset;
use time::{OffsetDateTime, UtcOffset};
use tracing::metadata::LevelFilter;
use tracing_subscriber::{
    fmt, prelude::__tracing_subscriber_SubscriberExt, util::SubscriberInitExt, Layer,
};
use utils::unlock_oma;

mod oma;
mod checksum;
mod cli;
mod contents;
mod db;
mod download;
mod formatter;
mod pager;
mod pkg;
mod utils;
mod verify;

static SUBPROCESS: AtomicI32 = AtomicI32::new(-1);
static ALLOWCTRLC: AtomicBool = AtomicBool::new(false);
static LOCKED: AtomicBool = AtomicBool::new(false);
static AILURUS: AtomicBool = AtomicBool::new(false);
static WRITER: Lazy<Writer> = Lazy::new(Writer::new);
static DRYRUN: AtomicBool = AtomicBool::new(false);
static TIME_OFFSET: Lazy<UtcOffset> =
    Lazy::new(|| UtcOffset::local_offset_at(OffsetDateTime::UNIX_EPOCH).unwrap_or(offset!(UTC)));
static ARGS: Lazy<String> = Lazy::new(|| std::env::args().collect::<Vec<_>>().join(" "));

fn single_handler() {
    // Kill subprocess
    let subprocess_pid = SUBPROCESS.load(Ordering::Relaxed);
    let allow_ctrlc = ALLOWCTRLC.load(Ordering::Relaxed);
    if subprocess_pid > 0 {
        let pid = nix::unistd::Pid::from_raw(subprocess_pid);
        signal::kill(pid, signal::SIGTERM).expect("Failed to kill child process.");
        if !allow_ctrlc {
            info!("User aborted the operation");
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

#[derive(Parser, Debug)]
#[clap(about, version, author)]
pub struct Args {
    #[clap(subcommand)]
    subcommand: OmaCommand,
    #[arg(long, hide = true, action = ArgAction::Count)]
    ailurus: u8,
    /// Dry-run oma
    #[arg(long, short = 'd')]
    dry_run: bool,
    /// Debug mode (for --dry-run argument)
    #[arg(long)]
    debug: bool,
}

#[derive(Subcommand, Debug)]
enum OmaCommand {
    /// Install Package
    Install(InstallOptions),
    /// Update Package
    #[clap(alias = "full-upgrade", alias = "dist-upgrade")]
    Upgrade(UpgradeOptions),
    /// Download Package
    Download(Download),
    /// Delete Package
    #[clap(alias = "delete", alias = "purge")]
    Remove(RemoveOptions),
    /// Refresh Package database
    #[clap(alias = "update")]
    Refresh,
    /// Show Package
    Show(Show),
    /// Search Package
    Search(Search),
    /// package list files
    ListFiles(ListFiles),
    /// Search file from package
    Provides(Provides),
    /// Fix system dependencies broken status
    FixBroken(FixBroken),
    /// Pick a package version
    Pick(PickOptions),
    /// Mark a package status
    Mark(Mark),
    #[clap(hide = true)]
    CommandNotFound(CommandNotFound),
    /// List of packages
    List(ListOptions),
    /// Check package dependencies
    #[clap(alias = "dep")]
    Depends(Dep),
    /// Check package reverse dependencies
    #[clap(alias = "rdep")]
    Rdepends(Dep),
    /// Clean downloaded packages
    Clean,
    /// See omakase log
    Log,
}

#[derive(Parser, Debug)]
struct Dep {
    /// Package(s) name
    pkgs: Vec<String>,
}

#[derive(Parser, Debug)]
struct CommandNotFound {
    kw: String,
}

#[derive(Parser, Debug)]
pub struct FixBroken {
    /// Dry-run oma
    #[arg(long, short = 'd')]
    dry_run: bool,
}

#[derive(Parser, Debug)]
pub struct Download {
    /// Package(s) name
    packages: Vec<String>,
    /// Download to path
    #[arg(long, short)]
    path: Option<String>,
}

#[derive(Parser, Debug, Clone)]
pub struct InstallOptions {
    /// Package(s) name
    pub packages: Vec<String>,
    /// Install package(s) debug symbol
    #[arg(long, alias = "dbg")]
    pub install_dbg: bool,
    /// Reinstall package(s)
    #[arg(long)]
    pub reinstall: bool,
    /// Do not try fix package depends broken status
    #[arg(long)]
    pub no_fixbroken: bool,
    /// Do not refresh packages database
    #[arg(long)]
    pub no_upgrade: bool,
    /// Automatic run oma install
    #[arg(long, short = 'y')]
    pub yes: bool,
    /// Force install packages for can't resolve depends
    #[arg(long)]
    pub force_yes: bool,
    /// Install package use dpkg --force-confnew
    #[arg(long)]
    pub force_confnew: bool,
    /// Dry-run oma
    #[arg(long, short = 'd')]
    pub dry_run: bool,
}

#[derive(Parser, Debug, Clone)]
pub struct UpgradeOptions {
    /// Package(s) name
    packages: Vec<String>,
    /// Automatic run oma install
    #[arg(long, short = 'y')]
    yes: bool,
    /// Force install packages for can't resolve depends
    #[arg(long)]
    force_yes: bool,
    /// Install package use dpkg --force-confnew
    #[arg(long)]
    force_confnew: bool,
    /// Dry-run oma
    #[arg(long, short = 'd')]
    dry_run: bool,
}

#[derive(Parser, Debug)]
struct ListFiles {
    /// Package name
    package: String,
}

#[derive(Parser, Debug, Clone)]
pub struct PickOptions {
    /// Package name
    package: String,
    /// Do not try fix package depends broken status
    #[arg(long)]
    no_fixbroken: bool,
    /// Do not refresh packages database
    #[arg(long)]
    no_upgrade: bool,
    /// Dry-run oma
    #[arg(long, short = 'd')]
    dry_run: bool,
}

#[derive(Parser, Debug)]
struct Provides {
    /// Search keyword
    kw: String,
}

#[derive(Parser, Debug, Clone)]
pub struct RemoveOptions {
    /// Package(s) name
    packages: Vec<String>,
    /// Automatic run oma install
    #[arg(long, short = 'y')]
    yes: bool,
    /// Force install packages for can't resolve depends
    #[arg(long)]
    force_yes: bool,
    /// Keep package config
    #[arg(long)]
    keep_config: bool,
    /// Dry-run oma
    #[arg(long, short = 'd')]
    dry_run: bool,
}

#[derive(Parser, Debug)]
struct Show {
    /// Package(s) name
    packages: Vec<String>,
    #[arg(long, short = 'a')]
    is_all: bool,
}

#[derive(Parser, Debug)]
struct Search {
    /// Search keyword(s)
    keyword: Vec<String>,
}

#[derive(Parser, Debug, Clone)]
pub struct Mark {
    #[clap(subcommand)]
    action: MarkAction,
    /// Dry-run oma
    #[arg(long, short = 'd')]
    dry_run: bool,
}

#[derive(Parser, Debug)]
pub struct ListOptions {
    packages: Option<Vec<String>>,
    #[arg(long, short = 'a')]
    all: bool,
    #[arg(long, short = 'i')]
    installed: bool,
    #[arg(long, short = 'u')]
    upgradable: bool,
}

fn main() {
    // 初始化时区偏移量，这个操作不能在多线程环境下运行
    let _ = *TIME_OFFSET;

    ctrlc::set_handler(single_handler).expect(
        "Oma could not initialize SIGINT handler.\n\nPlease restart your installation environment.",
    );

    if let Err(e) = try_main() {
        if !e.to_string().is_empty() {
            error!("{e}");
        }
        unlock_oma().ok();
        exit(1);
    }

    unlock_oma().ok();

    exit(0);
}

fn try_main() -> Result<()> {
    let args = Args::parse();

    let ailurus = args.ailurus;
    if ailurus == 3 {
        AILURUS.store(true, Ordering::Relaxed);
    } else if ailurus != 0 {
        println!(
            "{} unexpected argument '{}' found\n",
            style("error:").red().bold(),
            style("\x1b[33m--ailurus\x1b[0m").bold()
        );
        println!("{}: oma <COMMAND>\n", style("Usage").bold().underlined());
        println!("For more information, try '{}'.", style("--help").bold());
        exit(3);
    }

    if args.dry_run || args.debug {
        DRYRUN.store(true, Ordering::Relaxed);

        tracing_subscriber::registry()
            .with(
                fmt::layer()
                    .with_writer(std::io::stdout)
                    .without_time()
                    .with_target(false)
                    .with_filter(if args.debug {
                        LevelFilter::DEBUG
                    } else {
                        LevelFilter::INFO
                    }),
            )
            .try_init()?;

        tracing::info!("Running in Dry-run mode");
        tracing::debug!(
            "oma version: {}\n OS: {:#?}",
            env!("CARGO_PKG_VERSION"),
            OsRelease::new()
        );
    }

    tracing::info!("Running oma with args: {}", *ARGS);

    match args.subcommand {
        OmaCommand::Install(v) => Oma::build_async_runtime()?.install(v),
        OmaCommand::Remove(v) => Oma::remove(v),
        OmaCommand::Upgrade(v) => Oma::build_async_runtime()?.update(v),
        OmaCommand::Refresh => Oma::build_async_runtime()?.refresh(),
        OmaCommand::Show(v) => Oma::show(&v.packages, v.is_all),
        OmaCommand::Search(v) => Oma::search(&v.keyword.join(" ")),
        OmaCommand::ListFiles(v) => Oma::list_files(&v.package),
        OmaCommand::Provides(v) => Oma::search_file(&v.kw),
        OmaCommand::Download(v) => Oma::build_async_runtime()?.download(v),
        OmaCommand::FixBroken(v) => Oma::build_async_runtime()?.fix_broken(v),
        OmaCommand::Pick(v) => Oma::build_async_runtime()?.pick(v),
        OmaCommand::Mark(v) => Oma::mark(v),
        OmaCommand::CommandNotFound(v) => Oma::command_not_found(&v.kw),
        OmaCommand::List(v) => Oma::list(&v),
        OmaCommand::Depends(v) => Oma::dep(&v.pkgs, false),
        OmaCommand::Rdepends(v) => Oma::dep(&v.pkgs, true),
        OmaCommand::Clean => Oma::clean(),
        OmaCommand::Log => Oma::log(),
    }?;

    Ok(())
}
