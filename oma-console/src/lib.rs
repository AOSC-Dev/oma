pub mod pager;
pub mod pb;
pub mod writer;
use std::{collections::BTreeMap, sync::atomic::AtomicBool};
use tracing::{field::Field, Level};
use tracing_subscriber::Layer;

pub use console;
pub use indicatif;
use once_cell::sync::Lazy;
use writer::Writer;

pub static WRITER: Lazy<Writer> = Lazy::new(writer::Writer::default);
pub static DEBUG: AtomicBool = AtomicBool::new(false);

pub fn is_terminal() -> bool {
    WRITER.is_terminal()
}

pub struct OmaLayer;

impl<S> Layer<S> for OmaLayer
where
    S: tracing::Subscriber,
    S: for<'lookup> tracing_subscriber::registry::LookupSpan<'lookup>,
{
    fn on_event(
        &self,
        event: &tracing::Event<'_>,
        _ctx: tracing_subscriber::layer::Context<'_, S>,
    ) {
        let level = event.metadata().level().to_owned();
        let prefix = match level {
            Level::DEBUG => console::style("DEBUG").dim().to_string(),
            Level::INFO => console::style("INFO").blue().bold().to_string(),
            Level::WARN => console::style("WARNING").yellow().bold().to_string(),
            Level::ERROR => console::style("ERROR").red().bold().to_string(),
            Level::TRACE => console::style("TRACE").dim().to_string(),
        };

        let mut visitor = OmaRecorder(BTreeMap::new());
        event.record(&mut visitor);

        for (k, v) in visitor.0 {
            if k == "message" {
                WRITER.writeln(&prefix, &v).ok();
            }
        }
    }
}

struct OmaRecorder(BTreeMap<String, String>);

impl tracing::field::Visit for OmaRecorder {
    fn record_f64(&mut self, field: &Field, value: f64) {
        self.0.insert(field.name().to_owned(), value.to_string());
    }

    fn record_i64(&mut self, field: &Field, value: i64) {
        self.0.insert(field.name().to_owned(), value.to_string());
    }

    fn record_u64(&mut self, field: &Field, value: u64) {
        self.0.insert(field.name().to_owned(), value.to_string());
    }

    fn record_bool(&mut self, field: &Field, value: bool) {
        self.0.insert(field.name().to_owned(), value.to_string());
    }

    fn record_str(&mut self, field: &Field, value: &str) {
        self.0.insert(field.name().to_owned(), value.to_string());
    }

    fn record_error(&mut self, field: &Field, value: &(dyn std::error::Error + 'static)) {
        self.0
            .insert(field.name().to_owned(), format!("{value:#?}"));
    }

    fn record_debug(&mut self, field: &Field, value: &dyn std::fmt::Debug) {
        self.0
            .insert(field.name().to_owned(), format!("{value:#?}"));
    }
}

/// oma display normal message
#[macro_export]
macro_rules! msg {
    ($($arg:tt)+) => {
        oma_console::WRITER.writeln("", &format!($($arg)+)).ok();
    };
}

/// oma display success message
#[macro_export]
macro_rules! success {
    ($($arg:tt)+) => {
        oma_console::WRITER.writeln(&oma_console::console::style("SUCCESS").green().bold().to_string(), &format!($($arg)+)).ok();
    };
}

/// oma display due_to message
#[macro_export]
macro_rules! due_to {
    ($($arg:tt)+) => {
        oma_console::WRITER.writeln(&oma_console::console::style("DUE TO").yellow().bold().to_string(), &format!($($arg)+)).ok();
    };
}
