use std::borrow::Cow;
use std::error::Error;
use std::path::Path;

use oma_console::{error, due_to};
use oma_contents::{OmaContentsError, QueryMode};
use oma_pm::apt::{OmaApt, OmaAptArgsBuilder};
use oma_utils::dpkg::dpkg_arch;

use crate::error::OutputError;
use crate::fl;

pub fn execute(pkg: &str) -> Result<i32, OutputError> {
    let arch = dpkg_arch()?;
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
                let pkg = apt.cache.get(&pkg).unwrap();
                let desc = pkg
                    .candidate()
                    .and_then(|x| x.description().map(Cow::Owned))
                    .unwrap_or(Cow::Borrowed("no description."));

                println!("{} ({file}): {desc}", pkg.name());
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
