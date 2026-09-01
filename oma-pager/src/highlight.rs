use aho_corasick::{AhoCorasick, BuildError};

const HIGHLIGHT_START: &str = "\x1b[7m";
const HIGHLIGHT_END: &str = "\x1b[0m";

/// Highlight a pattern in a line of text using ANSI reverse video.
pub struct Highlight<'a> {
    pattern: &'a str,
    ac: AhoCorasick,
}

impl<'a> Highlight<'a> {
    pub fn new(pattern: &'a str) -> Result<Self, BuildError> {
        Ok(Self {
            ac: AhoCorasick::new([pattern])?,
            pattern,
        })
    }

    pub fn replace(&self, input: &str) -> String {
        self.ac.replace_all(
            input,
            &[format!(
                "{}{}{}",
                HIGHLIGHT_START, self.pattern, HIGHLIGHT_END
            )],
        )
    }
}

/// Remove the highlight markers added by [`Highlight`].
pub struct ClearHighlight(AhoCorasick);

impl ClearHighlight {
    pub fn new() -> Self {
        Self(AhoCorasick::new([HIGHLIGHT_START, HIGHLIGHT_END]).unwrap())
    }

    pub fn replace(&self, input: &str) -> String {
        self.0.replace_all(input, &["", ""])
    }
}

impl Default for ClearHighlight {
    fn default() -> Self {
        Self::new()
    }
}
