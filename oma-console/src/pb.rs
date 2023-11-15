use std::time::Duration;

use console::style;
use indicatif::ProgressStyle;

use crate::writer::Writer;

/// oma style progress bar
pub fn oma_style_pb(writer: Writer, is_global: bool) -> ProgressStyle {
    let bar_template = {
        let max_len = writer.get_length();
        if is_global {
            if max_len < 100 {
                " {prefix:.blue.bold}".to_owned()
                    + " {bytes:>10.green.bold} "
                    + &style("/").green().bold().to_string()
                    + " {total_bytes:.green.bold} "
                    + &style("@").green().bold().to_string()
                    + " {binary_bytes_per_sec:<13.green.bold}"
            } else {
                " {prefix:.blue.bold}".to_owned()
                    + " {bytes:>10.green.bold} "
                    + &style("/").green().bold().to_string()
                    + " {total_bytes:.green.bold} "
                    + &style("@").green().bold().to_string()
                    + " {binary_bytes_per_sec:<28.green.bold}"
                    + "{eta_precise:<10.blue.bold}  [{wide_bar:.blue.bold}] {percent:>3.blue.bold}"
                    + &style("%").blue().bold().to_string()
            }
        } else if max_len < 100 {
            " {msg} {percent:>3}%".to_owned()
        } else {
            " {msg:<60} {total_bytes:<10}  [{wide_bar:.white/black}] {percent:>3}%".to_owned()
        }
    };

    ProgressStyle::default_bar()
        .template(&bar_template)
        .unwrap()
        .progress_chars("=>-")
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
