use std::{path::PathBuf, time::Duration};

use oma_fetch::{
    DownloadEntry, DownloadManager, DownloadSource, DownloadSourceType, Event,
};
use reqwest::ClientBuilder;
use tokio::io::AsyncReadExt;
use tokio::net::TcpListener;

/// A server that accepts a connection, reads the request, then stalls
/// forever. The client's request phase therefore times out
/// (`SendRequestTimeout`).
async fn stall_server() -> u16 {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let port = listener.local_addr().unwrap().port();
    tokio::spawn(async move {
        while let Ok((mut socket, _)) = listener.accept().await {
            tokio::spawn(async move {
                let mut buf = [0u8; 4096];
                let _ = socket.read(&mut buf).await;
                // never respond
                std::future::pending::<()>().await;
            });
        }
    });
    port
}

/// Every bar that is opened (`NewProgressSpinner` / `NewProgressBar`) must be
/// closed (`ProgressDone` / `DownloadDone`) once the download finishes, no
/// matter which error path was taken. A leaked bar is exactly what makes the
/// terminal show more progress entries than there are worker threads.
#[tokio::test]
async fn every_bar_is_closed_on_request_timeout() {
    rustls::crypto::ring::default_provider()
        .install_default()
        .unwrap();

    let port = stall_server().await;
    let source = DownloadSource {
        url: format!("http://127.0.0.1:{port}/pkg.deb"),
        source_type: DownloadSourceType::Http,
    };
    let entry = DownloadEntry::builder()
        .source(vec![source])
        .filename("pkg.deb".to_string())
        .dir(PathBuf::from("/tmp/oma-fetch-bar-test"))
        .allow_resume(true)
        .build();

    let client = ClientBuilder::new().user_agent("oma").build().unwrap();

    let (tx, rx) = flume::unbounded();
    let download_manager = DownloadManager::builder()
        .client(client.into())
        .download_list(Box::new([entry]))
        .threads(1)
        .timeout(Duration::from_secs(2))
        .build();

    let lifecycle = tokio::spawn(async move {
        let mut opened = 0usize;
        let mut closed = 0usize;
        while let Ok(event) = rx.recv_async().await {
            match &event {
                Event::NewProgressSpinner { .. } | Event::NewProgressBar { .. } => opened += 1,
                Event::ProgressDone(_) | Event::DownloadDone { .. } => closed += 1,
                _ => {}
            }
            if let Event::AllDone = event {
                break;
            }
        }
        (opened, closed)
    });

    tokio::fs::create_dir_all("/tmp/oma-fetch-bar-test")
        .await
        .unwrap();

    let _summary = download_manager
        .start_download(move |event| {
            let tx = tx.clone();
            async move {
                let _ = tx.send_async(event).await;
            }
        })
        .await;

    let (opened, closed) = lifecycle.await.unwrap();

    assert_eq!(
        opened, closed,
        "bar lifecycle is unbalanced: {opened} opened but only {closed} closed \
         (a leaked spinner/bar would pile up on screen)"
    );
}
