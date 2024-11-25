use std::path::PathBuf;

use clap::{Args, Command, CommandFactory, Subcommand};

#[cfg(not(feature = "generate-completion"))]
use clap::builder::PossibleValue;

#[cfg(feature = "generate-completion")]
use clap_complete::Shell;
use clap_mangen::Man;

use crate::{
    args::{CliExecuter, OhManagerAilurus},
    config::Config,
    error::OutputError,
};

#[derive(Debug, Args)]
pub struct Generate {
    #[command(subcommand)]
    subcmd: Subcmd,
}

#[cfg(not(feature = "generate-completion"))]
#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub enum Shell {
    Bash,
    Fish,
}

#[cfg(not(feature = "generate-completion"))]
const BASH_COMPLETION: &str = include_str!("../../data/completions/oma.bash");

#[cfg(not(feature = "generate-completion"))]
const FISH_COMPLETION: &str = include_str!("../../data/completions/oma.fish");

#[cfg(not(feature = "generate-completion"))]
impl clap::ValueEnum for Shell {
    fn value_variants<'a>() -> &'a [Self] {
        &[Self::Bash, Self::Fish]
    }

    fn to_possible_value(&self) -> Option<PossibleValue> {
        Some(match self {
            Shell::Bash => PossibleValue::new("bash"),
            Shell::Fish => PossibleValue::new("fish"),
        })
    }
}

#[derive(Debug, Subcommand)]
pub enum Subcmd {
    Shell {
        #[arg(required = true)]
        shell: Shell,
    },
    Man {
        #[arg(short, long, default_value = ".")]
        path: PathBuf,
    },
}

impl CliExecuter for Generate {
    fn execute(self, _config: &Config, _no_progress: bool) -> Result<i32, OutputError> {
        let mut cmd = OhManagerAilurus::command();

        match self.subcmd {
            Subcmd::Shell { shell } => generate_shell(&mut cmd, shell),
            Subcmd::Man { path } => Ok(build_man(&cmd, path)?),
        }
    }
}

#[cfg(feature = "generate-completion")]
fn generate_shell(cmd: &mut Command, shell: Shell) -> Result<i32, OutputError> {
    clap_complete::generate(shell, cmd, "oma", &mut std::io::stdout());
    Ok(0)
}

#[cfg(not(feature = "generate-completion"))]
fn generate_shell(_cmd: &mut Command, shell: Shell) -> Result<i32, OutputError> {
    match shell {
        Shell::Bash => println!("{}", BASH_COMPLETION),
        Shell::Fish => println!("{}", FISH_COMPLETION),
    }

    Ok(0)
}

fn build_man(cmd: &Command, path: PathBuf) -> Result<i32, anyhow::Error> {
    let man = Man::new(cmd.clone());
    let mut buffer: Vec<u8> = Default::default();
    man.render(&mut buffer)?;

    let man_dir = path.join("man");
    if !man_dir.is_dir() {
        std::fs::create_dir_all(&man_dir)?;
    }

    std::fs::write(man_dir.join("oma.1"), buffer)?;

    for subcommand in cmd.get_subcommands() {
        if subcommand.is_hide_set() {
            continue;
        }

        let subcommand_name = format!("oma-{}", subcommand.get_name());
        let mut buffer: Vec<u8> = Default::default();

        let man = Man::new(subcommand.clone()).title(&subcommand_name);
        man.render(&mut buffer)?;

        std::fs::write(
            std::path::PathBuf::from(&man_dir).join(format!("{subcommand_name}.1")),
            buffer,
        )?;
    }

    Ok(0)
}
