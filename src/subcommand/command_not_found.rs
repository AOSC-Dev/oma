use std::path::Path;

use oma_console::error;
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

            for (k, v) in res {
                let (pkg, bin_path) = v.split_once(':').unwrap();
                let bin_path = bin_path.trim();
                let pkg = apt.cache.get(pkg);

                let desc = pkg
                    .unwrap()
                    .candidate()
                    .and_then(|x| x.description())
                    .unwrap_or_else(|| "no description.".to_string());

                println!("{k} ({bin_path}): {desc}");
            }
        }
        Err(e) => {
            if !matches!(e, OmaContentsError::NoResult) {
                error!("{}", OutputError::from(e).to_string());
            }
            error!("{}", fl!("command-not-found", kw = pkg));
        }
    }

    Ok(127)
}
