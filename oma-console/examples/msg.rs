use oma_console::OmaLayer;
use tracing_subscriber::{prelude::__tracing_subscriber_SubscriberExt, util::SubscriberInitExt};

fn main() {
    tracing_subscriber::registry().with(OmaLayer::new()).init();
    tracing::info!("Welcome");
    tracing::debug!("Hello");
    tracing::info!("I'am fine");
    tracing::warn!("Thank you");
    tracing::error!("and you?");
}
