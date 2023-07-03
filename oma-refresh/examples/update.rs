use std::{path::Path, result::Result};

use oma_refresh::db::{OmaRefresh, RefreshError};
use tracing_subscriber::{
    fmt, prelude::__tracing_subscriber_SubscriberExt, util::SubscriberInitExt, EnvFilter,
};

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
