use std::{
    env::var,
    io::Write,
    process::Child,
    sync::atomic::{AtomicI32, Ordering},
};

use crate::{writer::Writer, OmaConsoleError, Result};

pub static SUBPROCESS: AtomicI32 = AtomicI32::new(-1);

pub enum Pager {
    Plain,
    External((String, Child)),
}

impl Pager {
    pub fn new(no_pager: bool, tips: &str) -> Result<Self> {
        if no_pager {
            return Ok(Pager::Plain);
        }

        // Use plain mode for dumb terminals
        let term = var("TERM").unwrap_or_default();
        if term == "dumb" || term == "dialup" {
            return Ok(Pager::Plain);
        }

        let pager_cmd = var("PAGER").unwrap_or_else(|_| "less".to_owned());
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

    pub fn pager_name(&self) -> Option<&str> {
        match self {
            Pager::Plain => None,
            Pager::External((name, _)) => Some(name.as_str()),
        }
    }

    pub fn get_writer(&self) -> Result<Box<dyn Write + '_>> {
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

    pub fn wait_for_exit(&mut self) -> Result<bool> {
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
