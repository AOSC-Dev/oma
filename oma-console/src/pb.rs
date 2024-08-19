use std::{fmt::Write, time::Duration};

use indicatif::{BinaryBytes, HumanBytes, ProgressState, ProgressStyle};

use crate::writer::Writer;

pub struct OmaProgressStyle<'a> {
    style: ProgressStyle,
    writer: &'a Writer,
}

impl<'a> OmaProgressStyle<'a> {
    pub fn new(writer: &'a Writer) -> Self {
        Self {
            style: ProgressStyle::default_bar(),
            writer,
        }
    }

    pub fn progress_bar(self) -> ProgressStyle {
        let max_len = self.writer.get_length();
        let template = if max_len < 100 {
            " {msg} {percent:>3}".to_owned()
        } else {
            " {msg:<59} {total_bytes:<11} [{wide_bar:.white/black}] {percent:>3}".to_owned()
        };

        self.style
            .template(&template)
            .unwrap()
            .progress_chars("=>-")
            .with_key("percent", |state: &ProgressState, w: &mut dyn Write| {
                write!(w, "{:.*}%", 0, state.fraction() * 100f32).unwrap()
            })
    }

    pub fn global_progress_bar(self) -> ProgressStyle {
        let max_len = self.writer.get_length();
        let template = if max_len < 100 {
            " {prefix:.blue.bold} {bytes:>14.green.bold} {total_bytes:.green.bold} {binary_bytes_per_sec:<10.green.bold}".to_owned()
        } else {
            " {prefix:.blue.bold} {bytes:>14.green.bold} {total_bytes:>8.green.bold} {binary_bytes_per_sec:<25.green.bold} {eta_precise:<10.blue.bold} [{wide_bar:.blue.bold}] {percent:>3.blue.bold}".to_owned()
        };

        self.style
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

    pub fn spinner(self) -> (ProgressStyle, Duration) {
        let (template, inv) = (
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
        );

        let style = ProgressStyle::with_template(" {msg:<48} {spinner}")
            .unwrap()
            .tick_strings(template);

        (style, Duration::from_millis(inv))
    }
}
