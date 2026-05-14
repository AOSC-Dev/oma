use dialoguer::console::style;
use oma_pm::oma_apt::Package;
use oma_utils::human_bytes::HumanBytes;
use ratatui::{
    style::Style,
    text::{Line, Span},
};

use crate::subcommand::size_analyzer::tui::BgRenderMode;

const FULL: &str = "█";
const SEVEN_EIGHTHS: &str = "▉";
const THREE_QUARTERS: &str = "▊";
const FIVE_EIGHTHS: &str = "▋";
const HALF: &str = "▌";
const THREE_EIGHTHS: &str = "▍";
const ONE_QUARTER: &str = "▎";
const ONE_EIGHTH: &str = "▏";
const BAR_BLOCK_LENGTH: usize = 19;

#[derive(Clone, PartialEq, Eq)]
pub(crate) struct PkgWrapper<'a> {
    pub(crate) pkg: Package<'a>,
}

impl<'a> PkgWrapper<'a> {
    pub(crate) fn to_installed_line(
        &self,
        total_installed_size: u64,
        pending_to_delete: bool,
    ) -> Line<'a> {
        let ver = self.pkg.installed().unwrap();
        let size = ver.installed_size();

        let not_allow_delete = self.is_not_allow_delete();

        let hb_fmt = format_human_size(size);
        let percent_str = format!("{:.1}%", get_percent(size, total_installed_size));

        Line::from_iter(vec![
            Span::styled(hb_fmt, Style::new().green()),
            Span::raw(" "),
            Span::styled(
                format!(
                    "{}{}",
                    " ".repeat(6usize.saturating_sub(percent_str.len())),
                    percent_str
                ),
                Style::new().gray(),
            ),
            Span::raw(" "),
            Span::styled(
                make_bar(size as f64 / total_installed_size as f64, BAR_BLOCK_LENGTH).to_string(),
                Style::new().gray(),
            ),
            Span::styled(
                self.pkg.fullname(true),
                if pending_to_delete {
                    Style::new().yellow()
                } else {
                    Style::new().gray()
                },
            ),
        ])
        .style({
            if not_allow_delete {
                Style::new().on_red()
            } else {
                Style::new()
            }
        })
    }

    pub(crate) fn is_not_allow_delete(&self) -> bool {
        let ver = self.pkg.installed().unwrap();

        ver.get_record("X-AOSC-Features").is_some() || self.pkg.is_essential()
    }

    pub(crate) fn to_table_line(
        &self,
        total_installed_size: u64,
    ) -> (String, String, String, String) {
        let ver = self.pkg.installed().unwrap();
        let size = ver.installed_size();
        let not_allow_delete = self.is_not_allow_delete();

        let human_size = style(HumanBytes(size).to_string()).green().to_string();
        let percent = get_percent(size, total_installed_size);
        let percent_str = format!("{percent:.1}%");
        let bar = make_bar(size as f64 / total_installed_size as f64, BAR_BLOCK_LENGTH);
        let mut name = self.pkg.fullname(true);

        if not_allow_delete {
            name = style(name).red().to_string();
        }

        (human_size, percent_str, bar, name)
    }

    pub(crate) fn to_remove_line(
        &self,
        area_width: u16,
        bg_render_mode: BgRenderMode,
    ) -> Line<'static> {
        let ver = self.pkg.installed().unwrap();
        let size = ver.installed_size();
        let human_size = HumanBytes(size).to_string();
        let pkg_name = self.pkg.fullname(true);
        let name_len = pkg_name.len();
        let human_size_len = human_size.len();

        match bg_render_mode {
            BgRenderMode::Color(_) => Line::from_iter(vec![
                Span::styled(pkg_name, Style::new().yellow()),
                Span::raw(format!(
                    "{:-space$}",
                    "",
                    space = (area_width as usize)
                        .saturating_sub(name_len)
                        .saturating_sub(human_size_len),
                )),
                Span::styled(HumanBytes(size).to_string(), Style::new().red()),
            ]),
            BgRenderMode::Reverse => Line::from_iter(vec![
                Span::raw(pkg_name),
                Span::raw(format!(
                    "{:-space$}",
                    "",
                    space = (area_width as usize)
                        .saturating_sub(name_len)
                        .saturating_sub(human_size_len),
                )),
                Span::raw(HumanBytes(size).to_string()),
            ]),
        }
    }
}

fn format_human_size(size: u64) -> String {
    let hb = HumanBytes(size).to_string();
    let needs_size = 11usize.saturating_sub(hb.len());

    format!("{}{}", " ".repeat(needs_size), hb)
}

#[inline]
fn get_percent(size: u64, total: u64) -> f64 {
    (size as f64 / total as f64) * 100.0
}

// From https://github.com/Byron/dua-cli/blob/main/src/interactive/app/bytevis.rs
fn make_bar(percentage: f64, length: usize) -> String {
    let mut s = String::new();
    // Print the filled part of the bar
    let block_length = (length as f64 * percentage).floor() as usize;
    for _ in 0..block_length {
        s.push_str(FULL);
    }

    // Bar is done if full length is already used, continue working if not
    if block_length < length {
        let block_sections = [
            " ",
            ONE_EIGHTH,
            ONE_QUARTER,
            THREE_EIGHTHS,
            HALF,
            FIVE_EIGHTHS,
            THREE_QUARTERS,
            SEVEN_EIGHTHS,
            FULL,
        ];
        // Get the index based on how filled the remaining part is
        let index = (((length as f64 * percentage) - block_length as f64) * 8f64).round() as usize;
        s.push_str(block_sections[index]);

        // Remainder of the bar should be empty
        for _ in 0..length - block_length - 1 {
            s.push(' ');
        }
    }

    s
}
