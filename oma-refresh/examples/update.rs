use std::{result::Result, path::Path};

use oma_refresh::db::{RefreshError, OmaRefresh};

#[tokio::main]
async fn main() -> Result<(), RefreshError> {
    let p = Path::new("./oma-fetcher-test");
    tokio::fs::create_dir_all(p).await.unwrap();
    let mut refresher = OmaRefresh::scan(None)?;
    refresher.download_dir(p);
    refresher.start().await?;

    Ok(())
}
