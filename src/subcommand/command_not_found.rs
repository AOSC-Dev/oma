use std::error::Error;
use std::io::stdout;
use std::path::Path;

use oma_console::due_to;
use oma_console::print::{Action, OmaColorFormat};
use oma_contents::{OmaContentsError, QueryMode};
use oma_pm::apt::{OmaApt, OmaAptArgsBuilder};
use oma_pm::format_description;
use oma_utils::dpkg::dpkg_arch;
use tracing::error;

use crate::error::OutputError;
use crate::fl;
use crate::table::PagerPrinter;

pub fn execute(pkg: &str, color_format: OmaColorFormat) -> Result<i32, OutputError> {
    let arch = dpkg_arch("/")?;
    let res = oma_contents::find(
        pkg,
        QueryMode::CommandNotFound,
        Path::new("/var/lib/apt/lists"),
        &arch,
        |_| {},
        arch != "mips64r6el",
    );

    match res {
        Ok(res) if res.is_empty() => {
            error!("{}", fl!("command-not-found", kw = pkg));
        }
        Ok(res) => {
            println!("{}\n", fl!("command-not-found-with-result", kw = pkg));

            let oma_apt_args = OmaAptArgsBuilder::default().build()?;
            let apt = OmaApt::new(vec![], oma_apt_args, false)?;

            let res = res.into_iter().filter_map(|(pkg, file)| {
                let pkg = apt.cache.get(&pkg)?;
                let desc = pkg
                    .candidate()
                    .and_then(|x| {
                        x.description()
                            .map(|x| format_description(&x).0.to_string())
                    })
                    .unwrap_or_else(|| "no description.".to_string());

                return Some((
                    color_format
                        .color_str(pkg.name(), Action::Emphasis)
                        .bold()
                        .to_string(),
                    color_format.color_str(file, Action::Secondary).to_string(),
                    desc,
                ));
            });

            let mut printer = PagerPrinter::new(stdout());
            printer
                .print_table(res, vec!["Name", "Path", "Description"])
                .ok();
        }
        Err(e) => {
            if !matches!(e, OmaContentsError::NoResult) {
                let err = OutputError::from(e);
                if !err.to_string().is_empty() {
                    error!("{err}");
                    if let Some(source) = err.source() {
                        due_to!("{source}");
                    }
                }
            }
            error!("{}", fl!("command-not-found", kw = pkg));
        }
    }

    Ok(127)
}
