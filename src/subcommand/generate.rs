use std::path::PathBuf;

use clap::{Args, Command, CommandFactory};

use clap_mangen::Man;

use crate::{
    args::{CliExecuter, OhManagerAilurus},
    config::Config,
    error::OutputError,
};

#[derive(Debug, Args)]
pub struct GenerateManpages {
    #[arg(short, long, default_value = ".")]
    path: PathBuf,
}

impl CliExecuter for GenerateManpages {
    fn execute(self, _config: &Config, _no_progress: bool) -> Result<i32, OutputError> {
        let cmd = OhManagerAilurus::command();
        Ok(build_man(&cmd, self.path)?)
    }
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
