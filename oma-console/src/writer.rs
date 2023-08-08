use std::io::Write;

use crate::OmaConsoleResult;
use console::Term;
use indicatif::ProgressBar;

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

impl Default for Writer {
    fn default() -> Self {
        Writer {
            term: Term::stderr(),
            prefix_len: 10,
        }
    }
}

pub struct Writer {
    term: Term,
    prefix_len: u16,
}

impl Writer {
    pub fn new(prefix_len: u16) -> Self {
        Self {
            prefix_len,
            ..Default::default()
        }
    }

    pub fn show_cursor(&self) -> OmaConsoleResult<()> {
        self.term.show_cursor()?;
        Ok(())
    }

    pub fn get_max_len(&self) -> u16 {
        let len = self.term.size_checked().unwrap_or((25, 80)).1 - self.prefix_len;

        if len > 150 {
            150
        } else {
            len
        }
    }

    pub fn get_height(&self) -> u16 {
        self.term.size_checked().unwrap_or((25, 80)).0
    }

    pub fn get_writer(&self) -> Box<dyn Write> {
        Box::new(self.term.clone())
    }

    fn write_prefix(&self, prefix: &str) -> OmaConsoleResult<()> {
        self.term.write_str(&gen_prefix(prefix, self.prefix_len))?;

        Ok(())
    }

    pub fn writeln(
        &self,
        prefix: &str,
        msg: &str,
        is_pb: bool,
    ) -> OmaConsoleResult<(Vec<String>, Vec<String>)> {
        let max_len = self.get_max_len();
        let mut first_run = true;

        let mut ref_s = msg;
        let mut i = 1;

        let mut added_count = 0;

        let (mut prefix_res, mut msg_res) = (vec![], vec![]);

        // Print msg with left padding
        loop {
            let line_msg = if console::measure_text_width(ref_s) <= max_len.into() {
                format!("{}\n", ref_s).into()
            } else {
                console::truncate_str(ref_s, max_len.into(), "\n")
            };

            if first_run {
                if !is_pb {
                    self.write_prefix(prefix)?;
                } else {
                    prefix_res.push(gen_prefix(prefix, self.prefix_len));
                }
                first_run = false;
            } else if !is_pb {
                self.write_prefix("")?;
            } else {
                prefix_res.push(gen_prefix("", self.prefix_len));
            }

            if !is_pb {
                self.term.write_str(&line_msg)?;
            } else {
                msg_res.push(line_msg.to_string());
            }

            // added_count 是已经处理过字符串的长度
            added_count += line_msg.len();

            // i 代表了有多少个换行符
            // 因此，当预处理的消息长度等于已经处理的消息长度，减去加入的换行符
            // 则处理结束
            if msg.len() == added_count - i {
                break;
            }

            // 把本次已经处理的字符串切片剔除
            ref_s = &ref_s[line_msg.len() - 1..];
            i += 1;
        }

        Ok((prefix_res, msg_res))
    }

    pub fn writeln_with_pb(
        &self,
        pb: &ProgressBar,
        prefix: &str,
        msg: &str,
    ) -> OmaConsoleResult<()> {
        let (prefix, line_msgs) = self.writeln(prefix, msg, true)?;

        for (i, c) in prefix.iter().enumerate() {
            pb.println(format!("{c}{}", line_msgs[i]));
        }

        Ok(())
    }

    pub fn write_chunks<S: AsRef<str>>(
        &self,
        prefix: &str,
        chunks: &[S],
        prefix_len: u16,
    ) -> OmaConsoleResult<()> {
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
