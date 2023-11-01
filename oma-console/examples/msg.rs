use oma_console::{OmaLayer, due_to, success, DEBUG};

fn main() {
    tracing::info!("Welcome");
    tracing::debug!("Hello");
    tracing::info!("I'am fine");
    tracing::warn!("Thank you");
    tracing::error!("and you?");
    due_to!("QAQ");
    success!("PAP");
}
