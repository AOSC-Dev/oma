use std::io::{self, IsTerminal, Write};
use std::sync::{Arc, Mutex};

use ratatui::crossterm::cursor::Show;
use ratatui::crossterm::execute;

/// Gen oma style message prefix
pub fn gen_prefix(prefix: &str, prefix_len: u16) -> String {
    if crate::console::measure_text_width(prefix) > (prefix_len - 1).into() {
        panic!("Line prefix \"{prefix}\" too long!");
    }

    // Make sure the real_prefix has desired PREFIX_LEN in console
    let left_padding_size =
        (prefix_len as usize) - 1 - crate::console::measure_text_width(prefix);
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

/// Providing information about terminal, with optional write buffering.
///
/// Use `write_str` for immediate output, or `write_buf` / `flush` for buffered writing.
#[derive(Clone)]
pub struct Terminal {
    is_stderr: bool,
    pub(crate) limit_max_len: Option<u16>,
    pub(crate) prefix_len: u16,
    buf: Arc<Mutex<String>>,
}

impl Terminal {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn new_stdout() -> Self {
        Self {
            is_stderr: false,
            ..Default::default()
        }
    }

    pub fn with_max_len(&mut self, max_len: Option<u16>) {
        self.limit_max_len = max_len;
    }

    pub fn with_prefix_len(&mut self, prefix_len: u16) {
        self.prefix_len = prefix_len;
    }

    /// Check if the terminal is a real terminal (not piped)
    pub fn is_terminal(&self) -> bool {
        if self.is_stderr {
            std::io::stderr().is_terminal()
        } else {
            std::io::stdout().is_terminal()
        }
    }

    /// Show terminal cursor using `ratatui::crossterm`
    pub fn show_cursor(&self) -> io::Result<()> {
        if self.is_stderr {
            execute!(std::io::stderr(), Show)
        } else {
            execute!(std::io::stdout(), Show)
        }
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

    /// Get terminal height (rows)
    pub fn get_height(&self) -> u16 {
        ratatui::crossterm::terminal::size()
            .map(|s| s.1)
            .unwrap_or(25)
    }

    /// Get terminal width (columns)
    pub fn get_length(&self) -> u16 {
        ratatui::crossterm::terminal::size()
            .map(|s| s.0)
            .unwrap_or(80)
    }

    /// Get writer to write something to terminal
    pub fn get_writer(&self) -> Box<dyn Write> {
        Box::new(TerminalWriter(self.is_stderr))
    }

    pub fn wrap_content<'a>(&self, prefix: &'a str, msg: &str) -> Vec<(&'a str, String)> {
        wrap_content(prefix, msg, self.get_max_len(), self.prefix_len)
    }

    /// Write a string directly to the terminal (immediate output, no buffering).
    pub(crate) fn write_str(&self, s: &str) -> io::Result<()> {
        if self.is_stderr {
            let mut stderr = io::stderr();
            stderr.write_all(s.as_bytes())
        } else {
            let mut stdout = io::stdout();
            stdout.write_all(s.as_bytes())
        }
    }

    /// Write a string into the internal buffer (deferred output).
    /// Call `flush()` to write the accumulated buffer to the actual terminal.
    pub fn write_buf(&self, s: &str) {
        self.buf.lock().unwrap().push_str(s);
    }

    /// Flush the internal buffer to the terminal output (stdout / stderr).
    pub fn flush(&self) -> io::Result<()> {
        let mut buf = self.buf.lock().unwrap();
        if buf.is_empty() {
            return Ok(());
        }
        if self.is_stderr {
            io::stderr().write_all(buf.as_bytes())?;
            io::stderr().flush()?;
        } else {
            io::stdout().write_all(buf.as_bytes())?;
            io::stdout().flush()?;
        }
        buf.clear();
        Ok(())
    }
}

impl Default for Terminal {
    fn default() -> Self {
        Self {
            is_stderr: true,
            limit_max_len: Some(80),
            prefix_len: 10,
            buf: Arc::new(Mutex::new(String::new())),
        }
    }
}

/// A helper that implements `Write` for the terminal, wrapping the `is_stderr` flag.
#[derive(Clone, Copy)]
struct TerminalWriter(bool);

impl Write for TerminalWriter {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        if self.0 {
            io::stderr().write(buf)
        } else {
            io::stdout().write(buf)
        }
    }

    fn flush(&mut self) -> io::Result<()> {
        if self.0 {
            io::stderr().flush()
        } else {
            io::stdout().flush()
        }
    }
}
