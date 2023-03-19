use anyhow::{Context, Result};
use console::Term;
use std::io::Write;

const PREFIX_LEN: u16 = 10;

pub fn gen_prefix(prefix: &str) -> String {
    if console::measure_text_width(prefix) > (PREFIX_LEN - 1).into() {
        panic!("Line prefix \"{prefix}\" too long!");
    }

    // Make sure the real_prefix has desired PREFIX_LEN in console
    let left_padding_size = (PREFIX_LEN as usize) - 1 - console::measure_text_width(prefix);
    let mut real_prefix: String = " ".repeat(left_padding_size);
    real_prefix.push_str(prefix);
    real_prefix.push(' ');
    real_prefix
}

impl Default for Writer {
    fn default() -> Self {
        Writer {
            term: Term::stderr(),
        }
    }
}

pub struct Writer {
    term: Term,
}

impl Writer {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn show_cursor(&self) -> Result<()> {
        self.term.show_cursor()?;
        Ok(())
    }

    pub fn get_max_len(&self) -> u16 {
        self.term.size_checked().unwrap_or((25, 80)).1 - PREFIX_LEN
    }

    pub fn get_height(&self) -> u16 {
        self.term.size_checked().unwrap_or((25, 80)).0
    }

    pub fn get_writer(&self) -> Box<dyn Write> {
        Box::new(self.term.clone())
    }

    fn write_prefix(&self, prefix: &str) -> Result<()> {
        self.term
            .write_str(&gen_prefix(prefix))
            .context("Failed to write prefix to console.")?;
        Ok(())
    }

    pub fn writeln(&self, prefix: &str, msg: &str) -> Result<()> {
        let max_len = self.get_max_len();
        let mut first_run = true;

        let mut msg = msg.to_string();
        // Print msg with left padding
        while !msg.is_empty() {
            let line_msg = console::truncate_str(&msg, max_len.into(), "\n");
            if first_run {
                self.write_prefix(prefix)
                    .context("Failed to write prefix to console.")?;
                first_run = false;
            } else {
                self.write_prefix("")
                    .context("Failed to write prefix to console.")?;
            }
            self.term
                .write_str(&line_msg)
                .context("Failed to write message to console.")?;
            // Remove the already written part, strip ANSI since it can mess everything up
            let mut new_msg = console::strip_ansi_codes(&msg).to_string();
            let line_msg_len = console::measure_text_width(&line_msg);
            new_msg.replace_range(..line_msg_len, "");
            // Swap
            std::mem::swap(&mut msg, &mut new_msg);
        }
        self.term.write_line("")?;

        Ok(())
    }

    // pub fn write_chunks<S: AsRef<str>>(&self, prefix: &str, chunks: &[S]) -> Result<()> {
    //     if chunks.is_empty() {
    //         return Ok(());
    //     }

    //     let max_len: usize = (self.get_max_len() - PREFIX_LEN).into();
    //     // Write prefix first
    //     self.write_prefix(prefix)?;
    //     let mut cur_line_len: usize = PREFIX_LEN.into();
    //     for chunk in chunks {
    //         let chunk = chunk.as_ref();
    //         let chunk_len = console::measure_text_width(chunk);
    //         // If going to overflow the line, create new line
    //         // The `1` is the preceding space
    //         if cur_line_len + chunk_len + 1 > max_len {
    //             self.term.write_str("\n")?;
    //             self.write_prefix("")?;
    //             cur_line_len = 0;
    //         }
    //         self.term.write_str(chunk)?;
    //         self.term.write_str(" ")?;
    //         cur_line_len += chunk_len + 1;
    //     }
    //     // Write a new line
    //     self.term.write_str("\n")?;

    //     Ok(())
    // }
}

// We will ignore write errors in the following macros, since cannot print messages is not an emergency
#[macro_export]
macro_rules! msg {
    ($($arg:tt)+) => {
        $crate::WRITER.writeln("", &format!($($arg)+)).ok();
    };
}

#[macro_export]
macro_rules! debug {
    ($($arg:tt)+) => {
        tracing::debug!("{}", &format!($($arg)+));
        $crate::WRITER.writeln(&console::style("DEBUG").dim().to_string(), &format!($($arg)+)).ok();
    };
}

#[macro_export]
macro_rules! success {
    ($($arg:tt)+) => {
        tracing::info!("{}", &format!($($arg)+));
        $crate::WRITER.writeln(&console::style("SUCCESS").green().bold().to_string(), &format!($($arg)+)).ok();
    };
}

#[macro_export]
macro_rules! info {
    ($($arg:tt)+) => {
        tracing::info!("{}", &format!($($arg)+));
        $crate::WRITER.writeln(&console::style("INFO").blue().bold().to_string(), &format!($($arg)+)).ok();
    };
}

#[macro_export]
macro_rules! warn {
    ($($arg:tt)+) => {
        tracing::warn!("{}", &format!($($arg)+));
        $crate::WRITER.writeln(&console::style("WARNING").yellow().bold().to_string(), &format!($($arg)+)).ok();
    };
}

#[macro_export]
macro_rules! error {
    ($($arg:tt)+) => {
        tracing::error!("{}", &format!($($arg)+));
        $crate::WRITER.writeln(&console::style("ERROR").red().bold().to_string(), &format!($($arg)+)).ok();
    };
}

#[macro_export]
macro_rules! due_to {
    ($($arg:tt)+) => {
        tracing::info!("{}", &format!($($arg)+));
        $crate::WRITER.writeln(&console::style("DUE TO").yellow().bold().to_string(), &format!($($arg)+)).ok();
    };
}
