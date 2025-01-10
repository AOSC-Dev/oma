use std::path::PathBuf;

use clap::{ArgAction, Args};
use oma_console::{indicatif::ProgressBar, pager::Pager, pb::spinner_style};
use oma_pm::{
    apt::{AptConfig, OmaApt, OmaAptArgs},
    matches::SearchEngine,
    search::{IndiciumSearch, OmaSearch, SearchResult, StrSimSearch, TextSearch},
};
use tracing::warn;

use crate::{config::Config, error::OutputError, table::oma_display_with_normal_output};
use crate::{fl, utils::SearchResultDisplay};

use crate::args::CliExecuter;

#[derive(Debug, Args)]
pub struct Search {
    /// Keywords to search
    #[arg(required = true, action = ArgAction::Append)]
    pattern: Vec<String>,
    /// Output result to stdout, not pager
    #[arg(long)]
    no_pager: bool,
    /// Set output format as JSON
    #[arg(long)]
    json: bool,
    /// Set sysroot target directory
    #[arg(from_global)]
    sysroot: PathBuf,
    /// Set apt options
    #[arg(from_global)]
    apt_options: Vec<String>,
}

impl CliExecuter for Search {
    fn execute(self, config: &Config, no_progress: bool) -> Result<i32, OutputError> {
        let Search {
            pattern,
            no_pager,
            json,
            sysroot,
            apt_options,
        } = self;

        let no_pager = no_pager || config.search_contents_println();

        let oma_apt_args = OmaAptArgs::builder()
            .another_apt_options(apt_options)
            .sysroot(sysroot.to_string_lossy().to_string())
            .build();

        let apt = OmaApt::new(vec![], oma_apt_args, false, AptConfig::new())?;

        let (sty, inv) = spinner_style();

        let pb = if !no_progress && !json {
            let pb = ProgressBar::new_spinner().with_style(sty);
            pb.enable_steady_tick(inv);
            pb.set_message(fl!("searching"));

            Some(pb)
        } else {
            None
        };

        let res = search(
            &apt,
            &pattern.join(" "),
            match config.search_engine().as_str() {
                "indicium" => SearchEngine::Indicium(Box::new(|_| {})),
                "strsim" => SearchEngine::Strsim,
                "text" => SearchEngine::Text,
                x => {
                    warn!("Unsupported mode: {x}, fallback to indicium ...");
                    SearchEngine::Indicium(Box::new(|_| {}))
                }
            },
        )?;

        if let Some(pb) = pb {
            pb.finish_and_clear();
        }

        let mut pager = if !no_pager && !json {
            oma_display_with_normal_output(false, res.len() * 2)?
        } else {
            Pager::plain()
        };

        let mut writer = pager.get_writer().map_err(|e| OutputError {
            description: "Failed to get writer".to_string(),
            source: Some(Box::new(e)),
        })?;

        if !json {
            for i in res {
                write!(writer, "{}", SearchResultDisplay(&i)).ok();
            }
        } else {
            writeln!(
                writer,
                "{}",
                serde_json::to_string(&res).map_err(|e| OutputError {
                    description: e.to_string(),
                    source: None,
                })?
            )
            .ok();
        }

        drop(writer);
        let exit = pager.wait_for_exit().map_err(|e| OutputError {
            description: "Failed to wait exit".to_string(),
            source: Some(Box::new(e)),
        })?;

        Ok(exit.into())
    }
}

pub fn search(
    apt: &OmaApt,
    keyword: &str,
    engine: SearchEngine,
) -> Result<Vec<SearchResult>, OutputError> {
    let searcher: Box<dyn OmaSearch> = match engine {
        SearchEngine::Indicium(f) => Box::new(IndiciumSearch::new(&apt.cache, f)?),
        SearchEngine::Strsim => Box::new(StrSimSearch::new(&apt.cache)),
        SearchEngine::Text => Box::new(TextSearch::new(&apt.cache)),
    };

    let res = searcher.search(keyword)?;

    Ok(res)
}
