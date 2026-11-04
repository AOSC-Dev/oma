use std::time::Duration;

use oma_fetch::{
    CompressType, DownloadEntry, DownloadManager, DownloadSource, DownloadSourceType, TaskTracker,
};
use reqwest::ClientBuilder;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;

/// A minimal HTTP server that serves the payload in chunks with a pause in
/// between, so a download is still in progress when the test cancels it.
async fn slow_http_server(payload: Vec<u8>, chunk_size: usize, delay: Duration) -> u16 {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let port = listener.local_addr().unwrap().port();
    tokio::spawn(async move {
        while let Ok((mut socket, _)) = listener.accept().await {
            let payload = payload.clone();
            tokio::spawn(async move {
                let mut buf = [0u8; 4096];
                let _ = socket.read(&mut buf).await;
                let header = format!(
                    "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
                    payload.len()
                );
                if socket.write_all(header.as_bytes()).await.is_err() {
                    return;
                }
                for chunk in payload.chunks(chunk_size) {
                    if socket.write_all(chunk).await.is_err() {
                        return;
                    }
                    tokio::time::sleep(delay).await;
                }
                let _ = socket.shutdown().await;
            });
        }
    });
    port
}

/// Regression test: once cancellation has been drained, nothing may touch
/// the partial file any more.
///
/// Dropping a download worker drops its `TaskGuard`, but the worker's file
/// writes used to go through `tokio::fs`, which runs on `spawn_blocking`:
/// an operation that had already been dispatched could keep truncating or
/// writing the partial file after `TaskTracker::wait` returned and the lists
/// lock was released. The file operations are synchronous now, so a worker
/// can only be dropped at an await point, after every write finished.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn cancelled_download_stops_touching_the_partial_file() {
    let _ = rustls::crypto::ring::default_provider().install_default();

    let payload = vec![0xABu8; 4 * 1024 * 1024];
    let port = slow_http_server(payload.clone(), 32 * 1024, Duration::from_millis(10)).await;

    let temp = std::env::temp_dir().join(format!(
        "oma-fetch-cancel-partial-test-{}",
        std::process::id()
    ));
    let partial_dir = temp.join("partial");
    let _ = std::fs::remove_dir_all(&temp);
    std::fs::create_dir_all(&partial_dir).unwrap();

    let source = DownloadSource {
        url: format!("http://127.0.0.1:{port}/pkg"),
        source_type: DownloadSourceType::Http,
        file_type: CompressType::None,
    };
    let entry = DownloadEntry::builder()
        .source(vec![source])
        .filename("pkg".to_string())
        .dir(partial_dir.clone())
        .allow_resume(false)
        .build();

    let client = ClientBuilder::new().user_agent("oma").build().unwrap();

    let tracker = TaskTracker::default();
    let download_manager = DownloadManager::builder()
        .client(client.into())
        .download_list(Box::new([entry]))
        .threads(1)
        .timeout(Duration::from_secs(30))
        .tracker(tracker.clone())
        .build();

    let handle =
        tokio::spawn(async move { download_manager.start_download(|_event| async {}).await });

    // Wait until the partial file exists and has content: the worker is
    // downloading at this point.
    let partial = partial_dir.join("pkg");
    let deadline = std::time::Instant::now() + Duration::from_secs(10);
    loop {
        if std::fs::metadata(&partial).map(|m| m.len()).unwrap_or(0) > 0 {
            break;
        }
        assert!(
            std::time::Instant::now() < deadline,
            "the download should have started writing the partial file"
        );
        tokio::time::sleep(Duration::from_millis(10)).await;
    }

    // Cancel like the refresh pump does: drop the download future, then wait
    // until every worker has unregistered.
    handle.abort();
    let _ = handle.await;

    let wait_tracker = tracker.clone();
    tokio::time::timeout(
        Duration::from_secs(10),
        tokio::task::spawn_blocking(move || wait_tracker.wait()),
    )
    .await
    .expect("tracker.wait() timed out: a download worker is still alive")
    .unwrap();

    // After the tracker is drained the partial file must stay untouched:
    // no background write may still be on its way.
    let len_after_drain = std::fs::metadata(&partial).map(|m| m.len()).unwrap_or(0);
    tokio::time::sleep(Duration::from_millis(300)).await;
    let len_later = std::fs::metadata(&partial).map(|m| m.len()).unwrap_or(0);

    let _ = std::fs::remove_dir_all(&temp);
    assert_eq!(
        len_after_drain, len_later,
        "a cancelled download must not keep writing the partial file after tracker.wait()"
    );
    assert!(
        len_later < payload.len() as u64,
        "the download should have been cancelled mid-way, got {len_later} of {} bytes",
        payload.len()
    );
}
