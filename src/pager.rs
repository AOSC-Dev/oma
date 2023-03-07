use anyhow::{format_err, Result};
use std::{env::var, io::Write, process::Child, sync::atomic::Ordering};

pub enum Pager {
    Plain,
    External((String, Child)),
}

impl Pager {
    pub fn new(no_pager: bool) -> Result<Self> {
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

        // 检查用户是否在跑桌面环境，如果有则提示用户可以使用鼠标滚动
        let has_x11 = std::env::var("DISPLAY");

        let tips = if has_x11.is_ok() {
            "Press [q] to end review, [Ctrl-c] to abort, [PgUp/Dn], arrow keys, or mouse wheel to scroll."
        } else {
            "Press [q] to end review, [Ctrl-c] to abort, [PgUp/Dn] or arrow keys to scroll."
        };

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
        crate::SUBPROCESS.store(pager_process.id() as i32, Ordering::SeqCst);

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
            Pager::Plain => crate::WRITER.get_writer(),
            Pager::External((_, child)) => {
                let stdin = child
                    .stdin
                    .as_ref()
                    .ok_or_else(|| format_err!("Failed to take pager's stdin"))?;
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
        crate::SUBPROCESS.store(-1, Ordering::SeqCst);
    }
}
