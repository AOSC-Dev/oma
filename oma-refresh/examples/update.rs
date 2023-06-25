use std::result::Result;

use oma_refresh::db::{RefreshError, update_db, get_sources, Event};
use tokio::sync::mpsc::UnboundedSender;

#[tokio::main]
async fn main() -> Result<(), RefreshError> {
    let (tx, rx): (UnboundedSender<Event>, _) = tokio::sync::mpsc::unbounded_channel();

    let sources = get_sources()?;
    update_db(&sources, None, tx, "amd64").await?;
    Ok(())
}