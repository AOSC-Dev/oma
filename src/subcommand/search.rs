use std::fmt::Display;

use clap::{ArgAction, Args};
use clap_complete::ArgValueCompleter;
use oma_apt_pkg::search::{
    IndiciumSearch, OmaSearch as _, PackageStatus, SearchResult, SearchType, StrSimSearch,
    TextSearch,
};
use oma_console::{console::style, pager::Pager, print::Action, terminal::gen_prefix};
use oma_pm::matches::SearchEngine;
use oma_pm::oma_apt::raw::config as apt_config;
use oma_utils::zbus::proxy;
use spdlog::debug;
use zbus::Connection;

use crate::{
    RT, WRITER, color_formatter, completions::pkgnames_completions, config::OmaConfig,
    config_file::SearchEngine as ConfigSearchEngine, exit_handle::ExitHandle, fl,
};
use crate::{error::OutputError, table::oma_display_with_normal_output};

use crate::args::CliExecuter;

use super::utils::create_progress_spinner;

#[proxy(
    interface = "io.aosc.Amo1",
    default_service = "io.aosc.Amo",
    default_path = "/io/aosc/Amo"
)]
pub trait Amo {
    async fn search(&self, query: &str) -> zbus::Result<String>;
}

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
    fn execute(self, config: OmaConfig) -> Result<ExitHandle, OutputError> {
        let Search {
            pattern,
            no_pager,
            json,
        } = self;

        let no_pager = no_pager || config.search_contents_println;

        let pb = create_progress_spinner(config.no_progress() || json, fl!("searching"));
        let res = search(
            &pattern,
            match config.search_engine {
                ConfigSearchEngine::Indicium => SearchEngine::Indicium(Box::new(|_| {})),
                ConfigSearchEngine::StrSim => SearchEngine::Strsim,
                ConfigSearchEngine::Text => SearchEngine::Text,
            },
            &config,
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
    keywords: &[String],
    engine: SearchEngine,
    config: &OmaConfig,
) -> Result<Vec<SearchResult>, OutputError> {
    match engine {
        SearchEngine::Indicium(f) => {
            let query = keywords.join(" ");

            if config.amo && !config.no_check_dbus {
                match RT.block_on(amo_search(&query)) {
                    Ok(r) => Ok(r),
                    Err(_) => local_indicium_search(f, query),
                }
            } else {
                local_indicium_search(f, query)
            }
        }
        SearchEngine::Strsim => {
            let (apt_db, dpkg) = load_apt_db_and_dpkg()?;
            let searcher = StrSimSearch::new(&apt_db, &dpkg);
            Ok(searcher
                .search(&keywords.join(" "))
                .map_err(to_output_err)?)
        }
        SearchEngine::Text => {
            let (apt_db, dpkg) = load_apt_db_and_dpkg()?;
            let searcher = TextSearch::new(&apt_db, &dpkg);
            let mut result = vec![];
            for keyword in keywords {
                let res = searcher.search(keyword).map_err(to_output_err)?;
                result.extend(res);
            }

            Ok(result)
        }
    }
}

fn to_output_err(e: impl std::fmt::Display) -> OutputError {
    OutputError {
        description: e.to_string(),
        source: None,
    }
}

fn load_apt_db_and_dpkg() -> Result<(oma_apt_pkg::AptDb, oma_apt_pkg::DpkgState), OutputError> {
    let lists_dir = apt_config::find_dir(
        "Dir::State::lists".to_string(),
        "var/lib/apt/lists".to_string(),
    );
    let dpkg_path = apt_config::find_file(
        "Dir::State::status".to_string(),
        "var/lib/dpkg/status".to_string(),
    );
    let apt_cache = crate::utils::get_apt_cache_path("Dir::Cache::oma-aptdb", "oma-aptdb.bincode");
    let _search_cache = crate::utils::get_apt_cache_path("Dir::Cache::oma-search", "oma-search.bincode");

    let apt_db =
        oma_apt_pkg::AptDb::load_or_build(&apt_cache, &lists_dir).map_err(|e| OutputError {
            description: e.to_string(),
            source: None,
        })?;
    let dpkg = oma_apt_pkg::DpkgState::from_file(&dpkg_path).map_err(|e| OutputError {
        description: e.to_string(),
        source: None,
    })?;
    Ok((apt_db, dpkg))
}

fn local_indicium_search(
    f: Box<dyn Fn(usize) + 'static>,
    query: String,
) -> Result<Vec<SearchResult>, OutputError> {
    let lists_dir = apt_config::find_dir(
        "Dir::State::lists".to_string(),
        "var/lib/apt/lists".to_string(),
    );
    let dpkg_path = apt_config::find_file(
        "Dir::State::status".to_string(),
        "var/lib/dpkg/status".to_string(),
    );
    let apt_cache = crate::utils::get_apt_cache_path("Dir::Cache::oma-aptdb", "oma-aptdb.bincode");
    let search_cache = crate::utils::get_apt_cache_path("Dir::Cache::oma-search", "oma-search.bincode");

    let searcher = IndiciumSearch::from_paths(
        &lists_dir,
        &dpkg_path,
        &apt_cache,
        &search_cache,
        SearchType::Live,
        f,
    )
    .map_err(|e| OutputError {
        description: e.to_string(),
        source: None,
    })?;

    searcher.search(&query).map_err(to_output_err)
}

async fn amo_search(query: &str) -> anyhow::Result<Vec<SearchResult>> {
    let connection = Connection::system().await?;

    let peer_proxy = zbus::fdo::PeerProxy::builder(&connection)
        .destination("io.aosc.Amo")?
        .path("/io/aosc/Amo")?
        .build()
        .await?;

    peer_proxy
        .ping()
        .await
        .inspect_err(|e| debug!("Failed to connect amo: {e}"))?;

    let proxy = AmoProxy::new(&connection).await?;
    let result = proxy.search(query).await?;
    let result: Vec<SearchResult> = serde_json::from_str(&result)?;

    Ok(result)
}
