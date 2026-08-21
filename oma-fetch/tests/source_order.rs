use std::time::Duration;

use oma_fetch::{DownloadEntry, DownloadManager, DownloadSource, DownloadSourceType};
use reqwest::ClientBuilder;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;

/// A minimal HTTP server that serves the same fixed body for any request.
async fn http_server(body: &'static [u8]) -> u16 {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let port = listener.local_addr().unwrap().port();
    let body = body.to_vec();
    tokio::spawn(async move {
        while let Ok((mut socket, _)) = listener.accept().await {
            let body = body.clone();
            tokio::spawn(async move {
                let mut buf = [0u8; 4096];
                let _ = socket.read(&mut buf).await;
                let head = format!(
                    "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
                    body.len()
                );
                let _ = socket.write_all(head.as_bytes()).await;
                let _ = socket.write_all(&body).await;
            });
        }
    });
    port
}

/// A higher-priority HTTP mirror must be tried before a lower-priority
/// local `file:` fallback: the download manager sorts by the mirror-list
/// priority, not by transport (which would promote the local source).
#[tokio::test]
async fn higher_priority_http_mirror_wins_over_lower_priority_local() {
    let _ = rustls::crypto::ring::default_provider().install_default();

    let port = http_server(b"from-http").await;
    let work = std::env::temp_dir().join(format!("oma-fetch-source-order-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&work);
    tokio::fs::create_dir_all(&work).await.unwrap();

    let local_file = work.join("local.deb");
    std::fs::write(&local_file, b"from-local").unwrap();

    let entry = DownloadEntry::builder()
        .source(vec![
            // Primary: HTTP mirror with higher priority.
            DownloadSource {
                url: format!("http://127.0.0.1:{port}/pkg.deb"),
                source_type: DownloadSourceType::Http,
                order: 0,
                priority: 1,
            },
            // Fallback: local `file:` mirror with lower priority. A
            // transport sort would promote this one first; priority must not.
            DownloadSource {
                url: format!("file://{}", local_file.display()),
                source_type: DownloadSourceType::Local(false),
                order: 0,
                priority: 2,
            },
        ])
        .filename("pkg.deb".to_string())
        .dir(work.join("partial"))
        .allow_resume(true)
        .build();

    let client = ClientBuilder::new().user_agent("oma").build().unwrap();
    let (tx, rx) = flume::unbounded();
    let download_manager = DownloadManager::builder()
        .client(client.into())
        .download_list(Box::new([entry]))
        .threads(1)
        .timeout(Duration::from_secs(5))
        .build();

    let summary = download_manager
        .start_download(move |event| {
            let tx = tx.clone();
            async move {
                let _ = tx.send_async(event).await;
            }
        })
        .await
        .unwrap();

    // Drain events so the manager's channels do not block.
    drop(rx);

    assert_eq!(summary.success.len(), 1, "the HTTP source should win");
    let won = &summary.success[0];
    assert!(
        won.url.starts_with(&format!("http://127.0.0.1:{port}/")),
        "expected the HTTP source to be used, got: {}",
        won.url
    );
    // The body proves the HTTP source's content was written, not the local
    // fallback's (which a transport sort would have promoted).
    assert_eq!(
        std::fs::read(work.join("partial/pkg.deb")).unwrap(),
        b"from-http"
    );
}

/// Non-mirror sources keep the transport preference: at equal priority a
/// local `file:` source is tried before an HTTP one, even when the HTTP
/// source is listed first.
#[tokio::test]
async fn local_source_is_tried_before_http_at_equal_priority() {
    let _ = rustls::crypto::ring::default_provider().install_default();

    let port = http_server(b"from-http").await;
    let work = std::env::temp_dir().join(format!(
        "oma-fetch-source-order-local-{}",
        std::process::id()
    ));
    let _ = std::fs::remove_dir_all(&work);
    tokio::fs::create_dir_all(&work).await.unwrap();

    let local_file = work.join("local.deb");
    std::fs::write(&local_file, b"from-local").unwrap();

    let entry = DownloadEntry::builder()
        .source(vec![
            // HTTP listed first, but the local source must win at equal
            // priority (the download manager prefers local transports).
            DownloadSource {
                url: format!("http://127.0.0.1:{port}/pkg.deb"),
                source_type: DownloadSourceType::Http,
                order: 0,
                priority: u64::MAX,
            },
            DownloadSource {
                url: format!("file://{}", local_file.display()),
                source_type: DownloadSourceType::Local(false),
                order: 0,
                priority: u64::MAX,
            },
        ])
        .filename("pkg.deb".to_string())
        .dir(work.join("partial"))
        .allow_resume(true)
        .build();

    let client = ClientBuilder::new().user_agent("oma").build().unwrap();
    let (tx, rx) = flume::unbounded();
    let download_manager = DownloadManager::builder()
        .client(client.into())
        .download_list(Box::new([entry]))
        .threads(1)
        .timeout(Duration::from_secs(5))
        .build();

    let summary = download_manager
        .start_download(move |event| {
            let tx = tx.clone();
            async move {
                let _ = tx.send_async(event).await;
            }
        })
        .await
        .unwrap();

    drop(rx);

    assert_eq!(summary.success.len(), 1, "the local source should win");
    assert!(
        summary.success[0].url.starts_with("file://"),
        "expected the local source to be used, got: {}",
        summary.success[0].url
    );
    assert_eq!(
        std::fs::read(work.join("partial/pkg.deb")).unwrap(),
        b"from-local"
    );
}

/// The caller's alternate-URL order is preserved: a primary normal HTTP URL
/// (order 0) is tried before a `mirror://` fallback's expansions (order 1),
/// even though the mirror's priority number is much smaller — mirror
/// priority only applies among expansions of the same mirror list.
#[tokio::test]
async fn primary_alternate_url_wins_over_mirror_priority() {
    let _ = rustls::crypto::ring::default_provider().install_default();

    let port = http_server(b"from-http").await;
    let work = std::env::temp_dir().join(format!(
        "oma-fetch-source-order-alt-{}",
        std::process::id()
    ));
    let _ = std::fs::remove_dir_all(&work);
    tokio::fs::create_dir_all(&work).await.unwrap();

    let local_file = work.join("mirror.deb");
    std::fs::write(&local_file, b"from-mirror").unwrap();

    let entry = DownloadEntry::builder()
        .source(vec![
            // Primary: normal HTTP repository URL.
            DownloadSource {
                url: format!("http://127.0.0.1:{port}/pkg.deb"),
                source_type: DownloadSourceType::Http,
                order: 0,
                priority: u64::MAX,
            },
            // Fallback: a `mirror://` expansion. Its tiny priority number
            // must not promote it over the primary alternate.
            DownloadSource {
                url: format!("file://{}", local_file.display()),
                source_type: DownloadSourceType::Local(false),
                order: 1,
                priority: 1,
            },
        ])
        .filename("pkg.deb".to_string())
        .dir(work.join("partial"))
        .allow_resume(true)
        .build();

    let client = ClientBuilder::new().user_agent("oma").build().unwrap();
    let (tx, rx) = flume::unbounded();
    let download_manager = DownloadManager::builder()
        .client(client.into())
        .download_list(Box::new([entry]))
        .threads(1)
        .timeout(Duration::from_secs(5))
        .build();

    let summary = download_manager
        .start_download(move |event| {
            let tx = tx.clone();
            async move {
                let _ = tx.send_async(event).await;
            }
        })
        .await
        .unwrap();

    drop(rx);

    assert_eq!(summary.success.len(), 1, "the primary URL should win");
    assert!(
        summary.success[0].url.starts_with(&format!("http://127.0.0.1:{port}/")),
        "expected the primary HTTP source to be used, got: {}",
        summary.success[0].url
    );
}
