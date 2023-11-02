pub mod pager;
pub mod pb;
pub mod writer;
use std::{
    collections::BTreeMap,
    sync::atomic::{AtomicBool, Ordering},
};
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
    fn on_event(&self, event: &tracing::Event<'_>, ctx: tracing_subscriber::layer::Context<'_, S>) {
        let scope = ctx.event_scope(event);
        let mut spans = vec![];
        if let Some(scope) = scope {
            for span in scope.from_root() {
                let extensions = span.extensions();
                let storage = extensions.get::<CustomFieldStorage>().unwrap();
                let field_data: &BTreeMap<String, String> = &storage.0;
                spans.push(serde_json::json!({
                    "target": span.metadata().target(),
                    "name": span.name(),
                    "level": format!("{:?}", span.metadata().level()),
                    "fields": field_data,
                }));
            }
        }

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

        if !DEBUG.load(Ordering::Relaxed) {
            for (k, v) in visitor.0 {
                if k == "message" {
                    WRITER.writeln(&prefix, &v).ok();
                }
            }
        } else {
            let json = serde_json::json!({
                "Level": level.to_string(),
                "data": visitor.0,
                "spans": spans,
            });

            let output = serde_json::to_string_pretty(&json).unwrap();
            println!("{output}");
        }
    }

    fn on_new_span(
        &self,
        attrs: &tracing::span::Attributes<'_>,
        id: &tracing::span::Id,
        ctx: tracing_subscriber::layer::Context<'_, S>,
    ) {
        // 基于 field 值来构建我们自己的 JSON 对象
        let fields = BTreeMap::new();
        let mut visitor = OmaRecorder(fields.clone());
        attrs.record(&mut visitor);

        // 使用之前创建的 newtype 包裹下
        let storage = CustomFieldStorage(fields);

        // 获取内部 span 数据的引用
        let span = ctx.span(id).unwrap();
        // 获取扩展，用于存储我们的 span 数据
        let mut extensions = span.extensions_mut();
        // 存储！
        extensions.insert::<CustomFieldStorage>(storage);
    }

    fn on_record(
        &self,
        id: &tracing::span::Id,
        values: &tracing::span::Record<'_>,
        ctx: tracing_subscriber::layer::Context<'_, S>,
    ) {
        // 获取正在记录数据的 span
        let span = ctx.span(id).unwrap();

        // 获取数据的可变引用，该数据是在 on_new_span 中创建的
        let mut extensions_mut = span.extensions_mut();
        let custom_field_storage: &mut CustomFieldStorage =
            extensions_mut.get_mut::<CustomFieldStorage>().unwrap();
        let json_data: &BTreeMap<String, String> = &custom_field_storage.0;

        // 使用我们的访问器老朋友
        let mut visitor = OmaRecorder(json_data.clone());
        values.record(&mut visitor);
    }
}

struct OmaRecorder(BTreeMap<String, String>);
#[derive(Debug)]
struct CustomFieldStorage(BTreeMap<String, String>);

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
