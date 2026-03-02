use std::{
    borrow::Cow,
    io::{Write, stdout},
};

use clap::Args;
use clap_complete::ArgValueCompleter;
use oma_pm::{
    apt::{AptConfig, OmaApt, OmaAptArgs},
    matches::{GetArchMethod, PackagesMatcher},
};

use crate::{
    config::OmaConfig,
    error::OutputError,
    fl,
    utils::{ExitHandle, pkgnames_and_path_completions},
};

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
}

impl CliExecuter for Depends {
    fn execute(self, config: OmaConfig) -> Result<ExitHandle, OutputError> {
        let Depends { packages, json } = self;

        let no_progress = config.no_progress();

        let OmaConfig {
            sysroot,
            apt_options,
            ..
        } = config;

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

        Ok(ExitHandle::default())
    }
}
