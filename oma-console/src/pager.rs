use std::{
    env::var,
    io::Write,
    process::Child,
    sync::atomic::{AtomicI32, Ordering}, fmt::Display, ffi::OsStr,
};

use crate::{writer::Writer, OmaConsoleError, OmaConsoleResult};

pub static SUBPROCESS: AtomicI32 = AtomicI32::new(-1);

pub enum Pager {
    Plain,
    External((String, Child)),
}

impl Pager {
    pub fn plain() -> Self {
        Self::Plain
    }

    pub fn external<D: Display + AsRef<OsStr>>(tips: D) -> OmaConsoleResult<Self> {
        let pager_cmd = var("PAGER").unwrap_or_else(|_| "less".to_owned());

        let term = var("TERM").unwrap_or_default();
        if term == "dumb" || term == "dialup" {
            return Ok(Pager::Plain);
        }

        let pager_cmd_segments: Vec<&str> = pager_cmd.split_ascii_whitespace().collect();
        let pager_name = pager_cmd_segments.first().unwrap_or(&"less");
        let mut p = std::process::Command::new(pager_name);

        if pager_name == &"less" {
            p.arg("-R"); // Show ANSI escape sequences correctly
            p.arg("-c"); // Start from the top of the screen
            p.arg("-S"); // 打开横向滚动
            p.arg("-~"); // 让 less 不显示空行的波浪线
            p.arg("-P"); // 打开滚动提示
            p.arg(tips);
            p.env("LESSCHARSET", "UTF-8"); // Rust uses UTF-8
        } else if pager_cmd_segments.len() > 1 {
            p.args(&pager_cmd_segments[1..]);
        }
        let pager_process = p.stdin(std::process::Stdio::piped()).spawn()?;

        // Record PID
        SUBPROCESS.store(pager_process.id() as i32, Ordering::SeqCst);

        let res = Pager::External((pager_name.to_string(), pager_process));

        Ok(res)
    }

    /// Get pager name (like less)
    pub fn pager_name(&self) -> Option<&str> {
        match self {
            Pager::Plain => None,
            Pager::External((name, _)) => Some(name.as_str()),
        }
    }

    /// Get writer to writer something to pager
    pub fn get_writer(&self) -> OmaConsoleResult<Box<dyn Write + '_>> {
        let res = match self {
            Pager::Plain => Writer::default().get_writer(),
            Pager::External((_, child)) => {
                let stdin = child
                    .stdin
                    .as_ref()
                    .ok_or_else(|| OmaConsoleError::StdinDoesNotExist)?;
                let res: Box<dyn Write> = Box::new(stdin);
                res
            }
        };

        Ok(res)
    }

    /// Wait pager to exit
    pub fn wait_for_exit(&mut self) -> OmaConsoleResult<bool> {
        let success = if let Pager::External((_, child)) = self {
            child.wait()?.success()
        } else {
            true
        };

        Ok(success)
    }
}

impl Drop for Pager {
    fn drop(&mut self) {
        // Un-set subprocess pid
        SUBPROCESS.store(-1, Ordering::SeqCst);
    }
}
