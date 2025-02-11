use std::path::PathBuf;

use clap::{ArgAction, Args};
use oma_console::pager::Pager;
use oma_pm::{
    apt::{AptConfig, OmaApt, OmaAptArgs},
    matches::SearchEngine,
    search::{IndiciumSearch, OmaSearch, SearchResult, StrSimSearch, TextSearch},
};
use tracing::warn;

use crate::{config::Config, error::OutputError, table::oma_display_with_normal_output};
use crate::{fl, utils::SearchResultDisplay};

use crate::args::CliExecuter;

use super::utils::create_progress_spinner;

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

        let pb = create_progress_spinner(no_progress || json, fl!("searching"));
        let res = search(
            &apt,
            &pattern,
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
            pb.inner.finish_and_clear();
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
    keywords: &[String],
    engine: SearchEngine,
) -> Result<Vec<SearchResult>, OutputError> {
    match engine {
        SearchEngine::Indicium(f) => {
            let searcher = IndiciumSearch::new(&apt.cache, f)?;
            Ok(searcher.search(&keywords.join(" "))?)
        }
        SearchEngine::Strsim => {
            let searcher = StrSimSearch::new(&apt.cache);
            Ok(searcher.search(&keywords.join(" "))?)
        }
        SearchEngine::Text => {
            let searcher = TextSearch::new(&apt.cache);
            let mut result = vec![];
            for keyword in keywords {
                let res = searcher.search(keyword)?;
                result.extend(res);
            }

            if keywords.len() > 1 {
                result.sort_by(|a, b| b.status.cmp(&a.status));
                result.sort_by(|a, b| b.full_match.cmp(&a.full_match));
            }

            Ok(result)
        }
    }
}
