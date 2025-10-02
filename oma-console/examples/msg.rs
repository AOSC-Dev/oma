use oma_console::OmaFormatter;

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
