use std::{
    borrow::Cow,
    io::{Write, stdout},
    path::PathBuf,
};

use clap::Args;
use clap_complete::ArgValueCompleter;
use oma_pm::{
    apt::{AptConfig, OmaApt, OmaAptArgs},
    matches::{GetArchMethod, PackagesMatcher},
};

use crate::{config::Config, error::OutputError, fl, utils::pkgnames_and_path_completions};

use super::utils::handle_no_result;

use crate::args::CliExecuter;

#[derive(Debug, Args)]
pub struct Depends {
    /// Package(s) to query dependency(ies) for
    #[arg(required = true, add = ArgValueCompleter::new(pkgnames_and_path_completions), help = fl!("clap-depends-packages-help"))]
    #[arg(help_heading = &**crate::args::ARG_HELP_HEADING_MUST)]
    packages: Vec<String>,
    /// Set output format as JSON
    #[arg(long, help = fl!("clap-json-help"))]
    json: bool,
    /// Set sysroot target directory
    #[arg(from_global, help = fl!("clap-sysroot-help"))]
    sysroot: PathBuf,
    /// Set apt options
    #[arg(from_global, help = fl!("clap-apt-options-help"))]
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

        let local_debs = packages
            .iter()
            .filter(|x| x.ends_with(".deb"))
            .map(|x| x.to_owned())
            .collect::<Vec<_>>();

        let apt_config = AptConfig::new();
        let oma_apt_args = OmaAptArgs::builder()
            .sysroot(sysroot.to_string_lossy().to_string())
            .another_apt_options(apt_options)
            .build();
        let apt = OmaApt::new(local_debs, oma_apt_args, false, apt_config)?;

        let matcher = PackagesMatcher::builder()
            .cache(&apt.cache)
            .native_arch(GetArchMethod::SpecifySysroot(&sysroot))
            .build();

        let (pkgs, no_result) =
            matcher.match_pkgs_and_versions(packages.iter().map(|x| x.as_str()))?;

        handle_no_result(no_result, no_progress)?;

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
