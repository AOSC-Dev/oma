use std::{borrow::Cow, collections::BTreeMap, time::Duration};

use console::{Color, StyledObject, style};
use termbg::Theme;
use tracing::{Level, debug, field::Field};
use tracing_subscriber::Layer;

pub use termbg;

use crate::writer::{Writeln, Writer};

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
/// OmaLayer
/// `OmaLayer` is used for outputting oma-style logs to `tracing`
///
/// # Example:
/// ```
/// use tracing_subscriber::prelude::*;
/// use oma_console::OmaLayer;
/// use tracing::info;
///
/// tracing_subscriber::registry()
///     .with(OmaLayer::new())
///     .init();
///
/// info!("My name is oma!");
/// ```
///
pub struct OmaLayer {
    /// Display result with ansi
    with_ansi: bool,
    /// A Terminal writer to print oma-style message
    writer: Writer,
}

impl Default for OmaLayer {
    fn default() -> Self {
        Self {
            with_ansi: true,
            writer: Writer::default(),
        }
    }
}

impl OmaLayer {
    pub fn new() -> Self {
        OmaLayer::default()
    }

    /// Display with ANSI colors
    ///
    /// Set to false to disable ANSI color sequences.
    pub fn with_ansi(mut self, with_ansi: bool) -> Self {
        self.with_ansi = with_ansi;
        self
    }
}

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
        let level = *event.metadata().level();

        let prefix = if self.with_ansi {
            Cow::Owned(match level {
                Level::DEBUG => console::style("DEBUG").dim().to_string(),
                Level::INFO => console::style("INFO").blue().bold().to_string(),
                Level::WARN => console::style("WARNING").yellow().bold().to_string(),
                Level::ERROR => console::style("ERROR").red().bold().to_string(),
                Level::TRACE => console::style("TRACE").dim().to_string(),
            })
        } else {
            Cow::Borrowed(match level {
                Level::DEBUG => "DEBUG",
                Level::INFO => "INFO",
                Level::WARN => "WARNING",
                Level::ERROR => "ERROR",
                Level::TRACE => "TRACE",
            })
        };

        let mut visitor = OmaRecorder(BTreeMap::new());
        event.record(&mut visitor);

        for (k, v) in visitor.0 {
            if k == "message" {
                if self.with_ansi {
                    self.writer.writeln(&prefix, &v).ok();
                } else {
                    self.writer
                        .writeln(&prefix, &console::strip_ansi_codes(&v))
                        .ok();
                }
            }
        }
    }
}
/// OmaRecorder
/// `OmaRecorder` is used for recording oma-style logs.
///
/// # Example:
/// ```ignore
/// let mut visitor = OmaRecorder(BTreeMap::new());
/// event.record(&mut visitor);
/// for (k, v) in visitor.0 {
///     if k == "message" {
///         self.writer.writeln(&prefix, &v).ok();
///     }
/// }
/// ```
struct OmaRecorder<'a>(BTreeMap<&'a str, String>);

impl tracing::field::Visit for OmaRecorder<'_> {
    fn record_f64(&mut self, field: &Field, value: f64) {
        self.0.insert(field.name(), value.to_string());
    }

    fn record_i64(&mut self, field: &Field, value: i64) {
        self.0.insert(field.name(), value.to_string());
    }

    fn record_u64(&mut self, field: &Field, value: u64) {
        self.0.insert(field.name(), value.to_string());
    }

    fn record_bool(&mut self, field: &Field, value: bool) {
        self.0.insert(field.name(), value.to_string());
    }

    fn record_str(&mut self, field: &Field, value: &str) {
        self.0.insert(field.name(), value.to_string());
    }

    fn record_error(&mut self, field: &Field, value: &(dyn std::error::Error + 'static)) {
        self.0.insert(field.name(), format!("{value:#?}"));
    }

    fn record_debug(&mut self, field: &Field, value: &dyn std::fmt::Debug) {
        self.0.insert(field.name(), format!("{value:#?}"));
    }
}
