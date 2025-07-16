use std::error::Error;
use std::io::stderr;

use ahash::AHashMap;
use clap::Args;
use oma_console::print::Action;
use oma_contents::OmaContentsError;
use oma_contents::searcher::{Mode, search};
use oma_pm::apt::{AptConfig, OmaApt, OmaAptArgs};
use tracing::error;

use crate::config::Config;
use crate::error::OutputError;
use crate::table::PagerPrinter;
use crate::{color_formatter, due_to, fl};

use crate::args::CliExecuter;

const FILTER_JARO_NUM: u8 = 204;
const APT_LIST_PATH: &str = "/var/lib/apt/lists";

type IndexSet<T> = indexmap::IndexSet<T, ahash::RandomState>;

#[derive(Debug, Args)]
pub struct CommandNotFound {
    /// Package to query command-not-found
    #[arg(required = true)]
    keyword: String,
}

impl CliExecuter for CommandNotFound {
    fn execute(self, _config: &Config, _no_progress: bool) -> Result<i32, OutputError> {
        let CommandNotFound { keyword } = self;

        let mut res = IndexSet::with_hasher(ahash::RandomState::new());

        let cb = |line: (String, String)| {
            if !res.contains(&line) && line.1.starts_with("/usr/bin") {
                res.insert(line);
            }
        };

        let search_res = search(APT_LIST_PATH, Mode::BinProvides, &keyword, cb);

        match search_res {
            Ok(()) if res.is_empty() => {
                error!("{}", fl!("command-not-found", kw = keyword));
            }
            Ok(()) => {
                let apt_config = AptConfig::new();
                let oma_apt_args = OmaAptArgs::builder().build();
                let apt = OmaApt::new(vec![], oma_apt_args, false, apt_config)?;

                let mut jaro = jaro_nums(res, &keyword);

                let all_match = jaro
                    .iter()
                    .filter(|x| x.2 == u8::MAX)
                    .map(|x| x.to_owned())
                    .collect::<Vec<_>>();

                if !all_match.is_empty() {
                    jaro = all_match;
                }

                let mut res = vec![];

                let mut too_many = false;

                let mut map: AHashMap<String, String> = AHashMap::new();

                for (pkg, file, jaro) in jaro {
                    if res.len() == 10 {
                        too_many = true;
                        break;
                    }

                    if jaro < FILTER_JARO_NUM {
                        break;
                    }

                    let desc = if let Some(desc) = map.get(&pkg) {
                        desc.to_string()
                    } else if let Some(pkg) = apt.cache.get(&pkg) {
                        let desc = pkg
                            .candidate()
                            .and_then(|x| x.summary())
                            .unwrap_or_else(|| "no description.".to_string());

                        map.insert(pkg.fullname(true), desc.to_string());

                        desc
                    } else {
                        continue;
                    };

                    let entry = (
                        color_formatter()
                            .color_str(pkg, Action::Emphasis)
                            .bold()
                            .to_string(),
                        color_formatter()
                            .color_str(file, Action::Secondary)
                            .to_string(),
                        desc,
                    );

                    res.push(entry);
                }

                if res.is_empty() {
                    error!("{}", fl!("command-not-found", kw = keyword));
                } else {
                    eprintln!(
                        "{}\n",
                        fl!("command-not-found-with-result", kw = keyword.as_str())
                    );
                    let mut printer = PagerPrinter::new(stderr());
                    printer
                        .print_table(res, vec!["Name", "Path", "Description"], None)
                        .ok();

                    if too_many {
                        eprintln!("\n{}", fl!("cnf-too-many-query"));
                        eprintln!("{}", fl!("cnf-too-many-query-2", query = keyword));
                    }
                }
            }
            Err(e) => {
                if let OmaContentsError::NoResult = e {
                    error!("{}", fl!("command-not-found", kw = keyword));
                } else {
                    let err = OutputError::from(e);
                    if !err.to_string().is_empty() {
                        error!("{err}");
                        if let Some(source) = err.source() {
                            due_to!("{source}");
                        }
                    }
                }
            }
        }

        Ok(127)
    }
}

fn jaro_nums(input: IndexSet<(String, String)>, query: &str) -> Vec<(String, String, u8)> {
    let mut output = vec![];

    for (pkg, file) in input {
        if pkg == query {
            output.push((pkg, file, u8::MAX));
            continue;
        }

        let binary_name = file.split('/').next_back().unwrap_or(&file);

        if binary_name == query {
            output.push((pkg, file, u8::MAX));
            continue;
        }

        let num = (strsim::jaro_winkler(query, binary_name) * 255.0) as u8;

        output.push((pkg, file, num));
    }

    output.sort_unstable_by(|a, b| b.2.cmp(&a.2));

    output
}
