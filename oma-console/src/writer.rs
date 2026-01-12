use std::io::{self, Write};

use crate::terminal::Terminal;

pub trait Writeln {
    fn writeln(&self, prefix: &str, msg: &str) -> io::Result<()>;
}

impl Writeln for Writer {
    fn writeln(&self, prefix: &str, msg: &str) -> io::Result<()> {
        for (prefix, body) in self.term.wrap_content(prefix, msg).into_iter() {
            self.write_prefix(prefix)?;
            self.term.write_str(&body)?;
        }

        Ok(())
    }
}

impl Default for Writer {
    fn default() -> Self {
        Self {
            term: Terminal::default(),
        }
    }
}

pub struct Writer {
    term: Terminal,
}

impl Writer {
    pub fn new(prefix_len: u16) -> Self {
        let mut term = Terminal::new();

        term.with_prefix_len(prefix_len);

        Self { term }
    }

    pub fn new_no_limit_length(prefix_len: u16) -> Self {
        let mut writer = Self::new(prefix_len);

        writer.term.with_max_len(None);

        writer
    }

    pub fn new_stdout() -> Self {
        Self {
            term: Terminal::new_stdout(),
        }
    }

    /// See environment is terminal
    pub fn is_terminal(&self) -> bool {
        self.term.is_terminal()
    }

    /// Show terminal cursor
    pub fn show_cursor(&self) -> io::Result<()> {
        self.term.show_cursor()?;
        Ok(())
    }

    /// Get terminal max len to writer message to terminal
    pub fn get_max_len(&self) -> u16 {
        self.term.get_max_len()
    }

    /// Get terminal height
    pub fn get_height(&self) -> u16 {
        self.term.get_height()
    }

    /// Get terminal width
    pub fn get_length(&self) -> u16 {
        self.term.get_length()
    }

    /// Get writer to write something to terminal
    pub fn get_writer(&self) -> Box<dyn Write> {
        self.term.get_writer()
    }

    /// Write oma-style message prefix to terminal
    pub fn write_prefix(&self, prefix: &str) -> io::Result<()> {
        self.term.write_str(&self.term.gen_prefix(prefix))?;

        Ok(())
    }

    pub fn get_prefix_len(&self) -> u16 {
        self.term.get_prefix_len()
    }

    pub fn get_terminal(&self) -> &Terminal {
        &self.term
    }
}
