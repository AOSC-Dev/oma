use std::path::PathBuf;

use clap::Args;
use oma_pm::apt::{AptConfig, FilterMode, OmaApt, OmaAptArgs};

use crate::{config::Config, error::OutputError};

use crate::args::CliExecuter;

#[derive(Debug, Args)]
pub struct Pkgnames {
    /// Keyword to query
    keyword: Option<String>,
    /// Only query installed package
    #[arg(long)]
    installed: bool,
    /// Set sysroot target directory
    #[arg(from_global)]
    sysroot: PathBuf,
}

impl CliExecuter for Pkgnames {
    fn execute(self, _config: &Config, _no_progress: bool) -> Result<i32, OutputError> {
        let Pkgnames {
            keyword,
            installed,
            sysroot,
        } = self;

        let oma_apt_args = OmaAptArgs::builder()
            .sysroot(sysroot.to_string_lossy().to_string())
            .build();
        let apt = OmaApt::new(vec![], oma_apt_args, false, AptConfig::new())?;
        let mut modes = vec![FilterMode::Names];

        if installed {
            modes.push(FilterMode::Installed);
        }

        let mut pkgs: Box<dyn Iterator<Item = _>> = Box::new(apt.filter_pkgs(&modes)?);

        if let Some(keyword) = keyword {
            pkgs = Box::new(pkgs.filter(move |x| x.name().starts_with(&keyword)));
        }

        for pkg in pkgs {
            println!("{}", pkg.name());
        }

        Ok(0)
    }
}
