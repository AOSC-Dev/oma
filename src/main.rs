use action::AoscptAction;
use clap::{Parser, Subcommand};
use rust_apt::{raw::util::raw::{apt_lock, apt_unlock}, util::apt_is_locked};

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

    // if !apt_is_locked() {
    //     apt_lock().unwrap();
    // } else {
    //     eprintln!("Error: Apt is locked!");
    // }

    let app = AoscptAction::new().unwrap();

    let args = Args::parse();

    if let Err(e) = match args.subcommand {
        AoscptCommand::Install(v) => app.install(&v.packages),
        AoscptCommand::Remove(v) => app.remove(&v.packages, true),
        AoscptCommand::Update(_) => app.update(),
    } {
        eprintln!("{e}");
    }

    // apt_unlock()
}
