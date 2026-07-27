use clap::Args;
use clap_complete::ArgValueCompleter;
use dialoguer::{Select, theme::ColorfulTheme};
use oma_pm::{
    apt::{OmaApt, OmaAptArgs},
    oma_apt::VersionFile,
    pkginfo::OmaPackage,
};

use crate::{
    completions::pkgnames_completions,
    config::OmaConfig,
    core::operation_pipeline::Pipeline,
    error::OutputError,
    exit_handle::ExitHandle,
    menu::tui_select_list_size,
};
use crate::fl;
use anyhow::anyhow;

use crate::args::CliExecuter;

#[derive(Debug, Args)]
pub struct Pick {
    /// Package to pick specific version for
    #[arg(required = true, add = ArgValueCompleter::new(pkgnames_completions), help = fl!("clap-pick-package-help"))]
    #[arg(help_heading = &**crate::args::ARG_HELP_HEADING_MUST)]
    package: String,
    /// Fix apt broken status
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
    /// Do not refresh repository metadata
    #[arg(long, help = fl!("clap-no-refresh-help"))]
    no_refresh: bool,
    /// Ignore repository and package dependency issues
    #[arg(long, help = fl!("clap-force-yes-help"))]
    force_yes: bool,
    /// Replace configuration file(s) in the system those shipped in the package(s) to be installed (invokes `dpkg --force-confnew`)
    #[arg(long, help = fl!("clap-force-confnew-help"))]
    force_confnew: bool,
    /// Auto remove unnecessary package(s)
    #[arg(long, help = fl!("clap-autoremove-help"))]
    autoremove: bool,
    /// Remove package(s) also remove configuration file(s), like apt purge
    #[arg(long, visible_alias = "purge", help = fl!("clap-remove-config-help"))]
    remove_config: bool,
    /// Only download dependencies, not install
    #[arg(long, short, help = fl!("clap-download-only-help"))]
    download_only: bool,
    /// Do not clean local package cache
    #[arg(long, help = fl!("clap-noclean-help"), env = "OMA_NO_CLEAN", value_parser = clap::builder::FalseyValueParser::new())]
    no_clean: bool,
}

impl CliExecuter for Pick {
    fn execute(self, config: OmaConfig) -> Result<ExitHandle, OutputError> {
        let Pick {
            package,
            fix_broken,
            force_unsafe_io,
            no_refresh,
            force_yes,
            force_confnew,
            autoremove,
            remove_config,
            no_fix_dpkg_status,
            download_only,
            no_clean,
        } = self;

        Pipeline::builder()
            .config(&config)
            .need_refresh(!no_refresh)
            .no_fixbroken(fix_broken)
            .fix_dpkg_status(!no_fix_dpkg_status)
            .remove_config(remove_config)
            .autoremove(autoremove)
            .download_only(download_only)
            .no_clean(no_clean)
            .build()
            .run(|ctx| {
                let oma_apt_args = OmaAptArgs::builder()
                    .sysroot(ctx.config.sysroot.to_string_lossy().to_string())
                    .another_apt_options(&ctx.config.apt_options)
                    .dpkg_force_confnew(force_confnew)
                    .dpkg_force_unsafe_io(force_unsafe_io)
                    .force_yes(force_yes)
                    .build();

                let mut apt = OmaApt::new(vec![], oma_apt_args, ctx.config.dry_run)?;

                let pkg = apt
                    .cache
                    .get(&package)
                    .ok_or_else(|| anyhow!(fl!("can-not-get-pkg-from-database", name = package)))?;

                let versions = pkg.versions().collect::<Vec<_>>();
                let versions_str = versions
                    .iter()
                    .map(|x| x.version().to_string())
                    .collect::<Vec<_>>();

                let mut v = vec![];
                for i in 0..versions.len() {
                    for j in 1..versions.len() {
                        if i == j {
                            continue;
                        }

                        if versions_str[i] == versions_str[j] {
                            v.push((i, j));
                        }
                    }
                }

                let mut version_str_display = versions_str.clone();
                for (a, b) in v {
                    for i in [a, b] {
                        let mut site_branch_suite = vec![];
                        for i in versions[i].version_files() {
                            site_branch_suite.push(get_source_from_version_file(i));
                        }

                        version_str_display[i] = format!(
                            "{} ({})",
                            versions[i].version(),
                            site_branch_suite.join(",")
                        );
                    }
                }

                let theme = ColorfulTheme::default();
                let mut dialoguer = Select::with_theme(&theme)
                    .items(&version_str_display)
                    .with_prompt(fl!("pick-tips", pkgname = pkg.fullname(true)));

                let pos = if let Some(installed) = pkg.installed() {
                    versions
                        .iter()
                        .position(|v| {
                            v.version() == installed.version()
                                && v.uris()
                                    .iter()
                                    .any(|uri| installed.uris().iter().any(|uri2| uri == uri2))
                        })
                        .unwrap_or(0)
                } else {
                    0
                };

                dialoguer = dialoguer.default(pos);

                let size = tui_select_list_size();
                dialoguer = dialoguer.max_length(size.into());

                let sel = dialoguer.interact().map_err(|_| anyhow!(""))?;

                let pkgs = vec![
                    OmaPackage::new(&versions[sel], &pkg).map_err(|e| OutputError {
                        description: e.to_string(),
                        source: None,
                    })?,
                ];

                apt.install(&pkgs, false)?;

                ctx.commit(apt)
            })
    }
}

fn get_source_from_version_file(i: VersionFile<'_>) -> String {
    let pkg_file = i.package_file();

    let mut result = pkg_file
        .site()
        .map(|s| s.to_string())
        .unwrap_or_else(|| fl!("pick-unknown-source"));

    result.push(':');
    if let Some(archive) = pkg_file.archive() {
        result.push_str(archive);
    }

    if let Some(component) = pkg_file.component() {
        result.push('/');
        result.push_str(component);
    }

    result
}
