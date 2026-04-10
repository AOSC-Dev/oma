use std::{path::Path, result::Result, sync::Arc};

use apt_auth_config::AuthConfig;
use oma_fetch::reqwest::ClientBuilder;
use oma_refresh::db::{Event, OmaRefresh, RefreshError};
use oma_utils::dpkg::dpkg_arch;

#[tokio::main]
async fn main() -> Result<(), RefreshError> {
    let p = Path::new("./oma-fetcher-test");
    tokio::fs::create_dir_all(p).await.unwrap();
    let client = ClientBuilder::new().user_agent("oma").build().unwrap();

    let auth = Arc::new(AuthConfig::system("/").ok());

    let (tx, rx) = flume::unbounded();

    let refresh = OmaRefresh::builder()
        .client(client)
        .arch(dpkg_arch("/").unwrap())
        .download_dir(p.to_path_buf())
        .source("/".into())
        .topic_msg("test".into())
        .refresh_topics(false)
        .auth_config(auth)
        .build();

    tokio::spawn(async move {
        while let Ok(event) = rx.recv_async().await {
            println!("{:#?}", event);
            if let Event::Done = event {
                break;
            }
        }
    });

    refresh
        .start(async move |event| {
            if let Err(e) = tx.send_async(event).await {
                eprintln!("{:#?}", e);
            }
        })
        .await?;

    Ok(())
}
