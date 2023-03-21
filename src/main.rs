use std::process::exit;

use anyhow::Result;
use clap::{ArgAction, Parser, Subcommand};

use action::{MarkAction, OmaAction};
use cli::Writer;
use console::style;
use nix::sys::signal;
use once_cell::sync::Lazy;
use std::sync::atomic::{AtomicBool, AtomicI32, Ordering};
use utils::unlock_oma;

mod action;
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
    FixBroken,
    /// Pick a package version
    Pick(PickOptions),
    /// Mark a package status
    Mark(Mark),
    #[clap(hide = true)]
    CommandNotFound(CommandNotFound),
    /// List of packages
    List(List),
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
struct FixBroken {}

#[derive(Parser, Debug)]
struct Download {
    /// Package(s) name
    packages: Vec<String>,
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
struct Mark {
    #[clap(subcommand)]
    action: MarkAction,
}

#[derive(Parser, Debug)]
struct List {
    packages: Option<Vec<String>>,
    #[arg(long, short = 'a')]
    all: bool,
    #[arg(long, short = 'i')]
    installed: bool,
    #[arg(long, short = 'u')]
    upgradable: bool,
}

#[tokio::main]
async fn main() {
    ctrlc::set_handler(single_handler).expect(
        "Oma could not initialize SIGINT handler.\n\nPlease restart your installation environment.",
    );

    if let Err(e) = try_main().await {
        if !e.to_string().is_empty() {
            error!("{e}");
        }
        unlock_oma().ok();
        exit(1);
    }

    unlock_oma().ok();

    exit(0);
}

async fn try_main() -> Result<()> {
    let args = Args::parse();

    let ailurus = args.ailurus;
    if ailurus == 3 {
        AILURUS.store(true, Ordering::Relaxed);
    } else if ailurus != 0 {
        println!("{} unexpected argument '{}' found\n", style("error:").red().bold(), style("--ailurus").yellow());
        println!("{}: oma <COMMAND>\n", style("Usage").bold().underlined());
        println!("For more information, try '{}'.", style("--help").bold());
        exit(3);
    }

    match args.subcommand {
        OmaCommand::Install(v) => OmaAction::new().await?.install(v).await,
        OmaCommand::Remove(v) => OmaAction::remove(v),
        OmaCommand::Upgrade(v) => OmaAction::new().await?.update(v).await,
        OmaCommand::Refresh => OmaAction::new().await?.refresh().await,
        OmaCommand::Show(v) => OmaAction::show(&v.packages, v.is_all),
        OmaCommand::Search(v) => OmaAction::search(&v.keyword.join(" ")),
        // TODO: up_db 的值目前写死了 true，打算实现的逻辑是这样：
        // 如果用户在终端里执行了不存在的程序，会调用 oma 找可能的软件包
        // 这时候就不去更新 Contents 数据，若用户手动调用 oma provides/list-files
        // 则强制更新 Contents
        OmaCommand::ListFiles(v) => OmaAction::list_files(&v.package),
        OmaCommand::Provides(v) => OmaAction::search_file(&v.kw),
        OmaCommand::Download(v) => OmaAction::new().await?.download(&v.packages).await,
        OmaCommand::FixBroken => OmaAction::new().await?.fix_broken().await,
        OmaCommand::Pick(v) => OmaAction::new().await?.pick(v).await,
        OmaCommand::Mark(v) => OmaAction::mark(v.action),
        OmaCommand::CommandNotFound(v) => OmaAction::command_not_found(&v.kw),
        OmaCommand::List(v) => {
            OmaAction::list(v.packages.as_deref(), v.all, v.installed, v.upgradable)
        }
        OmaCommand::Depends(v) => OmaAction::dep(&v.pkgs, false),
        OmaCommand::Rdepends(v) => OmaAction::dep(&v.pkgs, true),
        OmaCommand::Clean => OmaAction::clean(),
        OmaCommand::Log => OmaAction::log(),
    }?;

    Ok(())
}
