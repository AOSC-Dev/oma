use std::process::exit;

use action::AoscptAction;
use clap::{Parser, Subcommand};

mod action;
mod download;
mod formatter;
mod pkgversion;
mod update;
mod utils;
mod verify;

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
}

#[derive(Parser, Debug)]
struct Install {
    packages: Vec<String>,
}

#[derive(Parser, Debug)]
struct Update {}

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
    } {
        eprintln!("{e}");
        exit(1);
    }

    exit(0);
}
