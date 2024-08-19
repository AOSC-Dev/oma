use std::{fmt::Write, time::Duration};

use indicatif::{BinaryBytes, HumanBytes, ProgressState, ProgressStyle};

use crate::writer::Writer;

const SPINNER: &[&str] = &[
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

pub fn progress_bar_style(writer: &Writer) -> ProgressStyle {
    let max_len = writer.get_length();
    let template = if max_len < 100 {
        " {msg} {percent:>3}".to_owned()
    } else {
        " {msg:<59} {total_bytes:<11} [{wide_bar:.white/black}] {percent:>3}".to_owned()
    };

    ProgressStyle::default_bar()
        .template(&template)
        .unwrap()
        .progress_chars("=>-")
        .with_key("percent", |state: &ProgressState, w: &mut dyn Write| {
            write!(w, "{:.*}%", 0, state.fraction() * 100f32).unwrap()
        })
}

pub fn global_progress_bar_style(writer: &Writer) -> ProgressStyle {
    let max_len = writer.get_length();
    let template = if max_len < 100 {
        " {prefix:.blue.bold} {bytes:>14.green.bold} {total_bytes:.green.bold} {binary_bytes_per_sec:<10.green.bold}".to_owned()
    } else {
        " {prefix:.blue.bold} {bytes:>14.green.bold} {total_bytes:>8.green.bold} {binary_bytes_per_sec:<25.green.bold} {eta_precise:<10.blue.bold} [{wide_bar:.blue.bold}] {percent:>3.blue.bold}".to_owned()
    };

    ProgressStyle::default_bar()
        .template(&template)
        .unwrap()
        .progress_chars("=>-")
        .with_key("bytes", |state: &ProgressState, w: &mut dyn Write| {
            write!(w, "{} /", HumanBytes(state.pos())).unwrap()
        })
        .with_key(
            "binary_bytes_per_sec",
            |state: &ProgressState, w: &mut dyn Write| {
                write!(w, "@ {}/s", BinaryBytes(state.per_sec() as u64)).unwrap()
            },
        )
        .with_key("percent", |state: &ProgressState, w: &mut dyn Write| {
            write!(w, "{:.*}%", 0, state.fraction() * 100f32).unwrap()
        })
}

pub fn spinner_style() -> (ProgressStyle, Duration) {
    let (template, inv) = (SPINNER, 80);

    let style = ProgressStyle::with_template(" {msg:<48} {spinner}")
        .unwrap()
        .tick_strings(template);

    (style, Duration::from_millis(inv))
}
