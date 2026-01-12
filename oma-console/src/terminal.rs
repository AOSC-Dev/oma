use std::io::{self, Write};

use console::Term;

/// Gen oma style message prefix
pub fn gen_prefix(prefix: &str, prefix_len: u16) -> String {
    if console::measure_text_width(prefix) > (prefix_len - 1).into() {
        panic!("Line prefix \"{prefix}\" too long!");
    }

    // Make sure the real_prefix has desired PREFIX_LEN in console
    let left_padding_size = (prefix_len as usize) - 1 - console::measure_text_width(prefix);
    let mut real_prefix: String = " ".repeat(left_padding_size);
    real_prefix.push_str(prefix);
    real_prefix.push(' ');
    real_prefix
}

pub fn wrap_content<'a>(
    prefix: &'a str,
    msg: &str,
    max_len: u16,
    prefix_len: u16,
) -> Vec<(&'a str, String)> {
    let len = (max_len - prefix_len) as usize;

    textwrap::wrap(msg, len)
        .into_iter()
        .enumerate()
        .map(|(i, s)| (if i == 0 { prefix } else { "" }, format!("{s}\n")))
        .collect()
}

/// Providing information about terminal
#[derive(Clone)]
pub struct Terminal {
    term: Term,
    pub(crate) limit_max_len: Option<u16>,
    pub(crate) prefix_len: u16,
}

impl Terminal {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn new_stdout() -> Self {
        Self {
            term: Term::stdout(),
            ..Default::default()
        }
    }

    pub fn with_max_len(&mut self, max_len: Option<u16>) {
        self.limit_max_len = max_len;
    }

    pub fn with_term(&mut self, term: Term) {
        self.term = term;
    }

    pub fn with_prefix_len(&mut self, prefix_len: u16) {
        self.prefix_len = prefix_len;
    }

    pub fn is_terminal(&self) -> bool {
        self.term.is_term()
    }

    /// Show terminal cursor
    pub fn show_cursor(&self) -> io::Result<()> {
        self.term.show_cursor()?;
        Ok(())
    }

    /// Get terminal max len to writer message to terminal
    pub fn get_max_len(&self) -> u16 {
        let len = self.get_length();

        if let Some(limit) = self.limit_max_len {
            if len < limit { len } else { limit }
        } else {
            len
        }
    }

    pub fn get_prefix_len(&self) -> u16 {
        self.prefix_len
    }

    pub fn gen_prefix(&self, prefix: &str) -> String {
        gen_prefix(prefix, self.prefix_len)
    }

    /// Get terminal height
    pub fn get_height(&self) -> u16 {
        self.term.size_checked().unwrap_or((25, 80)).0
    }

    /// Get terminal width
    pub fn get_length(&self) -> u16 {
        self.term.size_checked().unwrap_or((25, 80)).1
    }

    /// Get writer to write something to terminal
    pub fn get_writer(&self) -> Box<dyn Write> {
        Box::new(self.term.clone())
    }

    pub fn wrap_content<'a>(&self, prefix: &'a str, msg: &str) -> Vec<(&'a str, String)> {
        wrap_content(prefix, msg, self.get_max_len(), self.prefix_len)
    }

    pub(crate) fn write_str(&self, str: &str) -> io::Result<()> {
        self.term.write_str(str)
    }
}

impl Default for Terminal {
    fn default() -> Self {
        Self {
            term: Term::stderr(),
            limit_max_len: Some(80),
            prefix_len: 10,
        }
    }
}
