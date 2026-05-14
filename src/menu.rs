use std::{
    borrow::Cow,
    fmt::Display,
    io::{Write, stderr},
};

use dialoguer::console;
use inquire::{MultiSelect, ui::RenderConfig};

use crate::{WRITER, fl};

#[allow(dead_code)]
#[inline]
pub fn multiselect<T: Display>(
    msg: &str,
    opts: Vec<T>,
    formatter: &dyn Fn(&[inquire::list_option::ListOption<&T>]) -> String,
    render_config: RenderConfig<'_>,
    page_size: u16,
    default: Vec<usize>,
) -> Result<Vec<T>, anyhow::Error> {
    MultiSelect::new(msg, opts)
        .with_help_message(&fl!("tips"))
        .with_formatter(formatter)
        .with_default(&default)
        .with_page_size(page_size as usize)
        .with_render_config(render_config)
        .prompt()
        .map_err(|e| match e {
            inquire::InquireError::OperationInterrupted => {
                stderr().write_all(b"\n").ok();
                anyhow::anyhow!("")
            }
            e => e.into(),
        })
}

pub fn tui_select_list_size() -> u16 {
    match WRITER.get_height() {
        0 => panic!("Terminal height must be greater than 0"),
        1..=6 => 1,
        x @ 7..=25 => x - 6,
        26.. => 20,
    }
}

#[allow(dead_code)]
pub fn select_tui_display_msg(s: &str, is_inquire: bool) -> Cow<'_, str> {
    let term_width = WRITER.get_length() as usize;

    // 4 是 inquire 前面有四个空格缩进
    // 2 是 dialoguer 的保留字符长度
    let indent = if is_inquire { 4 } else { 2 };

    // 3 是 ... 的长度
    if console::measure_text_width(s) + indent > term_width {
        console::truncate_str(s, term_width - indent - 3, "...")
    } else {
        s.into()
    }
}
