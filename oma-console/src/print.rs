use std::fmt::{self, Write};
use std::sync::LazyLock;

use jiff::Timestamp;
use spdlog::{Level, formatter::Formatter};

pub use termbg;

use crate::terminal::Terminal;

static PREFIX_DEBUG: LazyLock<String> = LazyLock::new(|| console::style("DEBUG").dim().to_string());
static PREFIX_INFO: LazyLock<String> =
    LazyLock::new(|| console::style("INFO").blue().bold().to_string());
static PREFIX_WARN: LazyLock<String> =
    LazyLock::new(|| console::style("WARNING").yellow().bold().to_string());
static PREFIX_ERROR: LazyLock<String> =
    LazyLock::new(|| console::style("ERROR").red().bold().to_string());
static PREFIX_TRACE: LazyLock<String> = LazyLock::new(|| console::style("TRACE").dim().to_string());
static PREFIX_CRITICAL: LazyLock<String> =
    LazyLock::new(|| console::style("CRITICAL").red().bright().bold().to_string());

const TIME_RFC3339_LEN: u16 = "1970-01-01T00:00:00.000Z".len() as u16 + 1;

/// OmaFormatter
/// `OmaFormatter` is used for outputting oma-style logs to `spdlog-rs`
///
/// # Example:
/// ```
/// use spdlog::{info, sink::StdStreamSink, Logger, Result};
/// use oma_console::OmaFormatter;
///
/// use std::sync::Arc;
///
/// fn main() -> Result<()> {
///   let mut logger_builder = Logger::builder();
///
///   let stream_sink = StdStreamSink::builder()
///     .formatter(OmaFormatter::default())
///     .stdout()
///     .build()?;
///
///   let logger = logger_builder.sink(Arc::new(stream_sink)).build()?;
///
///   spdlog::set_default_logger(Arc::new(logger));
///   info!("My name is oma!");
///   Ok(())
/// }
/// ```
#[derive(Clone)]
pub struct OmaFormatter {
    /// Display result with ansi
    with_ansi: bool,
    with_time: bool,
    with_file: bool,
    #[allow(unused)]
    with_kv: bool,
    term: Terminal,
    debug: bool,
}

impl Default for OmaFormatter {
    fn default() -> Self {
        Self {
            with_ansi: true,
            with_file: false,
            with_time: false,
            with_kv: false,
            debug: false,
            term: Terminal::default(),
        }
    }
}

impl OmaFormatter {
    pub fn new() -> Self {
        OmaFormatter::default()
    }

    /// Display with ANSI colors
    ///
    /// Set to false to disable ANSI color sequences.
    pub fn with_ansi(mut self, with_ansi: bool) -> Self {
        self.with_ansi = with_ansi;
        self
    }

    pub fn with_file(mut self, with_file: bool) -> Self {
        self.with_file = with_file;
        self
    }

    pub fn with_time(mut self, with_time: bool) -> Self {
        self.with_time = with_time;
        self.term.prefix_len += TIME_RFC3339_LEN;
        self
    }

    #[allow(unused)]
    pub fn with_kv(mut self, with_kv: bool) -> Self {
        self.with_kv = with_kv;
        self
    }

    pub fn with_max_len(mut self, max_len: Option<u16>) -> Self {
        self.term.limit_max_len = max_len;
        self
    }

    pub fn with_debug(mut self, debug: bool) -> Self {
        self.debug = debug;
        self
    }

    pub fn with_prefix_len(mut self, prefix_len: u16) -> Self {
        self.term.prefix_len = if self.with_time {
            prefix_len + TIME_RFC3339_LEN
        } else {
            prefix_len
        };

        self
    }

    pub fn with_term(mut self, term: Terminal) -> Self {
        self.term = term;
        self
    }

    pub fn get_term(&self) -> &Terminal {
        &self.term
    }

    fn format_impl(
        &self,
        record: &spdlog::Record,
        dest: &mut spdlog::StringBuf,
        _: &mut spdlog::formatter::FormatterContext,
    ) -> fmt::Result {
        let level = record.level();

        let mut prefix = String::with_capacity(8);

        let prefix_str = if self.with_ansi {
            match level {
                Level::Debug => &*PREFIX_DEBUG,
                Level::Info => &*PREFIX_INFO,
                Level::Warn => &*PREFIX_WARN,
                Level::Error => &*PREFIX_ERROR,
                Level::Trace => &*PREFIX_TRACE,
                Level::Critical => &*PREFIX_CRITICAL,
            }
        } else {
            match level {
                Level::Debug => "DEBUG",
                Level::Info => "INFO",
                Level::Warn => "WARNING",
                Level::Error => "ERROR",
                Level::Trace => "TRACE",
                Level::Critical => "CRITICAL",
            }
        };

        if self.with_time {
            let time = {
                let time = format!(
                    "{:.3}",
                    Timestamp::try_from(record.time()).unwrap_or_default()
                );

                if self.with_ansi {
                    console::style(time).dim().to_string()
                } else {
                    time
                }
            };

            prefix.write_str(&time)?;
            prefix.write_char(' ')?;
        };

        prefix.write_str(prefix_str)?;

        let mut body = String::new();

        if self.with_file {
            let loc = record.source_location();

            if let Some(loc) = loc {
                let loc = format!("{}: {}:", loc.module_path(), loc.file());

                let loc = if self.with_ansi {
                    console::style(loc).dim().to_string()
                } else {
                    loc
                };

                body.write_str(&loc)?;
                body.write_char(' ')?;
            }
        }

        if self.with_ansi {
            body.write_str(record.payload())?;
        } else {
            body.write_str(&console::strip_ansi_codes(record.payload()))?;
        }

        if self.debug {
            dest.write_str(&prefix)?;
            dest.write_str(" ")?;
            dest.write_str(&body)?;
            writeln!(dest)?;
        } else {
            for (prefix, body) in self.term.wrap_content(&prefix, &body).into_iter() {
                dest.write_str(&self.term.gen_prefix(prefix))?;
                dest.write_str(&body)?;
            }
        }

        Ok(())
    }
}

impl Formatter for OmaFormatter {
    fn format(
        &self,
        record: &spdlog::Record,
        dest: &mut spdlog::StringBuf,
        ctx: &mut spdlog::formatter::FormatterContext,
    ) -> spdlog::Result<()> {
        self.format_impl(record, dest, ctx)
            .map_err(spdlog::Error::FormatRecord)
    }
}
