use std::error::Error;
use std::path::Path;

use clap::Args;
use oma_contents::OmaContentsError;
use oma_contents::searcher::{Mode, search};
use tracing::error;

use crate::config::Config;
use crate::error::OutputError;
use crate::{due_to, fl};

use crate::args::CliExecuter;

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
                println!(
                    "{}",
                    fl!("command-not-found-with-result", kw = keyword.as_str())
                );

                let res = sort(res, &keyword);
                let res_len = res.len();

                let mut count = 0;

                for (pkg, file) in res.into_iter().map(|x| {
                    (
                        x.0,
                        Path::new(&x.1)
                            .file_name()
                            .map(|s| s.to_string_lossy().to_string())
                            .unwrap_or(x.1),
                    )
                }) {
                    if count > 10 {
                        break;
                    }

                    count += 1;
                    println!("  {}", fl!("cnf-entry", cmd = file, pkg = pkg));
                }

                if res_len > 10 {
                    println!("\n{}", fl!("cnf-too-many-query"));
                    println!("{}", fl!("cnf-too-many-query-2", query = keyword));
                }

                println!("{}", fl!("cnf-install"));
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

fn sort(input: IndexSet<(String, String)>, query: &str) -> Vec<(String, String, u8)> {
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
