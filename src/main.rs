use std::{process::exit, sync::atomic::AtomicI32};

use action::OmaAction;
use anyhow::Result;
use clap::{Parser, Subcommand};
use lazy_static::lazy_static;

mod action;
mod checksum;
mod cli;
mod contents;
mod db;
mod download;
mod formatter;
mod pager;
mod search;
mod utils;
mod verify;

static SUBPROCESS: AtomicI32 = AtomicI32::new(-1);

lazy_static! {
    static ref WRITER: cli::Writer = cli::Writer::new();
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
    #[clap(alias = "delete")]
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
}

#[derive(Parser, Debug)]
struct Update {
    packages: Vec<String>,
}

#[derive(Parser, Debug)]
struct Refresh {}

#[derive(Parser, Debug)]
struct ListFiles {
    package: String,
}

#[derive(Parser, Debug)]
struct Provides {
    kw: String,
}

#[derive(Parser, Debug)]
struct Delete {
    packages: Vec<String>,
}

#[derive(Parser, Debug)]
struct Show {
    packages: Vec<String>,
}

#[derive(Parser, Debug)]
struct Search {
    keyword: String,
}

#[tokio::main]
async fn main() {
    if let Err(e) = try_main().await {
        error!("{e}");
        exit(1);
    }

    exit(0);
}

async fn try_main() -> Result<()> {
    let args = Args::parse();

    match args.subcommand {
        OmaCommand::Install(v) => OmaAction::new().await?.install(&v.packages).await,
        // TODO: 目前写死了删除的行为是 apt purge，以后会允许用户更改配置文件以更改删除行为
        OmaCommand::Remove(v) => OmaAction::new().await?.remove(&v.packages, true),
        OmaCommand::Upgrade(v) => OmaAction::new().await?.update(&v.packages).await,
        OmaCommand::Refresh(_) => OmaAction::new().await?.refresh().await,
        OmaCommand::Show(v) => OmaAction::show(&v.packages),
        OmaCommand::Search(v) => OmaAction::search(&v.keyword),
        OmaCommand::ListFiles(v) => OmaAction::new().await?.list_files(&v.package).await,
        OmaCommand::Provides(v) => OmaAction::new().await?.search_file(&v.kw).await,
        OmaCommand::Download(v) => OmaAction::new().await?.download(&v.packages).await,
        OmaCommand::FixBroken(_) => OmaAction::new().await?.fix_broken().await,
    }
}
