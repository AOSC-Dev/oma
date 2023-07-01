use std::{result::Result, path::Path};

use oma_refresh::db::{RefreshError, OmaRefresh};
use tracing_subscriber::{prelude::__tracing_subscriber_SubscriberExt, fmt, EnvFilter, util::SubscriberInitExt};

#[tokio::main]
async fn main() -> Result<(), RefreshError> {
    tracing_subscriber::registry()
    .with(fmt::layer())
    .with(EnvFilter::from_default_env())
    .init();

    let mut refresher = OmaRefresh::scan(None)?;
    let p = Path::new("./oma-fetcher-test");
    tokio::fs::create_dir_all(p).await.unwrap();
    refresher.download_dir(p);
    refresher.start().await.unwrap();

    Ok(())
}