use std::error::Error;
use std::io::stdout;

use oma_console::due_to;
use oma_console::print::Action;
use oma_contents::searcher::{pure_search, ripgrep_search, Mode};
use oma_contents::OmaContentsError;
use oma_pm::apt::{OmaApt, OmaAptArgsBuilder};
use oma_pm::format_description;
use tracing::error;

use crate::error::OutputError;
use crate::table::PagerPrinter;
use crate::{color_formatter, fl};

const FILTER_JARO_NUM: u8 = 204;
const APT_LIST_PATH: &str = "/var/lib/apt/lists";

pub fn execute(query: &str) -> Result<i32, OutputError> {
    let res = if which::which("rg").is_ok() {
        ripgrep_search(APT_LIST_PATH, Mode::BinProvides, query, |_| {})
    } else {
        pure_search(APT_LIST_PATH, Mode::BinProvides, query, |_| {})
    };

    match res {
        Ok(res) if res.is_empty() => {
            error!("{}", fl!("command-not-found", kw = query));
        }
        Ok(provides_res) => {
            let oma_apt_args = OmaAptArgsBuilder::default().build()?;
            let apt = OmaApt::new(vec![], oma_apt_args, false)?;

            let mut res = vec![];

            Some(()).and_then(|_| {
                for (pkg, file) in provides_res {
                    let num = strsim::jaro_winkler(query, file.split('/').next_back()?);

                    if num < JARO_NUM {
                        continue;
                    }

                    let pkg = apt.cache.get(&pkg)?;
                    let desc = pkg
                        .candidate()
                        .and_then(|x| {
                            x.description()
                                .map(|x| format_description(&x).0.to_string())
                        })
                        .unwrap_or_else(|| "no description.".to_string());

                    let entry = (
                        color_formatter()
                            .color_str(pkg.name(), Action::Emphasis)
                            .bold()
                            .to_string(),
                        color_formatter()
                            .color_str(file, Action::Secondary)
                            .to_string(),
                        desc,
                    );

                    if num == 1.0 {
                        res = vec![entry];
                        return Some(());
                    }

                    res.push(entry);
                }

                Some(())
            });

            if res.is_empty() {
                error!("{}", fl!("command-not-found", kw = query));
            } else {
                println!("{}\n", fl!("command-not-found-with-result", kw = query));
                let mut printer = PagerPrinter::new(stdout());
                printer
                    .print_table(res, vec!["Name", "Path", "Description"])
                    .ok();
            }
        }
        Err(e) => {
            if let OmaContentsError::NoResult = e {
                error!("{}", fl!("command-not-found", kw = query));
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
