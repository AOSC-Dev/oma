use std::{path::Path, result::Result};

use apt_auth_config::AuthConfig;
use oma_apt::config::Config;
use oma_fetch::reqwest::ClientBuilder;
use oma_refresh::db::{Event, OmaRefresh, RefreshError};
use oma_utils::dpkg::dpkg_arch;

#[tokio::main]
async fn main() -> Result<(), RefreshError> {
    let p = Path::new("./oma-fetcher-test");
    tokio::fs::create_dir_all(p).await.unwrap();
    let client = ClientBuilder::new().user_agent("oma").build().unwrap();

    let apt_config = Config::new();

    let auth = AuthConfig::system("/").unwrap();

    let (tx, rx) = flume::unbounded();

    let refresh = OmaRefresh::builder()
        .client(&client)
        .apt_config(&apt_config)
        .arch(dpkg_arch("/").unwrap())
        .download_dir(p.to_path_buf())
        .source("/".into())
        .topic_msg("test")
        .refresh_topics(false)
        .auth_config(&auth)
        .another_apt_options(vec![])
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
        .start(|event| async {
            if let Err(e) = tx.send_async(event).await {
                eprintln!("{:#?}", e);
            }
        })
        .await?;

    Ok(())
}
