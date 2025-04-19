use std::{io::stdout, path::PathBuf};

use clap::Args;
use oma_pm::{
    apt::{AptConfig, OmaApt, OmaAptArgs},
    matches::{GetArchMethod, PackagesMatcher},
    pkginfo::OmaPackage,
};
use tracing::info;

use crate::{config::Config, error::OutputError};

use super::utils::handle_no_result;
use crate::args::CliExecuter;
use crate::fl;

use std::io::Write;

#[derive(Debug, Args)]
pub struct Show {
    /// how information on all available version(s) of (a) package(s) from all repository(ies)
    #[arg(short, long)]
    all: bool,
    /// Set output format as JSON
    #[arg(long)]
    json: bool,
    /// Package(s) to show
    #[arg(required = true)]
    packages: Vec<String>,
    /// Set sysroot target directory
    #[arg(from_global)]
    sysroot: PathBuf,
    /// Set apt options
    #[arg(from_global)]
    apt_options: Vec<String>,
}

impl CliExecuter for Show {
    fn execute(self, _config: &Config, no_progress: bool) -> Result<i32, OutputError> {
        let Show {
            all,
            json,
            packages,
            sysroot,
            apt_options,
        } = self;

        let oma_apt_args = OmaAptArgs::builder()
            .another_apt_options(apt_options)
            .sysroot(sysroot.to_string_lossy().to_string())
            .build();

        let apt = OmaApt::new(vec![], oma_apt_args, false, AptConfig::new())?;

        let matcher = PackagesMatcher::builder()
            .cache(&apt.cache)
            .native_arch(GetArchMethod::SpecifySysroot(&sysroot))
            .filter_candidate(false)
            .build();

        let (pkgs, no_result) =
            matcher.match_pkgs_and_versions(packages.iter().map(|x| x.as_str()))?;

        handle_no_result(&sysroot, no_result, no_progress)?;

        let mut stdout = stdout();

        if !all {
            let mut filter_pkgs: Vec<OmaPackage> = vec![];
            let pkgs_len = pkgs.len();
            for pkg in pkgs {
                if !filter_pkgs
                    .iter()
                    .any(|x| pkg.raw_pkg.fullname(true) == x.raw_pkg.fullname(true))
                {
                    filter_pkgs.push(pkg);
                }
            }

            if filter_pkgs.is_empty() {
                return Ok(1);
            }

            for (i, pkg) in filter_pkgs.iter().enumerate() {
                if json {
                    writeln!(
                        stdout,
                        "{}",
                        serde_json::to_string(&pkg.pkg_info(&apt.cache)?).map_err(|e| {
                            OutputError {
                                description: e.to_string(),
                                source: None,
                            }
                        })?
                    )
                    .ok();
                } else {
                    writeln!(stdout, "{}", pkg.pkg_info(&apt.cache)?).ok();
                    if i != filter_pkgs.len() - 1 {
                        writeln!(stdout).ok();
                    }
                }
            }

            if filter_pkgs.len() == 1 && !json {
                let other_version = pkgs_len - 1;

                if other_version > 0 {
                    info!("{}", fl!("additional-version", len = other_version));
                }
            }
        } else {
            for (i, pkg) in pkgs.iter().enumerate() {
                if json {
                    writeln!(
                        stdout,
                        "{}",
                        serde_json::to_string(&pkg.pkg_info(&apt.cache)?).map_err(|e| {
                            OutputError {
                                description: e.to_string(),
                                source: None,
                            }
                        })?
                    )
                    .ok();
                } else if i != pkgs.len() - 1 {
                    writeln!(stdout, "{}", pkg.pkg_info(&apt.cache)?).ok();
                    writeln!(stdout).ok();
                } else {
                    writeln!(stdout, "{}", pkg.pkg_info(&apt.cache)?).ok();
                }
            }
        }

        Ok(0)
    }
}
