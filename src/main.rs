use std::process::exit;

use anyhow::Result;
use clap::{Parser, Subcommand};

use action::{unlock_oma, MarkAction, OmaAction};
use lazy_static::lazy_static;
use nix::sys::signal;
use std::sync::atomic::{AtomicBool, AtomicI32, Ordering};

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

pub static SUBPROCESS: AtomicI32 = AtomicI32::new(-1);
pub static ALLOWCTRLC: AtomicBool = AtomicBool::new(false);
pub static LOCKED: AtomicBool = AtomicBool::new(false);

lazy_static! {
    pub static ref WRITER: cli::Writer = cli::Writer::new();
}

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
}

#[derive(Subcommand, Debug)]
enum OmaCommand {
    /// Install Package
    Install(Install),
    /// Update Package
    #[clap(alias = "full-upgrade", alias = "dist-upgrade")]
    Upgrade(Update),
    /// Download Package
    Download(Download),
    /// Delete Package
    #[clap(alias = "delete", alias = "purge")]
    Remove(Delete),
    /// Refresh Package database
    #[clap(alias = "update")]
    Refresh(Refresh),
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
    Pick(Pick),
    /// Mark a package status
    Mark(Mark),
    #[clap(hide = true)]
    CommandNotFound(CommandNotFound),
    /// List of packages
    List(List),
    /// Check package dependencies
    #[clap(alias = "dep")]
    Depends(Dep),
    /// Clean downloaded packages
    Clean(Clean),
}

#[derive(Parser, Debug)]
struct Clean;

#[derive(Parser, Debug)]
struct Dep {
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
    packages: Vec<String>,
}

#[derive(Parser, Debug)]
struct Install {
    packages: Vec<String>,
    #[arg(long, alias = "dbg")]
    install_dbg: bool,
    #[arg(long)]
    reinstall: bool,
    #[arg(long)]
    no_fixbroken: bool,
    #[arg(long)]
    no_upgrade: bool,
    #[arg(long, short = 'y')]
    yes: bool,
    #[arg(long)]
    force_yes: bool,
}

#[derive(Parser, Debug)]
struct Update {
    packages: Vec<String>,
    #[arg(long, short = 'y')]
    yes: bool,
    #[arg(long)]
    force_yes: bool,
}

#[derive(Parser, Debug)]
struct Refresh {}

#[derive(Parser, Debug)]
struct ListFiles {
    package: String,
}

#[derive(Parser, Debug)]
struct Pick {
    package: String,
    #[arg(long)]
    no_fixbroken: bool,
    #[arg(long)]
    no_upgrade: bool,
}

#[derive(Parser, Debug)]
struct Provides {
    kw: String,
}

#[derive(Parser, Debug)]
struct Delete {
    packages: Vec<String>,
    #[arg(long, short = 'y')]
    yes: bool,
    #[arg(long)]
    force_yes: bool,
}

#[derive(Parser, Debug)]
struct Show {
    packages: Vec<String>,
    #[arg(long, short = 'a')]
    is_all: bool,
}

#[derive(Parser, Debug)]
struct Search {
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

    match args.subcommand {
        OmaCommand::Install(v) => {
            OmaAction::new()
                .await?
                .install(
                    &v.packages,
                    v.install_dbg,
                    v.reinstall,
                    v.no_fixbroken,
                    v.no_upgrade,
                    v.yes,
                    v.force_yes
                )
                .await
        }
        // TODO: 目前写死了删除的行为是 apt purge，以后会允许用户更改配置文件以更改删除行为
        OmaCommand::Remove(v) => OmaAction::new().await?.remove(&v.packages, true, v.yes, v.force_yes),
        OmaCommand::Upgrade(v) => OmaAction::new().await?.update(&v.packages, v.yes, v.force_yes).await,
        OmaCommand::Refresh(_) => OmaAction::new().await?.refresh().await,
        OmaCommand::Show(v) => OmaAction::show(&v.packages, v.is_all),
        OmaCommand::Search(v) => OmaAction::search(&v.keyword.join(" ")),
        // TODO: up_db 的值目前写死了 true，打算实现的逻辑是这样：
        // 如果用户在终端里执行了不存在的程序，会调用 oma 找可能的软件包
        // 这时候就不去更新 Contents 数据，若用户手动调用 oma provides/list-files
        // 则强制更新 Contents
        OmaCommand::ListFiles(v) => OmaAction::list_files(&v.package),
        OmaCommand::Provides(v) => OmaAction::search_file(&v.kw),
        OmaCommand::Download(v) => OmaAction::new().await?.download(&v.packages).await,
        OmaCommand::FixBroken(_) => OmaAction::new().await?.fix_broken().await,
        OmaCommand::Pick(v) => {
            OmaAction::new()
                .await?
                .pick(&v.package, v.no_fixbroken, v.no_upgrade)
                .await
        }
        OmaCommand::Mark(v) => OmaAction::mark(v.action),
        OmaCommand::CommandNotFound(v) => OmaAction::command_not_found(&v.kw),
        OmaCommand::List(v) => {
            OmaAction::list(v.packages.as_deref(), v.all, v.installed, v.upgradable)
        }
        OmaCommand::Depends(v) => OmaAction::dep(&v.pkgs),
        OmaCommand::Clean(_) => OmaAction::clean(),
    }?;

    Ok(())
}
