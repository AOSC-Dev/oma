use std::path::PathBuf;
use std::thread;

use clap::Args;
use clap_complete::ArgValueCompleter;
use flume::unbounded;
use oma_pm::apt::{AptConfig, DownloadConfig, OmaApt, OmaAptArgs};
use oma_pm::matches::PackagesMatcher;
use spdlog::error;

use crate::config::Config;
use crate::pb::{NoProgressBar, OmaMultiProgressBar, RenderPackagesDownloadProgress};
use crate::utils::pkgnames_completions;
use crate::{HTTP_CLIENT, fl, success};
use crate::{error::OutputError, subcommand::utils::handle_no_result};

use crate::args::CliExecuter;

use super::utils::{auth_config, download_message};

#[derive(Debug, Args)]
pub struct Download {
    /// Package(s) to download
    #[arg(required = true, add = ArgValueCompleter::new(pkgnames_completions), help = fl!("clap-download-packages-help"))]
    #[arg(help_heading = &**crate::args::ARG_HELP_HEADING_MUST)]
    packages: Vec<String>,
    /// The path where package(s) should be downloaded to
    #[arg(short, long, default_value = ".", help = fl!("clap-download-path-help"))]
    path: PathBuf,
    /// Run oma in "dry-run" mode. Useful for testing changes and operations without making changes to the system
    #[arg(from_global, help = fl!("clap-dry-run-help"), long_help = fl!("clap-dry-run-long-help"))]
    dry_run: bool,
    /// Setup download threads (default as 4)
    #[arg(from_global, help = fl!("clap-download-threads-help"))]
    download_threads: Option<usize>,
}

impl CliExecuter for Download {
    fn execute(self, config: &Config, no_progress: bool) -> Result<i32, OutputError> {
        let Download {
            packages,
            path,
            dry_run,
            download_threads,
        } = self;

        let path = path.canonicalize().map_err(|e| OutputError {
            description: format!("Failed to canonicalize path: {}", path.display()),
            source: Some(Box::new(e)),
        })?;

        let apt_config = AptConfig::new();
        let oma_apt_args = OmaAptArgs::builder().build();
        let apt = OmaApt::new(vec![], oma_apt_args, dry_run, apt_config)?;
        let matcher = PackagesMatcher::builder()
            .cache(&apt.cache)
            .filter_candidate(true)
            .filter_downloadable_candidate(true)
            .select_dbg(false)
            .build();

        let (pkgs, no_result) =
            matcher.match_pkgs_and_versions(packages.iter().map(|x| x.as_str()))?;
        handle_no_result(no_result, no_progress)?;

        let (tx, rx) = unbounded();

        thread::spawn(move || {
            let mut pb: Box<dyn RenderPackagesDownloadProgress> = if no_progress {
                Box::new(NoProgressBar::default())
            } else {
                Box::new(OmaMultiProgressBar::default())
            };
            pb.render_progress(&rx, true);
        });

        let summary = apt.download(
            &HTTP_CLIENT,
            pkgs,
            DownloadConfig {
                network_thread: Some(download_threads.unwrap_or_else(|| config.network_thread())),
                download_dir: Some(&path),
                auth: auth_config("/").as_ref(),
            },
            download_message(),
            |event| async {
                if let Err(e) = tx.send_async(event).await {
                    error!("{}", e);
                }
            },
        )?;

        if !summary.success.is_empty() {
            success!(
                "{}",
                fl!(
                    "successfully-download-to-path",
                    len = summary.success.len(),
                    path = path.display().to_string()
                )
            );
        }

        if !summary.is_download_success() {
            let len = summary.failed.len();

            return Err(OutputError {
                description: fl!("download-failed-with-len", len = len),
                source: None,
            });
        }

        Ok(0)
    }
}
