//! Progress Bar
//! pb is used as an acronym for Progress Bar

use std::{fmt::Write, time::Duration};

use console::style;
use indicatif::{HumanBytes, ProgressState, ProgressStyle};

use crate::writer::Writer;

const SPINNER_ANIME: &[&str] = &[
    "( ●    )",
    "(  ●   )",
    "(   ●  )",
    "(    ● )",
    "(     ●)",
    "(    ● )",
    "(   ●  )",
    "(  ●   )",
    "( ●    )",
    "(●     )",
];
const GLOBAL_BAR_SMALL_TEMPLATE: &str = " {progress_msg:<59}";
const GLOBAL_BAR_TEMPLATE: &str =
    " {progress_msg:<59} {elapsed_precise:<11.blue.bold} [{wide_bar:.cyan/blue}] {percent:>4}";
const NORMAL_BAR_SMALL_TEMPLATE: &str = " {msg} {percent:>3}";
const NORMAL_BAR_TEMPLATE: &str =
    " {msg:<59} {total_bytes:<11} [{wide_bar:.white/black}] {percent:>4}";
const SPINNER_TEMPLATE: &str = " {msg:<59} {spinner}";
/// Progress Bar Style
/// Returns a 'ProgressStyle' object that defines the style of the progress bar.
///
/// # Arguments
///
/// * `writer` - A reference to a `Writer` object.
pub fn progress_bar_style(writer: &Writer) -> ProgressStyle {
    let max_len = writer.get_length();
    let template = if max_len < 100 {
        NORMAL_BAR_SMALL_TEMPLATE
    } else {
        NORMAL_BAR_TEMPLATE
    };

    ProgressStyle::default_bar()
        .template(template)
        .unwrap()
        .progress_chars("=>-")
        .with_key("percent", |state: &ProgressState, w: &mut dyn Write| {
            write!(w, "{:.*}%", 0, state.fraction() * 100f32).unwrap()
        })
}
/// Global Progress Bar Style
/// Returns a 'ProgressStyle' object that defines the style of the progress bar.
///
/// # Arguments
///
/// * `writer` - A reference to a `Writer` object.
pub fn global_progress_bar_style(writer: &Writer) -> ProgressStyle {
    let max_len = writer.get_length();
    let template = if max_len < 100 {
        GLOBAL_BAR_SMALL_TEMPLATE
    } else {
        GLOBAL_BAR_TEMPLATE
    };

    ProgressStyle::default_bar()
        .template(template)
        .unwrap()
        .progress_chars("=>-")
        .with_key("percent", |state: &ProgressState, w: &mut dyn Write| {
            write!(w, "{:.*}%", 0, state.fraction() * 100f32).unwrap()
        })
        .with_key("progress_msg", oma_global_bar_template)
}

fn oma_global_bar_template(state: &ProgressState, w: &mut dyn Write) {
    write!(w, "{}  ", style("Progress").blue().bold()).unwrap();

    write!(
        w,
        "{}",
        style(format!(
            "{} / {}",
            HumanBytes(state.pos()),
            HumanBytes(state.len().unwrap_or(0))
        ))
        .green()
        .bold()
    )
    .unwrap();

    write!(
        w,
        "{}",
        style(format!(" @ {}/s", HumanBytes(state.per_sec() as u64)))
            .green()
            .bold()
    )
    .unwrap();
}

pub fn spinner_style() -> (ProgressStyle, Duration) {
    let (template, inv) = (SPINNER_ANIME, 80);

    let style = ProgressStyle::with_template(SPINNER_TEMPLATE)
        .unwrap()
        .tick_strings(template);

    (style, Duration::from_millis(inv))
}
