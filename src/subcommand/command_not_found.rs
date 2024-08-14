use std::borrow::Cow;
use std::error::Error;
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

            for (pkg, file) in res {
                let pkg = apt.cache.get(&pkg);
                if pkg.is_none() {
                    continue;
                }
                let pkg = pkg.unwrap();
                let desc = pkg
                    .candidate()
                    .and_then(|x| {
                        x.description()
                            .map(|x| Cow::Owned(format_description(&x).0.to_string()))
                    })
                    .unwrap_or(Cow::Borrowed("no description."));

                println!(
                    "{} {}: {}",
                    color_format.color_str(pkg.name(), Action::Emphasis).bold(),
                    color_format.color_str(format!("({})", file), Action::Secondary),
                    color_format.color_str(desc, Action::EmphasisSecondary),
                );
            }
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
