use std::time::Duration;

use console::style;
use indicatif::{style::TemplateError, ProgressStyle};

use crate::writer::Writer;

pub fn oma_style_pb(writer: Writer, is_global: bool) -> Result<ProgressStyle, TemplateError> {
    let bar_template = {
        let max_len = writer.get_max_len();
        if is_global {
            if max_len < 90 {
                " {msg:.blue.bold}".to_owned()
                    + " {bytes:>10.green.bold} "
                    + &style("/").green().bold().to_string()
                    + " {total_bytes:.green.bold} "
                    + &style("@").green().bold().to_string()
                    + " {binary_bytes_per_sec:<13.green.bold}"
            } else {
                " {msg:.blue.bold}".to_owned()
                    + " {bytes:>10.green.bold} "
                    + &style("/").green().bold().to_string()
                    + " {total_bytes:.green.bold} "
                    + &style("@").green().bold().to_string()
                    + " {binary_bytes_per_sec:<13.green.bold}"
                    + "{eta_precise:>12.blue.bold}   [{wide_bar:.blue.bold}] {percent:>3.blue.bold}"
                    + &style("%").blue().bold().to_string()
            }
        } else if max_len < 90 {
            " {msg} {percent:>3}%".to_owned()
        } else {
            " {msg:<48} {total_bytes:>10}   [{wide_bar:.white/black}] {percent:>3}%".to_owned()
        }
    };

    let barsty = ProgressStyle::default_bar()
        .template(&bar_template)?
        .progress_chars("=>-");

    Ok(barsty)
}

pub fn oma_spinner(ailurus: bool) -> Result<(ProgressStyle, Duration), TemplateError> {
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

    let style = ProgressStyle::with_template(" {msg:<48} {spinner}")?.tick_strings(is_egg);

    Ok((style, Duration::from_millis(inv)))
}
