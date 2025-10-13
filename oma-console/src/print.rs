use std::fmt::{self, Write};
use std::{borrow::Cow, time::Duration};

use chrono::{DateTime, SecondsFormat, Utc};
use console::{Color, StyledObject, style};
use spdlog::{Level, debug, formatter::Formatter};
use termbg::Theme;

pub use termbg;

use crate::writer::gen_prefix;

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
///
///   info!("My name is oma!");
///
///   Ok(())
/// }
/// ```
///
#[derive(Clone)]
pub struct OmaFormatter {
    with_ansi: bool,
    with_time: bool,
    with_file: bool,
    #[allow(unused)]
    with_kv: bool,
    prefix_len: u16,
}

impl Default for OmaFormatter {
    fn default() -> Self {
        Self {
            with_ansi: true,
            with_file: false,
            with_time: false,
            with_kv: false,
            prefix_len: 10,
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
        self
    }

    #[allow(unused)]
    pub fn with_kv(mut self, with_kv: bool) -> Self {
        self.with_kv = with_kv;
        self
    }

    fn format_impl(
        &self,
        record: &spdlog::Record,
        dest: &mut spdlog::StringBuf,
        _: &mut spdlog::formatter::FormatterContext,
    ) -> fmt::Result {
        let level = record.level();

        let prefix = if self.with_ansi {
            Cow::Owned(match level {
                Level::Debug => console::style("DEBUG").dim().to_string(),
                Level::Info => console::style("INFO").blue().bold().to_string(),
                Level::Warn => console::style("WARNING").yellow().bold().to_string(),
                Level::Error => console::style("ERROR").red().bold().to_string(),
                Level::Trace => console::style("TRACE").dim().to_string(),
                Level::Critical => console::style("CRITICAL").red().bright().bold().to_string(),
            })
        } else {
            Cow::Borrowed(match level {
                Level::Debug => "DEBUG",
                Level::Info => "INFO",
                Level::Warn => "WARNING",
                Level::Error => "ERROR",
                Level::Trace => "TRACE",
                Level::Critical => "CRITICAL",
            })
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

            dest.write_str(&time)?;
            dest.write_char(' ')?;
        };

        dest.write_str(&gen_prefix(&prefix, self.prefix_len))?;
        dest.write_char(' ')?;

        if self.with_file {
            let loc = record.source_location();

            if let Some(loc) = loc {
                let loc = format!("{}: {}:", loc.module_path(), loc.file());

                let loc = if self.with_ansi {
                    console::style(loc).dim().to_string()
                } else {
                    loc
                };

                dest.write_str(&loc)?;
                dest.write_char(' ')?;
            }
        }

        if self.with_ansi {
            dest.write_str(record.payload())?;
        } else {
            dest.write_str(&console::strip_ansi_codes(record.payload()))?;
        }

        dest.write_char('\n')?;

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
