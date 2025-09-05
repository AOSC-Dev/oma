#[cfg(feature = "spdlog-rs")]
use oma_console::OmaFormatter;

#[cfg(not(feature = "spdlog-rs"))]
use oma_console::OmaLayer;
#[cfg(not(feature = "spdlog-rs"))]
use tracing_subscriber::{prelude::__tracing_subscriber_SubscriberExt, util::SubscriberInitExt};

#[cfg(not(feature = "spdlog-rs"))]
fn main() {
    tracing_subscriber::registry().with(OmaLayer::new()).init();
    tracing::info!("Welcome");
    tracing::debug!("Hello");
    tracing::info!("I'am fine");
    tracing::warn!("Thank you");
    tracing::error!("and you?");
}

#[cfg(feature = "spdlog-rs")]
fn main() {
    spdlog::default_logger()
        .sinks()
        .iter()
        .for_each(|s| s.set_formatter(Box::new(OmaFormatter::default())));
    spdlog::default_logger().set_level_filter(spdlog::LevelFilter::All);

    spdlog::info!("Welcome");
    spdlog::debug!("Hello");
    spdlog::info!("I'am fine");
    spdlog::warn!("Thank you");
    spdlog::error!("and you?");
}
