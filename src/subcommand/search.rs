use std::{fmt::Display, path::PathBuf};

use clap::{ArgAction, Args};
use clap_complete::ArgValueCompleter;
use oma_console::{console::style, pager::Pager, print::Action, terminal::gen_prefix};
use oma_pm::{
    PackageStatus,
    apt::{AptConfig, OmaApt, OmaAptArgs},
    matches::SearchEngine,
    search::{IndiciumSearch, OmaSearch, SearchResult, StrSimSearch, TextSearch},
};

use crate::{
    WRITER, color_formatter,
    config::SearchEngine as ConfigSearchEngine,
    fl,
    utils::{ExitHandle, pkgnames_completions},
};
use crate::{config::Config, error::OutputError, table::oma_display_with_normal_output};

use crate::args::CliExecuter;

use super::utils::create_progress_spinner;

#[derive(Debug, Args)]
pub struct Search {
    /// Keywords to search
    #[arg(required = true, action = ArgAction::Append, add = ArgValueCompleter::new(pkgnames_completions), help = fl!("clap-search-pattern-help"))]
    #[arg(help_heading = &**crate::args::ARG_HELP_HEADING_MUST)]
    pattern: Vec<String>,
    /// Output result to stdout, not pager
    #[arg(long, help = fl!("clap-no-pager-help"))]
    no_pager: bool,
    /// Set output format as JSON
    #[arg(long, help = fl!("clap-json-help"))]
    json: bool,
    /// Set sysroot target directory
    #[arg(from_global, help = fl!("clap-sysroot-help"))]
    sysroot: PathBuf,
    /// Set apt options
    #[arg(from_global, help = fl!("clap-apt-options-help"))]
    apt_options: Vec<String>,
}

pub struct SearchResultDisplay<'a>(pub &'a SearchResult);

impl Display for SearchResultDisplay<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let i = self.0;
        let mut pkg_info_line = if i.is_base {
            color_formatter()
                .color_str(&i.name, Action::Purple)
                .bold()
                .to_string()
        } else {
            color_formatter()
                .color_str(&i.name, Action::Emphasis)
                .bold()
                .to_string()
        };

        pkg_info_line.push(' ');

        if i.status == PackageStatus::Upgrade {
            pkg_info_line.push_str(&format!(
                "{} -> {}",
                color_formatter().color_str(i.old_version.as_ref().unwrap(), Action::WARN),
                color_formatter().color_str(&i.new_version, Action::EmphasisSecondary)
            ));
        } else {
            pkg_info_line.push_str(
                &color_formatter()
                    .color_str(&i.new_version, Action::EmphasisSecondary)
                    .to_string(),
            );
        }

        let mut pkg_tags = vec![];

        if i.dbg_package {
            pkg_tags.push(fl!("debug-symbol-available"));
        }

        if i.full_match {
            pkg_tags.push(fl!("full-match"))
        }

        if !pkg_tags.is_empty() {
            pkg_info_line.push(' ');
            pkg_info_line.push_str(
                &color_formatter()
                    .color_str(format!("[{}]", pkg_tags.join(",")), Action::Note)
                    .to_string(),
            );
        }

        let prefix = match i.status {
            PackageStatus::Avail => style("AVAIL").dim(),
            PackageStatus::Installed => {
                color_formatter().color_str("INSTALLED", Action::Foreground)
            }
            PackageStatus::Upgrade => color_formatter().color_str("UPGRADE", Action::WARN),
        }
        .to_string();

        writeln!(f, "{}{}", gen_prefix(&prefix, 10), pkg_info_line)?;

        WRITER
            .get_terminal()
            .wrap_content("", &i.desc)
            .into_iter()
            .for_each(|(prefix, body)| {
                write!(f, "{}", gen_prefix(prefix, 10)).ok();
                writeln!(
                    f,
                    "{}",
                    color_formatter().color_str(body.trim(), Action::Secondary)
                )
                // Keep original behavior
                .ok();
            });

        Ok(())
    }
}

impl CliExecuter for Search {
    fn execute(self, config: &Config, no_progress: bool) -> Result<ExitHandle, OutputError> {
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
            match config.search_engine() {
                ConfigSearchEngine::Indicium => SearchEngine::Indicium(Box::new(|_| {})),
                ConfigSearchEngine::StrSim => SearchEngine::Strsim,
                ConfigSearchEngine::Text => SearchEngine::Text,
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

        Ok(ExitHandle::default().status(exit.into()))
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
