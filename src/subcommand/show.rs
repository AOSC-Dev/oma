use std::{borrow::Cow, io::stdout};

use clap::Args;
use clap_complete::ArgValueCompleter;
use dialoguer::console::{StyledObject, style};
use oma_console::indicatif::HumanBytes;
use oma_pm::{
    apt::{AptConfig, OmaApt, OmaAptArgs},
    matches::{GetArchMethod, PackagesMatcher},
    oma_apt::records::RecordField,
    pkginfo::{AptSource, OmaPackage},
};
use spdlog::info;

use crate::{
    config::OmaConfig,
    error::OutputError,
    utils::{ExitHandle, pkgnames_and_path_completions},
};

use super::utils::handle_no_result;
use crate::args::CliExecuter;
use crate::fl;

use std::io::Write;

#[derive(Debug, Args)]
pub struct Show {
    /// Package(s) to show
    #[arg(required = true, add = ArgValueCompleter::new(pkgnames_and_path_completions), help = fl!("clap-show-packages-help"))]
    #[arg(help_heading = &**crate::args::ARG_HELP_HEADING_MUST)]
    packages: Vec<String>,
    /// Show information on all available version(s) of (a) package(s) from all repository(ies)
    #[arg(short, long, help = fl!("clap-show-all-help"))]
    all: bool,
    /// Set output format as JSON
    #[arg(long, help = fl!("clap-json-help"))]
    json: bool,
}

const RECORDS: &[&str] = &[
    RecordField::Package,
    RecordField::Version,
    RecordField::Section,
    RecordField::Maintainer,
    RecordField::InstalledSize,
    RecordField::PreDepends,
    RecordField::Depends,
    RecordField::Breaks,
    RecordField::Conflicts,
    RecordField::Replaces,
    RecordField::Recommends,
    RecordField::Suggests,
    RecordField::Provides,
    RecordField::Size,
    RecordField::Description,
];

impl CliExecuter for Show {
    fn execute(self, config: OmaConfig) -> Result<ExitHandle, OutputError> {
        let Show {
            all,
            json,
            packages,
        } = self;

        let oma_apt_args = OmaAptArgs::builder()
            .another_apt_options(config.apt_options.clone())
            .sysroot(config.sysroot.to_string_lossy().to_string())
            .build();

        let local_debs = packages
            .iter()
            .filter(|x| x.ends_with(".deb"))
            .map(|x| x.to_owned())
            .collect::<Vec<_>>();

        let apt = OmaApt::new(local_debs, oma_apt_args, false, AptConfig::new())?;

        let matcher = PackagesMatcher::builder()
            .cache(&apt.cache)
            .native_arch(GetArchMethod::SpecifySysroot(&config.sysroot))
            .filter_candidate(!all)
            .build();

        let (pkgs, no_result) =
            matcher.match_pkgs_and_versions(packages.iter().map(|x| x.as_str()))?;

        handle_no_result(no_result, config.no_progress())?;

        let mut stdout = stdout();

        if !all {
            for_each_show_package(json, &apt, &mut stdout, &pkgs)?;

            if pkgs.len() == 1 && !json {
                let pkg = &pkgs[0];
                let pkg = pkg.package(&apt.cache);
                let other_version_count = pkg.versions().count() - 1;

                if other_version_count > 0 {
                    info!("{}", fl!("additional-version", len = other_version_count));
                }
            }
        } else {
            for_each_show_package(json, &apt, &mut stdout, &pkgs)?;
        }

        Ok(ExitHandle::default())
    }
}

fn for_each_show_package(
    json: bool,
    apt: &OmaApt,
    stdout: &mut std::io::Stdout,
    pkgs: &[OmaPackage],
) -> Result<(), OutputError> {
    for (i, pkg) in pkgs.iter().enumerate() {
        if json {
            display_to_json(stdout, pkg, apt)?;
        } else {
            display_records(stdout, pkg, apt);
        }

        if i != pkgs.len() - 1 {
            writeln!(stdout).ok();
        }
    }

    Ok(())
}

fn display_to_json(
    stdout: &mut std::io::Stdout,
    pkg: &OmaPackage,
    apt: &OmaApt,
) -> Result<(), OutputError> {
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

    Ok(())
}

fn display_records(stdout: &mut std::io::Stdout, pkg: &OmaPackage, apt: &OmaApt) {
    let version = pkg.version(&apt.cache);
    for i in RECORDS {
        let Some(mut v) = version.get_record(i) else {
            continue;
        };

        if *i == RecordField::InstalledSize {
            v = HumanBytes(v.parse::<u64>().unwrap() * 1024).to_string();
        } else if *i == RecordField::Size {
            v = HumanBytes(v.parse().unwrap()).to_string();
        }

        let i = if *i == RecordField::Size {
            key_style(Cow::Borrowed("Download-Size:"))
        } else {
            key_style(format!("{i}:").into())
        };

        writeln!(stdout, "{i} {v}").ok();
    }

    let apt_sources = version
        .package_files()
        .map(AptSource::from)
        .collect::<Vec<_>>();

    write!(stdout, "{}", key_style("APT-Sources:".into())).ok();
    let apt_sources_without_dpkg = apt_sources
        .iter()
        .filter(|x| x.index_type.as_deref() != Some("Debian dpkg status file"))
        .collect::<Vec<_>>();

    match apt_sources_without_dpkg.len() {
        0 => {
            writeln!(stdout, " {}", &apt_sources[0]).ok();
        }
        1 => {
            writeln!(stdout, " {}", &apt_sources_without_dpkg[0]).ok();
        }
        2.. => {
            writeln!(stdout).ok();
            for i in apt_sources_without_dpkg {
                writeln!(stdout, "  {i}").ok();
            }
        }
    }

    if version.is_installed() {
        write!(stdout, "{}", key_style("APT-Manual-Installed: ".into())).ok();
        if version.parent().is_auto_installed() {
            writeln!(stdout, "no").ok();
        } else {
            writeln!(stdout, "yes").ok();
        }
    }
}

#[inline]
fn key_style(key: Cow<str>) -> StyledObject<Cow<str>> {
    style(key).bold()
}
