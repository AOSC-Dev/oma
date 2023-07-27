use oma_console::{debug, due_to, error, info, msg, success, warn};

fn main() {
    oma_console::DEBUG.store(true, DebugOrdering::Relaxed);
    msg!("Welcome");
    debug!("Hello");
    info!("I'am fine");
    warn!("Thank you");
    error!("and you?");
    due_to!("QAQ");
    success!("PAP");
}
