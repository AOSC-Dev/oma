use std::fmt::{self, Write};
use std::sync::LazyLock;
use std::time::Duration;

use chrono::{DateTime, SecondsFormat, Utc};
use console::{Color, StyledObject, style};
use spdlog::{Level, debug, formatter::Formatter};
use termbg::Theme;

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

#[derive(Clone)]
enum StyleFollow {
    OmaTheme,
    TermTheme,
}

pub enum Action {
    Emphasis,
    Foreground,
    Secondary,
    EmphasisSecondary,
    WARN,
    Purple,
    Note,
    UpgradeTips,
    PendingBg,
}

impl Action {
    fn dark(&self) -> u8 {
        match self {
            Action::Emphasis => 148,
            Action::Foreground => 72,
            Action::Secondary => 182,
            Action::EmphasisSecondary => 114,
            Action::WARN => 214,
            Action::Purple => 141,
            Action::Note => 178,
            Action::UpgradeTips => 87,
            Action::PendingBg => 25,
        }
    }

    fn light(&self) -> u8 {
        match self {
            Action::Emphasis => 142,
            Action::Foreground => 72,
            Action::Secondary => 167,
            Action::EmphasisSecondary => 106,
            Action::WARN => 208,
            Action::Purple => 141,
            Action::Note => 172,
            Action::UpgradeTips => 63,
            Action::PendingBg => 189,
        }
    }
}
/// OmaColorFormat
///
/// `OmaColorFormat` is a structure that defines the color format and theme settings for oma.
pub struct OmaColorFormat {
    /// A `StyleFollow` enum that indicates whether to follow the terminal theme or use the oma-defined theme.
    follow: StyleFollow,
    /// An optional `Theme` object that defined by oma.
    pub theme: Option<Theme>,
}

impl OmaColorFormat {
    pub fn new(follow: bool, duration: Duration) -> Self {
        Self {
            follow: if follow {
                StyleFollow::TermTheme
            } else {
                StyleFollow::OmaTheme
            },
            theme: if !follow {
                termbg::theme(duration)
                    .map_err(|e| {
                        debug!(
                            "Failed to apply oma color schemes, falling back to default terminal colors: {e:?}."
                        );
                        e
                    })
                    .ok()
            } else {
                None
            },
        }
    }
    /// Convert input into StyledObject
    ///
    /// This function applies a color scheme to the given input string based on the specified action and the current terminal color schemes.
    ///
    /// # Arguments
    ///
    /// * `input` - The input data to be themed.
    /// * `color` - An `Action` enum value that specifies the color to be applied.
    ///
    /// # Returns
    ///
    /// Returns a `StyledObject` that contains the styled input data.
    pub fn color_str<D>(&self, input: D, color: Action) -> StyledObject<D> {
        match self.follow {
            StyleFollow::OmaTheme => match self.theme {
                Some(Theme::Dark) => match color {
                    x @ Action::PendingBg => style(input).bg(Color::Color256(x.dark())).bold(),
                    x => style(input).color256(x.dark()),
                },
                Some(Theme::Light) => match color {
                    x @ Action::PendingBg => style(input).bg(Color::Color256(x.light())).bold(),
                    x => style(input).color256(x.light()),
                },
                None => term_color(input, color),
            },
            StyleFollow::TermTheme => term_color(input, color),
        }
    }
}

fn term_color<D>(input: D, color: Action) -> StyledObject<D> {
    match color {
        Action::Emphasis => style(input).green(),
        Action::Secondary => style(input).dim(),
        Action::EmphasisSecondary => style(input).cyan(),
        Action::WARN => style(input).yellow().bold(),
        Action::Purple => style(input).magenta(),
        Action::Note => style(input).yellow(),
        Action::Foreground => style(input).cyan().bold(),
        Action::UpgradeTips => style(input).blue().bold(),
        Action::PendingBg => style(input).bg(Color::Blue).bold(),
    }
}

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
}

impl Default for OmaFormatter {
    fn default() -> Self {
        Self {
            with_ansi: true,
            with_file: false,
            with_time: false,
            with_kv: false,
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
        // '1970-01-01T00:00:00Z' counts 20 chars.
        self.term.prefix_len += 20;
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

    pub fn with_prefix_len(mut self, prefix_len: u16) -> Self {
        self.term.prefix_len = prefix_len;
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
                let time = DateTime::<Utc>::from(record.time())
                    .to_rfc3339_opts(SecondsFormat::Millis, true);

                if self.with_ansi {
                    console::style(time).dim().to_string()
                } else {
                    time
                }
            };

            prefix.write_str(&time)?;
            prefix.write_char(' ')?;
        };

        prefix.write_str(&prefix_str)?;

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

        for (prefix, body) in self.term.wrap_content(&prefix, &body).into_iter() {
            dest.write_str(&self.term.gen_prefix(prefix))?;
            dest.write_str(&body)?;
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
