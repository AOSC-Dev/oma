use std::{
    borrow::Cow,
    io::{stdout, Write},
    path::PathBuf,
};

use clap::Args;
use oma_pm::{
    apt::{AptConfig, OmaApt, OmaAptArgs},
    matches::PackagesMatcher,
};
use oma_utils::dpkg::dpkg_arch;

use crate::{config::Config, error::OutputError};

use super::utils::{check_unsupported_stmt, handle_no_result};

use crate::args::CliExecuter;

#[derive(Debug, Args)]
pub struct Depends {
    /// Package(s) to query dependency(ies) for
    #[arg(required = true)]
    packages: Vec<String>,
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

impl CliExecuter for Depends {
    fn execute(self, _config: &Config, no_progress: bool) -> Result<i32, OutputError> {
        let Depends {
            packages,
            json,
            sysroot,
            apt_options,
        } = self;

        for pkg in &packages {
            check_unsupported_stmt(pkg);
        }

        let apt_config = AptConfig::new();
        let oma_apt_args = OmaAptArgs::builder()
            .sysroot(sysroot.to_string_lossy().to_string())
            .another_apt_options(apt_options)
            .build();
        let apt = OmaApt::new(vec![], oma_apt_args, false, apt_config)?;

        let arch = dpkg_arch(&sysroot)?;
        let matcher = PackagesMatcher::builder()
            .cache(&apt.cache)
            .native_arch(&arch)
            .build();

        let (pkgs, no_result) =
            matcher.match_pkgs_and_versions(packages.iter().map(|x| x.as_str()))?;

        handle_no_result(sysroot, no_result, no_progress)?;

        if !json {
            for pkg in pkgs {
                println!("{}:", pkg.raw_pkg.fullname(true));
                let all_deps = pkg.get_deps(&apt.cache)?;

                for (k, v) in all_deps {
                    for dep in v.inner() {
                        for b_dep in dep {
                            let s = if let Some(comp_ver) = b_dep.comp_ver {
                                Cow::Owned(format!("({comp_ver})"))
                            } else {
                                Cow::Borrowed("")
                            };

                            println!("  {k}: {} {}", b_dep.name, s);
                        }
                    }
                }
            }
        } else {
            let mut stdout = stdout();
            for pkg in pkgs {
                writeln!(
                    stdout,
                    "{}",
                    serde_json::json!({
                        "name": pkg.raw_pkg.fullname(true),
                        "deps": pkg.get_deps(&apt.cache)?,
                    })
                )
                .ok();
            }
        }

        Ok(0)
    }
}
