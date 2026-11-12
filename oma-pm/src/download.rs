use std::{path::Path, sync::Arc};

use ahash::HashMap;
use flume::Sender;
use oma_fetch::{
    CompressType, DownloadEntry, DownloadManager, DownloadSource, DownloadSourceType, Event,
    Summary,
    checksum::Checksum,
    mirror::{MirrorList, MirrorTarget, MirrorTransport, TargetKind},
};
use oma_logger::debug;
use oma_pm_operation_type::InstallEntry;
use oma_utils::url_no_escape::url_no_escape_times;
use reqwest_middleware::ClientWithMiddleware;

use crate::{
    CustomDownloadMessage,
    apt::{DownloadConfig, OmaAptError, OmaAptResult},
};

async fn package_sources(
    client: &ClientWithMiddleware,
    entry: &InstallEntry,
    download_only: bool,
    mirror_lists: &mut HashMap<String, Arc<MirrorList>>,
) -> OmaAptResult<(Vec<DownloadSource>, bool)> {
    let mut sources = Vec::new();
    let mut is_mirror = false;

    for pkg_url in entry.pkg_urls() {
        if is_mirror_uri(&pkg_url.index_url) {
            is_mirror = true;

            let list = if let Some(list) = mirror_lists.get(&pkg_url.index_url) {
                Arc::clone(list)
            } else {
                let list = Arc::new(
                    MirrorList::load(&pkg_url.index_url, client)
                        .await
                        .map_err(anyhow::Error::new)?,
                );
                mirror_lists.insert(pkg_url.index_url.clone(), Arc::clone(&list));
                list
            };

            let target = MirrorTarget {
                release: pkg_url.release.as_deref(),
                suite: pkg_url.suite.as_deref(),
                codename: pkg_url.codename.as_deref(),
                component: pkg_url.component.as_deref(),
                arch: pkg_url.arch.as_deref(),
                kind: TargetKind::File("deb"),
                ..Default::default()
            };

            let candidates = list
                .resolve(&pkg_url.index_url, &pkg_url.download_url, &target)
                .map_err(anyhow::Error::new)?;

            for candidate in candidates {
                let (source_type, url) = match candidate.transport {
                    MirrorTransport::File => (
                        DownloadSourceType::Local(!download_only),
                        url_no_escape_times(&candidate.url, 1),
                    ),
                    MirrorTransport::Http => (DownloadSourceType::Http, candidate.url),
                };

                sources.push(DownloadSource {
                    url,
                    source_type,
                    file_type: CompressType::None,
                });
            }

            continue;
        }

        let (source_type, url) = if pkg_url.index_url.starts_with("file:") {
            (
                DownloadSourceType::Local(!download_only),
                url_no_escape_times(&pkg_url.download_url, 1),
            )
        } else {
            (DownloadSourceType::Http, pkg_url.download_url.clone())
        };

        sources.push(DownloadSource {
            url,
            source_type,
            file_type: CompressType::None,
        });
    }

    Ok((sources, is_mirror))
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
    let mut mirror_lists: HashMap<String, Arc<MirrorList>> = HashMap::default();

    for entry in download_pkg_list.iter() {
        let (sources, is_mirror) =
            package_sources(&client, entry, download_only, &mut mirror_lists).await?;

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
            .preserve_source_order(is_mirror)
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

fn is_mirror_uri(uri: &str) -> bool {
    uri.starts_with("mirror+") || uri.starts_with("mirror://")
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
    use oma_fetch::reqwest::Client;
    use oma_pm_operation_type::{InstallOperation, PackageUrl};
    use std::time::{SystemTime, UNIX_EPOCH};

    fn block_on<F: std::future::Future>(future: F) -> F::Output {
        tokio::runtime::Runtime::new().unwrap().block_on(future)
    }

    fn temp_path(name: &str) -> std::path::PathBuf {
        std::env::temp_dir().join(format!(
            "oma-pm-mirror-download-{}-{}-{name}",
            std::process::id(),
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ))
    }

    fn install_entry(pkg_url: PackageUrl) -> InstallEntry {
        InstallEntry::builder()
            .name("go:amd64".to_string())
            .name_without_arch("go".to_string())
            .new_version("1.0".to_string())
            .new_size(0)
            .pkg_urls(vec![pkg_url])
            .arch("amd64".to_string())
            .download_size(0)
            .op(InstallOperation::Download)
            .index(0)
            .build()
    }

    #[test]
    fn resolves_mirror_package_sources_in_priority_order() {
        block_on(async {
            let list_path = temp_path("mirrors.list");
            std::fs::write(
                &list_path,
                "file:/repo-a\tpriority:2\tsuite:bookworm\tcomponent:main\tarch:amd64\ttype:deb\n\
                 file:/repo-b\tpriority:1\tsuite:bookworm\tcomponent:main\tarch:amd64\ttype:deb\n\
                 file:/repo-contrib\tpriority:0\tsuite:bookworm\tcomponent:contrib\tarch:amd64\ttype:deb\n",
            )
            .unwrap();

            let source_uri = format!("mirror+file:{}", list_path.display());
            let download_url = format!("{source_uri}/pool/main/g/go/go.deb");
            let entry = install_entry(PackageUrl {
                download_url,
                index_url: source_uri,
                release: Some("bookworm".to_string()),
                suite: Some("bookworm".to_string()),
                codename: None,
                component: Some("main".to_string()),
                arch: Some("amd64".to_string()),
            });

            let client: ClientWithMiddleware = Client::new().into();
            let mut lists = HashMap::default();
            let (sources, is_mirror) = package_sources(&client, &entry, true, &mut lists)
                .await
                .unwrap();

            assert!(is_mirror);
            assert_eq!(sources.len(), 2);
            assert_eq!(sources[0].url, "file:/repo-b/pool/main/g/go/go.deb");
            assert_eq!(sources[1].url, "file:/repo-a/pool/main/g/go/go.deb");
            assert_eq!(sources[0].source_type, DownloadSourceType::Local(false));

            let _ = std::fs::remove_file(&list_path);
        });
    }

    #[test]
    fn downloads_a_package_from_a_local_mirror() {
        block_on(async {
            let root = temp_path("repo");
            let repo = root.join("repo");
            let pool = repo.join("pool/main/g/go");
            std::fs::create_dir_all(&pool).unwrap();
            std::fs::write(pool.join("go_1.0_amd64.deb"), b"deb").unwrap();

            let list_path = root.join("mirrors.list");
            std::fs::write(&list_path, format!("file:{}\n", repo.display())).unwrap();

            let source_uri = format!("mirror+file:{}", list_path.display());
            let entry = install_entry(PackageUrl {
                download_url: format!("{source_uri}/pool/main/g/go/go_1.0_amd64.deb"),
                index_url: source_uri,
                release: None,
                suite: None,
                codename: None,
                component: None,
                arch: Some("amd64".to_string()),
            });

            let dest = root.join("out");
            std::fs::create_dir_all(&dest).unwrap();
            let (tx, _rx) = flume::unbounded();

            let summary = download_pkgs(
                Client::new().into(),
                Arc::from(vec![entry]),
                DownloadConfig {
                    network_thread: Some(1),
                    download_dir: Some(Arc::from(dest.as_path())),
                },
                true,
                Box::new(|_| "go".into()),
                tx,
            )
            .await
            .unwrap();

            assert!(summary.is_download_success());
            assert!(dest.join("go_1.0_amd64.deb").is_file());

            let _ = std::fs::remove_dir_all(&root);
        });
    }

    #[test]
    fn uses_configured_release_for_mirror_targeting() {
        block_on(async {
            let list_path = temp_path("release.list");
            std::fs::write(
                &list_path,
                "file:/repo-b\tpriority:1\trelease:bookworm\tcomponent:main\tarch:amd64\ttype:deb\n",
            )
            .unwrap();

            let source_uri = format!("mirror+file:{}", list_path.display());
            let entry = install_entry(PackageUrl {
                download_url: format!("{source_uri}/pool/main/g/go/go.deb"),
                index_url: source_uri,
                release: Some("bookworm".to_string()),
                suite: Some("oldstable".to_string()),
                codename: Some("bookworm".to_string()),
                component: Some("main".to_string()),
                arch: Some("amd64".to_string()),
            });

            let client: ClientWithMiddleware = Client::new().into();
            let mut lists = HashMap::default();
            let (sources, _) = package_sources(&client, &entry, true, &mut lists)
                .await
                .unwrap();

            assert_eq!(sources.len(), 1);
            assert_eq!(sources[0].url, "file:/repo-b/pool/main/g/go/go.deb");

            let _ = std::fs::remove_file(&list_path);
        });
    }

    #[test]
    fn keeps_plain_package_urls_as_http_sources() {
        block_on(async {
            let entry = install_entry(PackageUrl {
                download_url: "https://example.com/pool/main/g/go/go.deb".to_string(),
                index_url: "https://example.com/debian/".to_string(),
                release: None,
                ..Default::default()
            });

            let client: ClientWithMiddleware = Client::new().into();
            let mut lists = HashMap::default();
            let (sources, is_mirror) = package_sources(&client, &entry, true, &mut lists)
                .await
                .unwrap();

            assert!(!is_mirror);
            assert_eq!(sources.len(), 1);
            assert_eq!(sources[0].url, "https://example.com/pool/main/g/go/go.deb");
            assert_eq!(sources[0].source_type, DownloadSourceType::Http);
        });
    }
}
