use std::path::PathBuf;
use std::thread;

use apt_auth_config::AuthConfig;
use flume::unbounded;
use oma_console::{due_to, success};
use oma_pm::apt::{AptConfig, DownloadConfig, OmaApt, OmaAptArgs};
use oma_pm::matches::PackagesMatcher;
use oma_utils::dpkg::dpkg_arch;
use reqwest::StatusCode;
use tracing::{error, info};

use crate::pb::{NoProgressBar, OmaMultiProgressBar, RenderDownloadProgress};
use crate::utils::is_root;
use crate::{error::OutputError, subcommand::utils::handle_no_result};
use crate::{fl, OmaArgs, HTTP_CLIENT};

use super::utils::is_terminal;

pub fn execute(
    keyword: Vec<&str>,
    path: Option<PathBuf>,
    oma_args: OmaArgs,
) -> Result<i32, OutputError> {
    let OmaArgs {
        dry_run,
        network_thread,
        no_progress,
        ..
    } = oma_args;

    let path = path.unwrap_or_else(|| PathBuf::from("."));

    let path = path.canonicalize().map_err(|e| OutputError {
        description: format!("Failed to canonicalize path: {}", path.display()),
        source: Some(Box::new(e)),
    })?;

    let apt_config = AptConfig::new();
    let oma_apt_args = OmaAptArgs::builder().build();
    let apt = OmaApt::new(vec![], oma_apt_args, dry_run, apt_config)?;
    let arch = dpkg_arch("/")?;
    let matcher = PackagesMatcher::builder()
        .cache(&apt.cache)
        .filter_candidate(true)
        .filter_downloadable_candidate(true)
        .select_dbg(false)
        .native_arch(&arch)
        .build();

    let (pkgs, no_result) = matcher.match_pkgs_and_versions(keyword)?;
    handle_no_result("/", no_result, no_progress)?;

    let (tx, rx) = unbounded();

    thread::spawn(move || {
        let mut pb: Box<dyn RenderDownloadProgress> = if oma_args.no_progress || !is_terminal() {
            Box::new(NoProgressBar::default())
        } else {
            Box::new(OmaMultiProgressBar::default())
        };
        pb.render_progress(&rx);
    });

    let (success, failed) = apt.download(
        &HTTP_CLIENT,
        pkgs,
        DownloadConfig {
            network_thread: Some(network_thread),
            download_dir: Some(&path),
            auth: &AuthConfig::system("/")?,
        },
        tx,
        dry_run,
    )?;

    if !success.is_empty() {
        success!(
            "{}",
            fl!(
                "successfully-download-to-path",
                len = success.len(),
                path = path.display().to_string()
            )
        );
    }

    if !failed.is_empty() {
        let len = failed.len();
        for f in failed {
            let e = OutputError::from(f);

            error!("{e}");
            if let Some(s) = e.source {
                due_to!("{s}");
                if let Some(e) = s.downcast_ref::<reqwest::Error>() {
                    if e.status().is_some_and(|x| x == StatusCode::UNAUTHORIZED) {
                        if !is_root() {
                            info!("{}", fl!("auth-need-permission"));
                        } else {
                            info!("{}", fl!("lack-auth-config-1"));
                            info!("{}", fl!("lack-auth-config-2"));
                        }
                    }
                }
            }
        }

        return Err(OutputError {
            description: fl!("download-failed-with-len", len = len),
            source: None,
        });
    }

    Ok(0)
}
