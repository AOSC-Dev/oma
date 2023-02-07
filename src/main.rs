use std::process::exit;

use action::OmaAction;
use clap::{Parser, Subcommand};
use lazy_static::lazy_static;

mod action;
mod cli;
mod download;
mod formatter;
mod pkgversion;
mod update;
mod utils;
mod verify;
mod checksum;

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
struct Update {}

#[derive(Parser, Debug)]
struct Refresh {}

#[derive(Parser, Debug)]
struct Delete {
    packages: Vec<String>,
}

#[tokio::main]
async fn main() {
    env_logger::init();

    let app = OmaAction::new().await;

    let app = if let Ok(app) = app {
        app
    } else {
        error!("{:?}", app.err());
        exit(1);
    };

    let args = Args::parse();

    if let Err(e) = match args.subcommand {
        AoscptCommand::Install(v) => app.install(&v.packages).await,
        // TODO: 目前写死了删除的行为是 apt purge，以后会允许用户更改配置文件以更改删除行为
        AoscptCommand::Remove(v) => app.remove(&v.packages, true),
        AoscptCommand::Upgrade(_) => app.update().await,
        AoscptCommand::Refresh(_) => app.refresh().await,
    } {
        error!("{e}");
        exit(1);
    }

    exit(0);
}
