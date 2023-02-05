use std::process::exit;

use action::AoscptAction;
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
    Update(Update),
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

fn main() {
    env_logger::init();

    let app = AoscptAction::new().unwrap();

    let args = Args::parse();

    if let Err(e) = match args.subcommand {
        AoscptCommand::Install(v) => app.install(&v.packages),
        AoscptCommand::Remove(v) => app.remove(&v.packages, true),
        AoscptCommand::Update(_) => app.update(),
        AoscptCommand::Refresh(_) => app.refresh(),
    } {
        eprintln!("{e}");
        exit(1);
    }

    exit(0);
}
