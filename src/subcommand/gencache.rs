use clap::Args;
use oma_pm::{
    apt::{AptConfig, OmaApt, OmaAptArgs},
    search::make_search_cache,
};

use crate::{args::CliExecuter, config::Config, error::OutputError};

#[derive(Debug, Args)]
pub struct GenCache;

impl CliExecuter for GenCache {
    fn execute(self, _config: &Config, _no_progress: bool) -> Result<i32, OutputError> {
        let apt = OmaApt::new(
            vec![],
            OmaAptArgs::builder().build(),
            false,
            AptConfig::new(),
        )?;
        make_search_cache(&apt.cache, |_| {})?;
        Ok(0)
    }
}
