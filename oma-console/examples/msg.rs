use oma_console::{info, writer::WRITER, msg, debug, warn, error, due_to, success};

fn main() {
    msg!(WRITER, "Welcome");
    debug!(WRITER, "Hello");
    info!(WRITER, "I'am fine");
    warn!(WRITER, "Thank you");
    error!(WRITER, "and you?");
    due_to!(WRITER, "QAQ");
    success!(WRITER, "PAP");
}