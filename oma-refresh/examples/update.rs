use std::{path::Path, result::Result};

use oma_refresh::db::{OmaRefresh, RefreshError};

#[tokio::main]
async fn main() -> Result<(), RefreshError> {
    let mut refresher = OmaRefresh::scan(None)?;
    let p = Path::new("./oma-fetcher-test");
    tokio::fs::create_dir_all(p).await.unwrap();
    refresher.download_dir(p);
    refresher.start().await.unwrap();

    Ok(())
}
