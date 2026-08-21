use std::{collections::HashMap, path::Path, sync::Arc};

use flume::Sender;
use oma_fetch::{
    DownloadEntry, DownloadManager, DownloadSource, DownloadSourceType, Event, Summary,
    checksum::Checksum,
    mirror::{MirrorSourceType, ResolvedMirror, resolve_mirrors},
};
use oma_pm_operation_type::InstallEntry;
use oma_utils::url_no_escape::url_no_escape_times;
use reqwest_middleware::ClientWithMiddleware;
use spdlog::debug;

use crate::{
    CustomDownloadMessage,
    apt::{DownloadConfig, OmaAptError, OmaAptResult},
};

/// Whether a URL uses the apt `mirror://` protocol (mirror, mirror+http(s),
/// mirror+file).
fn is_mirror_uri(uri: &str) -> bool {
    uri.starts_with("mirror:") || uri.starts_with("mirror+")
}

/// Expand a mirrored package URL (`download_url`) into one `DownloadSource`
/// per resolved mirror. `index_url` is the source's base URI (`mirror://...`,
/// with a trailing `/`), which `download_url` starts with; the remainder is
/// the path inside the repository. All expansions share `order` — the index
/// of this URL among the package's alternates — so mirror priority is only
/// compared between mirrors of this same list.
fn mirror_download_sources(
    download_url: &str,
    index_url: &str,
    mirrors: &[ResolvedMirror],
    download_only: bool,
    order: usize,
) -> Vec<DownloadSource> {
    let suffix = download_url.strip_prefix(index_url).unwrap_or(download_url);

    mirrors
        .iter()
        .map(|m| {
            let url = format!(
                "{}/{}",
                m.url.trim_end_matches('/'),
                suffix.trim_start_matches('/')
            );
            let (source_type, url) = match m.source_type {
                MirrorSourceType::Http => (DownloadSourceType::Http, url),
                MirrorSourceType::File => (
                    DownloadSourceType::Local(!download_only),
                    url_no_escape_times(&url, 1),
                ),
            };
            DownloadSource {
                url,
                source_type,
                order,
                // Keep the mirror list's priority so the download manager
                // tries higher-priority mirrors (even HTTP) before
                // lower-priority ones (even local `file:` fallbacks) —
                // among expansions of this same list only.
                priority: m.priority,
            }
        })
        .collect()
}

/// Download packages (inner)
pub async fn download_pkgs(
    client: ClientWithMiddleware,
    download_pkg_list: Arc<[InstallEntry]>,
    config: DownloadConfig,
    download_only: bool,
    custom_download_message: CustomDownloadMessage,
    tx: Sender<Event>,
) -> OmaAptResult<Summary> {
    let DownloadConfig {
        network_thread,
        download_dir,
    } = config;

    debug!(
        "Download list: {download_pkg_list:?}, download to: {}",
        download_dir
            .clone()
            .unwrap_or(Path::new(".").into())
            .display()
    );

    if download_pkg_list.is_empty() {
        return Ok(Summary {
            success: vec![],
            failed: vec![],
        });
    }

    let mut download_list = vec![];
    let mut total_size = 0;

    // Resolve each distinct mirror list once per (URI, architecture) and
    // reuse it for every package from the same source and arch: a large
    // install/upgrade can otherwise fetch and parse the same `mirror://`
    // list once per package, sequentially, and a single transient failure
    // on a repeat would abort the whole download.
    let mut resolved_mirrors: HashMap<(String, String), Vec<ResolvedMirror>> = HashMap::new();

    for entry in download_pkg_list.iter() {
        let arch = entry.arch();
        // Select mirrors for this package's architecture so `arch:`-tagged
        // mirrors of other architectures are not tried first (an amd64
        // package must not be attempted against a higher-priority
        // `arch:arm64` mirror). `all`/`any` packages are served by any
        // mirror, so they keep the unfiltered selection.
        let arch_filter = if arch == "all" || arch == "any" {
            None
        } else {
            Some(arch)
        };

        let uris = entry.pkg_urls();
        let mut sources = vec![];

        // `uris` are the package's alternate URLs in the caller's order
        // (primary first). Every source of one URL shares its `order`, so
        // the alternates keep that order and mirror priority is only ever
        // compared between expansions of the same mirror list.
        for (order, x) in uris.iter().enumerate() {
            if is_mirror_uri(&x.download_url) {
                // `index_url` is the source's base URI (e.g. `mirror://...`,
                // with a trailing `/`); resolve the mirror list and expand
                // into one `DownloadSource` per mirror. The download manager
                // tries the sources in order, so later mirrors are the
                // fallbacks (apt's Alternate-URIs behavior).
                let key = (x.index_url.clone(), arch.to_string());
                let mirrors = match resolved_mirrors.get(&key) {
                    Some(mirrors) => mirrors.clone(),
                    None => {
                        let mirrors =
                            resolve_mirrors(&x.index_url, &client, arch_filter, None, false).await?;
                        resolved_mirrors.insert(key, mirrors.clone());
                        mirrors
                    }
                };
                sources.extend(mirror_download_sources(
                    &x.download_url,
                    &x.index_url,
                    &mirrors,
                    download_only,
                    order,
                ));
            } else if x.index_url.starts_with("file:") {
                // Local sources are preferred over HTTP ones.
                sources.push(DownloadSource {
                    url: url_no_escape_times(&x.download_url, 1),
                    source_type: DownloadSourceType::Local(!download_only),
                    order,
                    priority: 0,
                });
            } else {
                sources.push(DownloadSource {
                    url: x.download_url.clone(),
                    source_type: DownloadSourceType::Http,
                    order,
                    priority: u64::MAX,
                });
            }
        }

        debug!("Sources is: {:?}", sources);

        let msg = custom_download_message(entry);

        let download_dir = download_dir
            .clone()
            .map(|x| x.to_path_buf())
            .unwrap_or_else(|| ".".into());

        let download_entry = DownloadEntry::builder()
            .source(sources)
            .filename(apt_style_filename(entry))
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
            });

        let download_entry = if download_only {
            download_entry.dir(download_dir).build()
        } else {
            download_entry
                .dir(download_dir.join("partial"))
                .final_dir(download_dir)
                .build()
        };

        total_size += entry.download_size();

        download_list.push(download_entry);
    }

    let downloader = DownloadManager::builder()
        .client(client.clone())
        .download_list(download_list.into())
        .maybe_threads(network_thread)
        .total_size(total_size)
        .build();

    let tx_for_closure = tx.clone();

    let res = downloader
        .start_download(move |event| {
            let tx_inner = tx_for_closure.clone();
            async move {
                let _ = tx_inner.send_async(event).await;
            }
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_mirror_uri() {
        assert!(is_mirror_uri("mirror://host/list"));
        assert!(is_mirror_uri("mirror+http://host/list"));
        assert!(is_mirror_uri("mirror+https://host/list"));
        assert!(is_mirror_uri("mirror+file:///path"));
        assert!(!is_mirror_uri("http://host"));
        assert!(!is_mirror_uri("https://host"));
        assert!(!is_mirror_uri("file:///path"));
    }

    #[test]
    fn test_mirror_download_sources() {
        let mirrors = vec![
            ResolvedMirror {
                url: "http://m1.example.com/debian/".into(),
                source_type: MirrorSourceType::Http,
                priority: 1,
            },
            ResolvedMirror {
                url: "file:///local/repo".into(),
                source_type: MirrorSourceType::File,
                priority: 2,
            },
        ];

        // `index_url` is the mirror base URI (with trailing `/`); the
        // download URL is the same base plus the pool path.
        let sources = mirror_download_sources(
            "mirror+http://host/list/pool/main/a/apt_1_amd64.deb",
            "mirror+http://host/list/",
            &mirrors,
            false,
            2,
        );

        assert_eq!(sources.len(), 2);
        assert_eq!(
            sources[0].url,
            "http://m1.example.com/debian/pool/main/a/apt_1_amd64.deb"
        );
        assert_eq!(sources[0].source_type, DownloadSourceType::Http);
        // The mirror-list priority survives onto each source, scoped to the
        // caller's alternate-URL order.
        assert_eq!(sources[0].priority, 1);
        assert_eq!(sources[0].order, 2);
        assert!(
            sources[1]
                .url
                .starts_with("file:///local/repo/pool/main/a/apt_1_amd64.deb")
        );
        assert_eq!(sources[1].source_type, DownloadSourceType::Local(true));
        assert_eq!(sources[1].priority, 2);
    }
}
