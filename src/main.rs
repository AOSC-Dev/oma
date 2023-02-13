use std::{process::exit, sync::atomic::AtomicI32};

use action::OmaAction;
use clap::{Parser, Subcommand};
use lazy_static::lazy_static;
use anyhow::Result;

mod action;
mod checksum;
mod cli;
mod db;
mod download;
mod formatter;
mod pager;
mod pkgversion;
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
    subcommand: AoscptCommand,
}

#[derive(Subcommand, Debug)]
enum AoscptCommand {
    /// Install Package
    Install(Install),
    /// Update Package
    #[clap(alias = "update", alias = "full-upgrade", alias = "dist-upgrade")]
    Upgrade(Update),
    /// Delete Package
    Remove(Delete),
    /// Refresh Package database
    Refresh(Refresh),
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
struct Delete {
    packages: Vec<String>,
}

#[tokio::main]
async fn main() {
    env_logger::init();

    if let Err(e) = try_main().await {
        error!("{e}");
    }

    exit(0);
}

async fn try_main() -> Result<()> {
    let app = OmaAction::new().await?;
    let args = Args::parse();

    match args.subcommand {
        AoscptCommand::Install(v) => app.install(&v.packages).await,
        // TODO: 目前写死了删除的行为是 apt purge，以后会允许用户更改配置文件以更改删除行为
        AoscptCommand::Remove(v) => app.remove(&v.packages, true),
        AoscptCommand::Upgrade(v) => app.update(&v.packages).await,
        AoscptCommand::Refresh(_) => app.refresh().await,
    }
}
