mod key_binding;
mod pkg;
mod state;
mod tui;

use std::io::stdout;
use std::time::Duration;

use anyhow::anyhow;
use clap::Args;
use dialoguer::console::style;
use oma_console::indicatif::HumanBytes;
use oma_console::pager::{exit_tui, prepare_create_tui};
use oma_pm::apt::{AptConfig, OmaApt, OmaAptArgs};
use oma_pm::oma_apt::{Package, PackageSort};
use oma_pm::pkginfo::OmaPackageWithoutVersion;
use oma_utils::is_termux;

use spdlog::info;
use std::io::Write;
use tabled::builder::Builder;
use tabled::settings::{Alignment, Settings};

use crate::config::OmaConfig;
use crate::core::commit_changes::CommitChanges;
use crate::exit_handle::ExitHandle;
use crate::root::is_root;
use crate::subcommand::size_analyzer::pkg::PkgWrapper;
use crate::subcommand::size_analyzer::tui::PkgSizeAnalyzer;
use crate::subcommand::utils::auth_config;
use crate::table::table_no_color;
use crate::utils::dbus_check;
use crate::{CliExecuter, error::OutputError};
use crate::{NO_COLOR, fl};

#[derive(Debug, Args)]
pub struct SizeAnalyzer {
    /// Only display packages size details
    #[arg(short, long, help = fl!("clap-size-analyzer-details-help"))]
    details: bool,
    /// Resolve broken dependencies in the system
    #[arg(short, long, help = fl!("clap-fix-broken-help"))]
    fix_broken: bool,
    /// Do not fix dpkg broken status
    #[arg(short, long, help = fl!("clap-no-fix-dpkg-status-help"))]
    no_fix_dpkg_status: bool,
    /// Install package(s) without fsync(2)
    #[arg(
        long,
        help = &**crate::args::FORCE_UNSAFE_IO_TRANSLATE
    )]
    force_unsafe_io: bool,
    /// Ignore repository and package dependency issues
    #[arg(long, help = fl!("clap-force-yes-help"))]
    force_yes: bool,
    /// Replace configuration file(s) in the system those shipped in the package(s) to be installed (invokes `dpkg --force-confnew`)
    #[arg(long, help = fl!("clap-force-confnew-help"))]
    force_confnew: bool,
    /// Do not auto remove unnecessary package(s)
    #[arg(long, help = fl!("clap-no-autoremove-help"))]
    no_autoremove: bool,
    /// Remove package(s) also remove configuration file(s), like apt purge
    #[arg(long, visible_alias = "purge", help = fl!("clap-remove-config-help"))]
    remove_config: bool,
}

impl CliExecuter for SizeAnalyzer {
    fn execute(self, config: OmaConfig) -> Result<ExitHandle, OutputError> {
        let SizeAnalyzer {
            fix_broken,
            no_fix_dpkg_status,
            force_unsafe_io,
            force_yes,
            force_confnew,
            no_autoremove,
            remove_config,
            details,
        } = self;

        let detail = if !is_root() && !is_termux() {
            false
        } else {
            !details
        };

        let mut apt = OmaApt::new(
            vec![],
            OmaAptArgs::builder()
                .another_apt_options(config.apt_options.clone())
                .dpkg_force_unsafe_io(force_unsafe_io)
                .force_yes(force_yes)
                .dpkg_force_confnew(force_confnew)
                .build(),
            false,
            AptConfig::new(),
        )?;

        let mut exit_code = ExitHandle::default();

        if !detail {
            let installed = installed_packages(&apt, true)
                .unwrap()
                .into_iter()
                .map(|pkg| PkgWrapper { pkg })
                .collect::<Vec<_>>();

            let total_size = get_total_installed_size(&installed);

            let res = installed.iter().map(|pkg| pkg.to_table_line(total_size));

            let mut table = Builder::default();
            res.for_each(|r| table.push_record([r.0, r.1, r.3]));

            table.push_record([
                style(HumanBytes(total_size)).green().to_string(),
                "100%".to_string(),
                fl!("psa-total"),
            ]);

            let table_settings = Settings::default().with(tabled::settings::style::Style::blank());

            let mut table = table.build();
            table.with(table_settings).modify(
                tabled::settings::object::Columns::new(..1),
                Alignment::right(),
            );

            if NO_COLOR.load(std::sync::atomic::Ordering::Relaxed) {
                table_no_color(&mut table);
            }

            writeln!(stdout(), "{table}").ok();
            writeln!(stdout()).ok();
            info!("{}", fl!("psa-without-root-tips"));
        } else {
            let _fds = dbus_check(false, &config)?;

            let tui = PkgSizeAnalyzer::new(&apt);
            let mut terminal =
                prepare_create_tui().map_err(|e| anyhow!("Failed to create terminal: {e}"))?;

            let remove_pkgs = tui
                .run(&mut terminal, Duration::from_millis(250))
                .map_err(|e| anyhow!("{e}"))?;

            exit_tui(&mut terminal).map_err(|e| anyhow!("{e}"))?;

            if remove_pkgs.is_empty() {
                return Ok(exit_code);
            }

            apt.remove(
                remove_pkgs
                    .into_iter()
                    .map(|p| OmaPackageWithoutVersion {
                        raw_pkg: unsafe { p.pkg.unique() },
                    })
                    .collect::<Vec<_>>(),
                remove_config,
                no_autoremove,
            )?;

            let auth_config = auth_config(&config.sysroot);
            let auth_config = auth_config.as_ref();

            exit_code = CommitChanges::builder()
                .apt(apt)
                .dry_run(config.dry_run)
                .no_fixbroken(!fix_broken)
                .no_progress(config.no_progress())
                .sysroot(config.sysroot.to_string_lossy().to_string())
                .protect_essential(config.protect_essentials)
                .yes(false)
                .remove_config(remove_config)
                .autoremove(!no_autoremove)
                .network_thread(config.download_threads)
                .maybe_auth_config(auth_config)
                .fix_dpkg_status(!no_fix_dpkg_status)
                .yn_mode(config.yn_mode)
                .build()
                .run()?;
        }

        Ok(exit_code)
    }
}

pub(crate) fn installed_packages(
    apt: &OmaApt,
    small_to_big: bool,
) -> Result<Vec<Package<'_>>, OutputError> {
    let mut installed_packages = apt
        .cache
        .packages(&PackageSort::default().installed())
        .collect::<Vec<_>>();

    if small_to_big {
        installed_packages.sort_unstable_by(|a, b| {
            a.installed()
                .unwrap()
                .installed_size()
                .cmp(&b.installed().unwrap().installed_size())
        });
    } else {
        installed_packages.sort_unstable_by(|a, b| {
            b.installed()
                .unwrap()
                .installed_size()
                .cmp(&a.installed().unwrap().installed_size())
        });
    }

    Ok(installed_packages)
}

fn get_total_installed_size(installed: &[PkgWrapper]) -> u64 {
    installed
        .iter()
        .map(|p| p.pkg.installed().unwrap().installed_size())
        .sum()
}
