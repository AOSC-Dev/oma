use ratatui::crossterm::{ExecutableCommand, cursor::Show, terminal};
use std::io::{self, IsTerminal, Write, stderr, stdout};

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
) -> impl Iterator<Item = (&'a str, String)> {
    let len = (max_len - prefix_len) as usize;

    textwrap::wrap(msg, len)
        .into_iter()
        .enumerate()
        .map(move |(i, s)| (if i == 0 { prefix } else { "" }, format!("{s}\n")))
}

#[derive(Clone)]
pub enum TermOutput {
    Stdout,
    Stderr,
}

impl TermOutput {
    fn show_cursor(&self) -> io::Result<()> {
        match self {
            TermOutput::Stdout => {
                stdout().execute(Show)?;
            }
            TermOutput::Stderr => {
                stderr().execute(Show)?;
            }
        }

        Ok(())
    }

    fn write_str(&self, s: &str) -> io::Result<()> {
        match self {
            TermOutput::Stdout => stdout().write_all(s.as_bytes()),
            TermOutput::Stderr => stderr().write_all(s.as_bytes()),
        }
    }

    fn is_terminal(&self) -> bool {
        match self {
            TermOutput::Stdout => stdout().is_terminal(),
            TermOutput::Stderr => stderr().is_terminal(),
        }
    }

    fn get_lock_writer(&self) -> Box<dyn Write> {
        match self {
            TermOutput::Stdout => Box::new(stdout().lock()),
            TermOutput::Stderr => Box::new(stderr().lock()),
        }
    }
}

impl Write for TermOutput {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        match self {
            TermOutput::Stdout => stdout().write(buf),
            TermOutput::Stderr => stderr().write(buf),
        }
    }

    fn flush(&mut self) -> io::Result<()> {
        match self {
            TermOutput::Stdout => stdout().flush(),
            TermOutput::Stderr => stderr().flush(),
        }
    }
}

/// Providing information about terminal
#[derive(Clone)]
pub struct Terminal {
    term: TermOutput,
    pub(crate) limit_max_len: Option<u16>,
    pub(crate) prefix_len: u16,
}

impl Terminal {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn new_stdout() -> Self {
        Self {
            term: TermOutput::Stdout,
            ..Default::default()
        }
    }

    pub fn with_max_len(&mut self, max_len: Option<u16>) {
        self.limit_max_len = max_len;
    }

    pub fn with_prefix_len(&mut self, prefix_len: u16) {
        self.prefix_len = prefix_len;
    }

    pub fn is_terminal(&self) -> bool {
        self.term.is_terminal()
    }

    /// Show terminal cursor
    pub fn show_cursor(&self) -> io::Result<()> {
        self.term.show_cursor()
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
        terminal::size().map(|s| s.1).unwrap_or(25)
    }

    /// Get terminal width
    pub fn get_length(&self) -> u16 {
        terminal::size().map(|s| s.0).unwrap_or(80)
    }

    /// Get writer to write something to terminal
    pub fn get_writer(&self) -> Box<dyn Write> {
        self.term.get_lock_writer()
    }

    pub fn wrap_content<'a>(
        &self,
        prefix: &'a str,
        msg: &str,
    ) -> impl Iterator<Item = (&'a str, String)> {
        wrap_content(prefix, msg, self.get_max_len(), self.prefix_len)
    }

    pub(crate) fn write_str(&self, str: &str) -> io::Result<()> {
        self.term.write_str(str)
    }
}

impl Default for Terminal {
    fn default() -> Self {
        Self {
            term: TermOutput::Stderr,
            limit_max_len: Some(80),
            prefix_len: 10,
        }
    }
}
