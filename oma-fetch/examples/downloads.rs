use std::path::PathBuf;

use oma_fetch::{
    DownloadEntry, DownloadManager, DownloadSource, DownloadSourceType, Event, checksum::Checksum,
};
use reqwest::ClientBuilder;

#[tokio::main]
async fn main() {
    rustls::crypto::ring::default_provider()
        .install_default()
        .unwrap();

    let source_1 = DownloadSource {
        url: "https://mirrors.jlu.edu.cn/anthon/aosc-os/os-amd64/base/aosc-os_base_20260312_amd64.squashfs".to_string(),
        source_type: DownloadSourceType::Http,
    };

    let file_1 = DownloadEntry::builder()
        .source(vec![source_1])
        .filename("aosc-os_base_20260312_amd64.squashfs".to_string())
        .dir(PathBuf::from("./oma-fetcher-test/partial"))
        .hash(
            Checksum::from_sha256_str(
                "675eef205388e2f3afb27ddbcbb61856f497986c2813d04b4e983078bb464d33",
            )
            .unwrap(),
        )
        .final_dir(PathBuf::from("./oma-fetcher-test"))
        .allow_resume(true)
        .build();

    let source_2 = DownloadSource {
        url: "https://mirrors.jlu.edu.cn/anthon/oma/pool/beige/main/o/oma_1.27.0~rc.1-1_amd64-deepin23.deb".to_string(),
        source_type: DownloadSourceType::Http,
    };

    let file_2 = DownloadEntry::builder()
        .source(vec![source_2])
        .filename("oma_1.27.0~rc.1-1_amd64-deepin23.deb".to_string().into())
        .dir(PathBuf::from("./oma-fetcher-test/partial"))
        .hash(
            Checksum::from_sha256_str(
                "82d61126e3a01506726bf7863cd2d7fbbfbf51c5674bd6fc43854265fc0aedcf",
            )
            .unwrap(),
        )
        .final_dir(PathBuf::from("./oma-fetcher-test"))
        .allow_resume(true)
        .build();

    // let mut test_local_file = tokio::fs::File::create("test_file").await.unwrap();
    // test_local_file.write_all(b"test").await.unwrap();

    let client = ClientBuilder::new().user_agent("oma").build().unwrap();

    let (tx, rx) = flume::unbounded();

    let binding = [file_1, file_2];
    let download_manager = DownloadManager::builder()
        .client(client.into())
        .download_list(Box::new(binding))
        .build();

    let event_worker = tokio::spawn(async move {
        while let Ok(event) = rx.recv_async().await {
            println!("{:?}", event);
            if let Event::AllDone = event {
                break;
            }
        }
    });

    tokio::fs::create_dir_all("./oma-fetcher-test")
        .await
        .unwrap();

    let summary = download_manager
        .start_download(move |event| {
            let tx = tx.clone();
            async move {
                if let Err(e) = tx.send_async(event).await {
                    eprintln!("Got Error: {:#?}", e);
                }
            }
        })
        .await;

    let _ = event_worker.await;

    println!("{:#?}", summary);
}
