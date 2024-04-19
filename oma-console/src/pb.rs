use std::{fmt::Write, time::Duration};

use indicatif::{BinaryBytes, HumanBytes, ProgressState, ProgressStyle};

use crate::writer::Writer;

/// oma style progress bar
pub fn oma_style_pb(writer: Writer, is_global: bool) -> ProgressStyle {
    let bar_template = {
        let max_len = writer.get_length();
        if is_global {
            if max_len < 100 {
                " {prefix:.blue.bold} {bytes:>12.green.bold} {total_bytes:.green.bold} {binary_bytes_per_sec:<10.green.bold}".to_owned()
            } else {
                " {prefix:.blue.bold} {bytes:>12.green.bold} {total_bytes:>8.green.bold} {binary_bytes_per_sec:<27.green.bold} {eta_precise:<10.blue.bold} [{wide_bar:.blue.bold}] {percent:>3.blue.bold}".to_owned()
            }
        } else if max_len < 100 {
            " {msg} {percent:>3}".to_owned()
        } else {
            " {msg:<59} {total_bytes:<11} [{wide_bar:.white/black}] {percent:>3}".to_owned()
        }
    };

    ProgressStyle::default_bar()
        .template(&bar_template)
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

/// oma style spinner
pub fn oma_spinner(ailurus: bool) -> (ProgressStyle, Duration) {
    let (is_egg, inv) = if ailurus {
        (
            &[
                "☀️ ", "☀️ ", "☀️ ", "🌤 ", "⛅️ ", "🌥 ", "☁️ ", "🌧 ", "🌨 ", "🌧 ", "🌨 ", "🌧 ", "🌨 ",
                "⛈ ", "🌨 ", "🌧 ", "🌨 ", "☁️ ", "🌥 ", "⛅️ ", "🌤 ", "☀️ ", "☀️ ",
            ][..],
            100,
        )
    } else {
        (
            &[
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
            ][..],
            80,
        )
    };

    let style = ProgressStyle::with_template(" {msg:<48} {spinner}")
        .unwrap()
        .tick_strings(is_egg);

    (style, Duration::from_millis(inv))
}
