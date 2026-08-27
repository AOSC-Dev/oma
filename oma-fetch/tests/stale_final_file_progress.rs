use std::time::Duration;

use oma_fetch::{
    DownloadEntry, DownloadManager, DownloadSource, DownloadSourceType, Event, checksum::Checksum,
};
use reqwest::ClientBuilder;
use sha2::{Digest, Sha256};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;

/// A minimal HTTP server that serves a fixed payload to every GET request.
async fn http_server(payload: Vec<u8>) -> u16 {
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
                let _ = socket.write_all(header.as_bytes()).await;
                let _ = socket.write_all(&payload).await;
                let _ = socket.shutdown().await;
            });
        }
    });
    port
}

/// Regression test: a stale `final_dir` file with a mismatched hash must not
/// inflate the global progress bar; its bytes are undone before re-download.
#[tokio::test]
async fn global_progress_not_inflated_by_stale_final_file() {
    rustls::crypto::ring::default_provider()
        .install_default()
        .unwrap();

    let payload = b"hello world".to_vec();
    let port = http_server(payload.clone()).await;

    let hash = Checksum::Sha256(Sha256::digest(&payload).to_vec());

    let temp = std::env::temp_dir().join("oma-fetch-stale-final-file-test");
    let _ = tokio::fs::remove_dir_all(&temp).await;
    tokio::fs::create_dir_all(&temp).await.unwrap();

    // A stale file in the final dir whose content does NOT match `hash`.
    tokio::fs::write(temp.join("pkg"), vec![b'x'; 100])
        .await
        .unwrap();

    let source = DownloadSource {
        url: format!("http://127.0.0.1:{port}/pkg"),
        source_type: DownloadSourceType::Http,
    };
    let entry = DownloadEntry::builder()
        .source(vec![source])
        .filename("pkg".to_string())
        .dir(temp.join("partial"))
        .final_dir(temp.clone())
        .allow_resume(false)
        .maybe_hash(Some(hash))
        .build();

    let client = ClientBuilder::new().user_agent("oma").build().unwrap();

    let (tx, rx) = flume::unbounded();
    let download_manager = DownloadManager::builder()
        .client(client.into())
        .download_list(Box::new([entry]))
        .threads(1)
        .timeout(Duration::from_secs(5))
        .build();

    let accounting = tokio::spawn(async move {
        let mut added: u64 = 0;
        let mut subbed: u64 = 0;
        while let Ok(event) = rx.recv_async().await {
            match &event {
                Event::GlobalProgressAdd(n) => added += n,
                Event::GlobalProgressSub(n) => subbed += n,
                _ => {}
            }
            if let Event::AllDone = event {
                break;
            }
        }
        (added, subbed)
    });

    let summary = download_manager
        .start_download(move |event| {
            let tx = tx.clone();
            async move {
                let _ = tx.send_async(event).await;
            }
        })
        .await
        .unwrap();

    let (added, subbed) = accounting.await.unwrap();

    assert!(summary.is_download_success());
    assert_eq!(summary.success.len(), 1);

    let net = added - subbed;
    assert_eq!(
        net,
        payload.len() as u64,
        "global progress must only count the new download ({}) once, \
         not the stale final file's 100 bytes too; got net {net} \
         (added {added}, subbed {subbed})",
        payload.len(),
    );

    // The final file must have been replaced with the correct content.
    let final_content = tokio::fs::read(temp.join("pkg")).await.unwrap();
    assert_eq!(final_content, payload);

    let _ = tokio::fs::remove_dir_all(&temp).await;
}
