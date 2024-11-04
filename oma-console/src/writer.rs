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

pub trait Writeln {
    fn writeln(&self, prefix: &str, msg: &str) -> io::Result<()>;
}

impl Writeln for Writer {
    fn writeln(&self, prefix: &str, msg: &str) -> io::Result<()> {
        let max_len = self.get_max_len();

        let mut res = Ok(());

        writeln_inner(msg, prefix, max_len as usize, self.prefix_len, |t, s| {
            match t {
                MessageType::Msg => res = self.term.write_str(s),
                MessageType::Prefix => res = self.write_prefix(s),
            };
        });

        res
    }
}

impl Default for Writer {
    fn default() -> Self {
        Writer {
            term: Term::stderr(),
            prefix_len: 10,
            limit_max_len: Some(80),
        }
    }
}

pub struct Writer {
    term: Term,
    pub prefix_len: u16,
    limit_max_len: Option<u16>,
}

impl Writer {
    pub fn new(prefix_len: u16) -> Self {
        Self {
            prefix_len,
            ..Default::default()
        }
    }

    pub fn new_no_limit_length(prefix_len: u16) -> Self {
        Self {
            prefix_len,
            limit_max_len: None,
            ..Default::default()
        }
    }

    pub fn new_stdout() -> Self {
        Self {
            term: Term::stdout(),
            ..Default::default()
        }
    }

    /// See environment is terminal
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
            if len < limit {
                len
            } else {
                limit
            }
        } else {
            len
        }
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

    /// Write oma-style message prefix to terminal
    pub fn write_prefix(&self, prefix: &str) -> io::Result<()> {
        self.term.write_str(&gen_prefix(prefix, self.prefix_len))?;

        Ok(())
    }

    pub fn get_prefix_len(&self) -> u16 {
        self.prefix_len
    }

    pub fn write_chunks<S: AsRef<str>>(
        &self,
        prefix: &str,
        chunks: &[S],
        prefix_len: u16,
    ) -> io::Result<()> {
        if chunks.is_empty() {
            return Ok(());
        }

        let max_len: usize = (self.get_max_len() - prefix_len).into();
        // Write prefix first
        self.write_prefix(prefix)?;
        let mut cur_line_len: usize = prefix_len.into();
        for chunk in chunks {
            let chunk = chunk.as_ref();
            let chunk_len = console::measure_text_width(chunk);
            // If going to overflow the line, create new line
            // The `1` is the preceding space
            if cur_line_len + chunk_len + 1 > max_len {
                self.term.write_str("\n")?;
                self.write_prefix("")?;
                cur_line_len = 0;
            }
            self.term.write_str(chunk)?;
            self.term.write_str(" ")?;
            cur_line_len += chunk_len + 1;
        }
        // Write a new line
        self.term.write_str("\n")?;

        Ok(())
    }
}

pub enum MessageType {
    Msg,
    Prefix,
}

pub fn writeln_inner(
    msg: &str,
    prefix: &str,
    max_len: usize,
    prefix_len: u16,
    mut callback: impl FnMut(MessageType, &str),
) {
    let len = max_len - prefix_len as usize;
    let mut first_run = true;

    for i in textwrap::wrap(msg, len) {
        if first_run {
            callback(MessageType::Prefix, prefix);
            first_run = false;
        } else {
            callback(MessageType::Prefix, "");
        }

        callback(MessageType::Msg, &format!("{i}\n"));
    }
}
