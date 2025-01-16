use std::path::PathBuf;

use oma_fetch::{
    checksum::Checksum, DownloadEntry, DownloadManager, DownloadSource, DownloadSourceType, Event,
};
use reqwest::ClientBuilder;

#[tokio::main]
async fn main() {
    let source_1 = DownloadSource {
        url: "https://mirrors.bfsu.edu.cn/anthon/mascots/zhaxia-stickers-v1.zip".to_string(),
        source_type: DownloadSourceType::Http { auth: None },
    };

    let file_1 = DownloadEntry::builder()
        .source(vec![source_1])
        .filename("zhaxia-stickers-v1.zip".to_string().into())
        .dir(PathBuf::from("./oma-fetcher-test"))
        .hash(
            Checksum::from_sha256_str(
                "de700bdb45a4b8ab322d7eb2d30a7b448f117693b5a4164a3f05345177884134",
            )
            .unwrap(),
        )
        .allow_resume(true)
        .build();

    let source_2 = DownloadSource {
        url: "https://mirrors.bfsu.edu.cn/anthon/mascots/mascots.zip".to_string(),
        source_type: DownloadSourceType::Http { auth: None },
    };

    let file_2 = DownloadEntry::builder()
        .source(vec![source_2])
        .filename("mascots.zip".to_string().into())
        .dir(PathBuf::from("./oma-fetcher-test"))
        .allow_resume(false)
        .build();

    // let mut test_local_file = tokio::fs::File::create("test_file").await.unwrap();
    // test_local_file.write_all(b"test").await.unwrap();

    let client = ClientBuilder::new().user_agent("oma").build().unwrap();

    let (tx, rx) = flume::unbounded();

    let binding = [file_1, file_2];
    let download_manager = DownloadManager::builder()
        .client(&client)
        .download_list(&binding)
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
        .start_download(|event| async {
            if let Err(e) = tx.send_async(event).await {
                eprintln!("Got Error: {:#?}", e);
            }
        })
        .await;

    let _ = event_worker.await;

    println!("{:#?}", summary);
}
