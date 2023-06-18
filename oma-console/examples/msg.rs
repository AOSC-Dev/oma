use oma_console::{debug, due_to, error, info, msg, success, warn, writer::Writer};

fn main() {
    let writer = Writer::default();
    msg!(writer, "Welcome");
    debug!(writer, "Hello");
    info!(writer, "I'am fine");
    warn!(writer, "Thank you");
    error!(writer, "and you?");
    due_to!(writer, "QAQ");
    success!(writer, "PAP");
}
