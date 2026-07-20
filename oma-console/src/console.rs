//! Compatibility module that replaces the `console` crate
//! Provides the same public API surface used by `oma-console` and the main `oma` crate.
//!
//! Uses `ratatui::crossterm` for terminal styling (`StyledContent`, `Stylize` trait).

use std::borrow::Cow;

use ratatui::crossterm::style::{ContentStyle, StyledContent};
use unicode_width::UnicodeWidthChar;
use unicode_width::UnicodeWidthStr;

/// Re-export crossterm's Color and Stylize trait so callers can chain methods
pub use ratatui::crossterm::style::Color;
pub use ratatui::crossterm::style::Stylize;

/// Styled string type backed by crossterm's `StyledContent<String>`.
///
/// Call `.bold()`, `.red()`, `.on(Color::AnsiValue(n))`, etc. via the `Stylize` trait.
pub type StyledStr = StyledContent<String>;

/// Create a new `StyledStr` from anything displayable (converted to `String` internally).
pub fn style(value: impl std::fmt::Display) -> StyledStr {
    StyledContent::new(ContentStyle::new(), value.to_string())
}

/// Measure the display width of a string, accounting for wide characters (CJK, etc.)
/// and stripping ANSI escape codes before measuring.
pub fn measure_text_width(s: &str) -> usize {
    UnicodeWidthStr::width(strip_ansi_codes(s).as_str())
}

/// Strip ANSI escape codes from a string
pub fn strip_ansi_codes(s: &str) -> String {
    let mut result = String::with_capacity(s.len());
    let mut chars = s.chars();
    while let Some(c) = chars.next() {
        if c == '\x1b' && chars.next() == Some('[') {
            for c in chars.by_ref() {
                if c.is_ascii_alphabetic() || c == '~' {
                    break;
                }
            }
        } else {
            result.push(c);
        }
    }
    result
}

/// Truncate a string to the given display width, appending a suffix if truncated
pub fn truncate_str<'a>(s: &'a str, max_width: usize, suffix: &str) -> Cow<'a, str> {
    if measure_text_width(s) <= max_width {
        return Cow::Borrowed(s);
    }

    let suffix_width = measure_text_width(suffix);
    let available = max_width.saturating_sub(suffix_width);

    let mut width = 0;
    let mut result = String::new();
    for c in s.chars() {
        let cw = UnicodeWidthChar::width(c).unwrap_or(0);
        if width + cw > available {
            break;
        }
        width += cw;
        result.push(c);
    }
    result.push_str(suffix);
    Cow::Owned(result)
}
