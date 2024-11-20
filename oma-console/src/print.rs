use std::{collections::BTreeMap, time::Duration};

use console::{style, Color, StyledObject};
use termbg::Theme;
use tracing::{debug, field::Field, Level};
use tracing_subscriber::Layer;

pub use termbg;

use crate::{writer::Writeln, WRITER};

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
/// `OmaColorFormat` is a structure that defines the color format and theme settings for the Oma application.
///
/// # Fields
///
/// * `follow` - A `StyleFollow` enum that indicates whether to follow the terminal theme or use the Oma theme.
/// * `theme` - An optional `Theme` object that defines the color scheme for the Oma application.
pub struct OmaColorFormat {
    follow: StyleFollow,
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
    /// This function applies a color to the given input string based on the specified action and the current theme settings.
    ///
    /// # Arguments
    ///
    /// * `input` - The input data to be styled.
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
        let level = *event.metadata().level();
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
/// OmaRecorder
struct OmaRecorder<'a>(BTreeMap<&'a str, String>);

impl<'a> tracing::field::Visit for OmaRecorder<'a> {
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

/// oma display normal message
#[macro_export]
macro_rules! msg {
    ($($arg:tt)+) => {
        use oma_console::writer::Writeln as _;
        oma_console::WRITER.writeln("", &format!($($arg)+)).ok();
    };
}

/// oma display success message
#[macro_export]
macro_rules! success {
    ($($arg:tt)+) => {
        use oma_console::writer::Writeln as _;
        oma_console::WRITER.writeln(&oma_console::console::style("SUCCESS").green().bold().to_string(), &format!($($arg)+)).ok();
    };
}

/// oma display due_to message
#[macro_export]
macro_rules! due_to {
    ($($arg:tt)+) => {
        use oma_console::writer::Writeln as _;
        oma_console::WRITER.writeln(&oma_console::console::style("DUE TO").yellow().bold().to_string(), &format!($($arg)+)).ok();
    };
}
