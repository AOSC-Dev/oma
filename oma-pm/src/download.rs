use std::{borrow::Cow, future::Future, path::Path};

use oma_console::console;
use oma_fetch::{
    checksum::Checksum, reqwest::Client, DownloadEntry, DownloadManager, DownloadSource,
    DownloadSourceType, Event, Summary,
};
use oma_pm_operation_type::InstallEntry;
use tracing::debug;

use crate::apt::{DownloadConfig, OmaAptError, OmaAptResult};

/// Download packages (inner)
pub async fn download_pkgs<F, Fut>(
    client: &Client,
    download_pkg_list: &[InstallEntry],
    config: DownloadConfig<'_>,
    callback: F,
) -> OmaAptResult<Summary>
where
    F: Fn(Event) -> Fut,
    Fut: Future<Output = ()>,
{
    let DownloadConfig {
        network_thread,
        download_dir,
        auth,
    } = config;

    debug!(
        "Download list: {download_pkg_list:?}, download to: {}",
        download_dir.unwrap_or(Path::new(".")).display()
    );

    if download_pkg_list.is_empty() {
        return Ok(Summary {
            success: vec![],
            failed: vec![],
        });
    }

    let mut download_list = vec![];
    let mut total_size = 0;

    for entry in download_pkg_list {
        let uris = entry.pkg_urls();
        let sources = uris
            .iter()
            .map(|x| {
                let source_type = if x.starts_with("file:") {
                    DownloadSourceType::Local(false)
                } else {
                    let auth = auth.find_package_url(x);

                    DownloadSourceType::Http {
                        auth: auth.map(|x| (x.user.to_owned(), x.password.to_owned())),
                    }
                };

                DownloadSource {
                    url: x.to_string(),
                    source_type,
                }
            })
            .collect::<Vec<_>>();

        debug!("Sources is: {:?}", sources);

        let filename = uris
            .first()
            .and_then(|x| x.split('/').last())
            .take()
            .ok_or_else(|| OmaAptError::InvalidFileName(entry.name().to_string()))?;

        debug!("filename is: {}", filename);

        let new_version = if console::measure_text_width(entry.new_version()) > 25 {
            console::truncate_str(entry.new_version(), 25, "...")
        } else {
            Cow::Borrowed(entry.new_version())
        };

        let msg = format!("{} {new_version} ({})", entry.name(), entry.arch());

        let download_entry = DownloadEntry::builder()
            .source(sources)
            .filename(apt_style_filename(entry))
            .dir(
                download_dir
                    .map(|x| x.to_path_buf())
                    .unwrap_or_else(|| ".".into()),
            )
            .allow_resume(true)
            .msg(msg)
            .maybe_hash({
                if let Some(checksum) = entry.sha256() {
                    Some(Checksum::from_sha256_str(checksum)?)
                } else if let Some(checksum) = entry.sha512() {
                    Some(Checksum::from_sha512_str(checksum)?)
                } else if let Some(checksum) = entry.md5() {
                    Some(Checksum::from_md5_str(checksum)?)
                } else {
                    None
                }
            })
            .build();

        total_size += entry.download_size();

        download_list.push(download_entry);
    }

    let downloader = DownloadManager::builder()
        .client(client)
        .download_list(&download_list)
        .maybe_threads(network_thread)
        .total_size(total_size)
        .build();

    let res = downloader
        .start_download(|event| async {
            callback(event).await;
        })
        .await
        .unwrap();

    if !res.is_download_success() {
        return Err(OmaAptError::FailedToDownload(res.failed.len()));
    }

    Ok(res)
}

/// Get apt style file name
fn apt_style_filename(entry: &InstallEntry) -> String {
    let package = entry.name_without_arch();
    let version = entry.new_version();
    let arch = entry.arch();

    let version = version.replace(':', "%3a");

    format!("{package}_{version}_{arch}.deb").replace("%2b", "+")
}
