use std::{path::Path, result::Result};

use oma_fetch::reqwest::ClientBuilder;
use oma_refresh::db::{Event, OmaRefresh, RefreshError};
use oma_utils::dpkg::dpkg_arch;

fn main() -> Result<(), RefreshError> {
    rustls::crypto::ring::default_provider()
        .install_default()
        .unwrap();
    let p = Path::new("./oma-fetcher-test");
    std::fs::create_dir_all(p).unwrap();
    let client = ClientBuilder::new().user_agent("oma").build().unwrap();

    let (tx, rx) = flume::unbounded();

    let refresh = OmaRefresh::builder()
        .client(client.into())
        .arch(dpkg_arch("/").unwrap())
        .download_dir(p.to_path_buf())
        .source("/".into())
        .topic_msg("test".into())
        .refresh_topics(false)
        .build();

    std::thread::spawn(move || {
        while let Ok(event) = rx.recv() {
            println!("{:#?}", event);
            if let Event::Done = event {
                break;
            }
        }
    });

    refresh.start(move |event| {
        if let Err(e) = tx.send(event) {
            eprintln!("{:#?}", e);
        }
    })?;

    Ok(())
}
