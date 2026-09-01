use std::{
    io::{self, BufRead, Write},
    time::{Duration, Instant},
};

use ansi_to_tui::IntoText;
use oma_console::{console, writer::Writer};
use oma_logger::debug;
use ratatui::crossterm::{
    self,
    event::{self, Event},
};
use ratatui::{
    Frame, Terminal,
    backend::Backend,
    layout::{Alignment, Constraint, Layout},
    style::{Color, Stylize},
    widgets::{Block, Borders, Paragraph, Scrollbar, ScrollbarOrientation, ScrollbarState},
};
use tui_input::Input;

use crate::highlight::{ClearHighlight, Highlight};
use crate::key_binding::Control;
use crate::traits::{PagerExit, PagerTheme, PagerUIText};

enum PagerInner {
    Working(Vec<u8>),
    Finished(Vec<String>),
}

/// `OmaPager` is a structure that implements a pager displaying text-based content in a terminal UI.
pub struct OmaPager {
    /// The internal state of the pager, which can be either `Working` or `Finished`.
    inner: PagerInner,
    /// The state of the vertical scrollbar.
    vertical_scroll_state: ScrollbarState,
    /// The state of the horizontal scrollbar.
    horizontal_scroll_state: ScrollbarState,
    /// The current vertical scroll position.
    vertical_scroll: usize,
    /// The current horizontal scroll position.
    horizontal_scroll: usize,
    /// The height of the display area.
    area_height: u16,
    /// The maximum width of the display area.
    max_width: u16,
    /// A string containing tips to be displayed in the pager at the bottom.
    pub(crate) tips: String,
    /// An optional title for the pager.
    title: Option<String>,
    /// The length of the inner content.
    inner_len: usize,
    /// The user-provided theme used to customize the pager's colors.
    theme: Option<Box<dyn PagerTheme>>,
    /// A vector containing the indices of search results.
    pub(crate) search_results: Vec<usize>,
    /// The index of the current search result being displayed.
    pub(crate) current_result_index: usize,
    /// The current mode of the pager, which can be either `Normal`, `Search` and `SearchInputText`.
    pub(crate) mode: TuiMode,
    /// A reference to a trait object that provides UI text for the pager.
    pub(crate) ui_text: Box<dyn PagerUIText>,
    /// A terminal writer to print oma-style message
    writer: Writer,
    /// Use y/n to replace 'q' to confirm/cancel if is question mode
    pub(crate) yn_mode: bool,
    /// For search query editing.
    pub(crate) search_input: Input,
}

impl Write for OmaPager {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        match self.inner {
            PagerInner::Working(ref mut v) => v.extend_from_slice(buf),
            PagerInner::Finished(_) => {
                return Err(io::Error::other("write is finished"));
            }
        }

        Ok(buf.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

#[derive(PartialEq, Eq)]
pub(crate) enum TuiMode {
    Search,
    SearchInputText,
    Normal,
}

impl OmaPager {
    pub fn new(
        title: Option<String>,
        theme: Option<Box<dyn PagerTheme>>,
        ui_text: Box<dyn PagerUIText>,
        yn_mode: bool,
    ) -> Self {
        Self {
            inner: PagerInner::Working(vec![]),
            vertical_scroll_state: ScrollbarState::new(0),
            horizontal_scroll_state: ScrollbarState::new(0),
            vertical_scroll: 0,
            horizontal_scroll: 0,
            area_height: 0,
            max_width: 0,
            tips: ui_text.normal_tips(yn_mode),
            title,
            inner_len: 0,
            theme,
            search_results: Vec::new(),
            current_result_index: 0,
            mode: TuiMode::Normal,
            ui_text,
            writer: Writer::default(),
            yn_mode,
            search_input: Input::new(String::new()),
        }
    }
    /// Run the pager
    ///
    /// This function runs the pager, processes user/program input, and renders the output in a terminal UI.
    /// Note: Please use `wait_for_exit` to run a pager instead of calling this function directly.
    ///
    /// # Arguments
    /// * `terminal` - A mutable reference to a `Terminal` object that handles the terminal UI rendering.
    /// * `tick_rate` - A `Duration` object that specifies the tick rate for the terminal updates.
    ///
    /// # Returns
    ///
    /// Returns an `io::Result` containing a `PagerExit` value that indicates the exit status of the pager.
    pub fn run<B: Backend>(
        mut self,
        terminal: &mut Terminal<B>,
        tick_rate: Duration,
    ) -> io::Result<PagerExit> {
        self.inner = if let PagerInner::Working(v) = self.inner {
            PagerInner::Finished(v.lines().map_while(Result::ok).collect::<Vec<_>>())
        } else {
            return Err(io::Error::other("write is finished"));
        };

        let PagerInner::Finished(ref text) = self.inner else {
            unreachable!()
        };

        let width = text
            .iter()
            .map(|x| console::measure_text_width(x))
            .max()
            .unwrap_or(1);

        self.max_width = width as u16;
        self.inner_len = text.len();

        let mut last_tick = Instant::now();
        // Start the loop, waiting for the keyboard interrupts.
        loop {
            terminal
                .draw(|f| self.ui(f))
                .map_err(|e| io::Error::other(e.to_string()))?;

            let timeout = tick_rate.saturating_sub(last_tick.elapsed());
            if crossterm::event::poll(timeout)? {
                match event::read()? {
                    Event::Key(key) => match self.handle_key_binding(key) {
                        Control::Continue => continue,
                        Control::Exit(exit) => return Ok(exit),
                    },
                    _ => continue,
                }
            }
            if last_tick.elapsed() >= tick_rate {
                last_tick = Instant::now();
            }
        }
    }

    pub(crate) fn page_down(&mut self) {
        let pos = self
            .vertical_scroll
            .saturating_add(self.area_height as usize);
        if pos < self.inner_len {
            self.vertical_scroll = pos;
        } else {
            return;
        }
        self.vertical_scroll_state = self.vertical_scroll_state.position(self.vertical_scroll);
    }

    pub(crate) fn page_up(&mut self) {
        self.vertical_scroll = self
            .vertical_scroll
            .saturating_sub(self.area_height as usize);
        self.vertical_scroll_state = self.vertical_scroll_state.position(self.vertical_scroll);
    }

    pub(crate) fn goto_end(&mut self) {
        self.vertical_scroll = self.inner_len.saturating_sub(self.area_height.into());
        self.vertical_scroll_state = self.vertical_scroll_state.position(self.vertical_scroll);
    }

    pub(crate) fn goto_begin(&mut self) {
        self.vertical_scroll = 0;
        self.vertical_scroll_state = self.vertical_scroll_state.position(0);
    }

    pub(crate) fn right(&mut self) {
        let width = self.writer.get_length();

        if self.max_width <= self.horizontal_scroll as u16 + width {
            return;
        }

        self.horizontal_scroll = self.horizontal_scroll.saturating_add((width / 4).into());
        self.horizontal_scroll_state = self
            .horizontal_scroll_state
            .position(self.horizontal_scroll);
    }

    pub(crate) fn left(&mut self) {
        let width = self.writer.get_length();
        self.horizontal_scroll = self.horizontal_scroll.saturating_sub((width / 4).into());
        self.horizontal_scroll_state = self
            .horizontal_scroll_state
            .position(self.horizontal_scroll);
    }

    pub(crate) fn up(&mut self) {
        self.vertical_scroll = self.vertical_scroll.saturating_sub(1);
        self.vertical_scroll_state = self.vertical_scroll_state.position(self.vertical_scroll);
    }

    pub(crate) fn down(&mut self) {
        if self
            .vertical_scroll
            .saturating_add(self.area_height as usize)
            >= self.inner_len
        {
            return;
        }
        self.vertical_scroll = self.vertical_scroll.saturating_add(1);
        self.vertical_scroll_state = self.vertical_scroll_state.position(self.vertical_scroll);
    }
    /// Search for a pattern in the pager content
    /// # Returns:
    /// The lines contain this pattern (In vec<usize>)
    fn search(&mut self, pattern: &str) -> Vec<usize> {
        let mut result: Vec<usize> = Vec::new();

        if let PagerInner::Finished(ref mut text) = self.inner {
            match Highlight::new(pattern) {
                Ok(highlight) => {
                    for (i, line) in text.iter_mut().enumerate() {
                        if line.contains(pattern) {
                            result.push(i);
                            // highlight the pattern
                            *line = highlight.replace(line);
                        }
                    }
                }
                Err(e) => {
                    debug!("{e}");
                }
            }
        }

        result
    }

    pub(crate) fn exit_search_mode(&mut self) {
        self.mode = TuiMode::Normal;
        self.clear_highlight();
        self.tips = self.ui_text.normal_tips(self.yn_mode);
    }

    pub(crate) fn perform_search(&mut self, pattern: &str) {
        self.search_results = self.search(pattern);
        if self.search_results.is_empty() {
            self.tips = self.ui_text.search_tips_not_found();
        } else {
            self.current_result_index = 0;
            self.jump_to(self.search_results[self.current_result_index]);
            self.tips = self.ui_text.search_tips_with_result();
        }
    }

    /// Jump to line
    pub(crate) fn jump_to(&mut self, line: usize) {
        self.vertical_scroll = line;
        self.vertical_scroll_state = self.vertical_scroll_state.position(self.vertical_scroll);
    }

    pub(crate) fn clear_highlight(&mut self) {
        if let PagerInner::Finished(ref mut text) = self.inner {
            let clear_highlighter = ClearHighlight::new();
            for line_index in &self.search_results {
                if let Some(line) = text.get_mut(*line_index) {
                    *line = clear_highlighter.replace(line);
                }
            }
        }
    }

    /// Render and fresh the UI
    fn ui(&mut self, f: &mut Frame) {
        let area = f.area();
        let mut layout = vec![
            Constraint::Min(0),
            // 2 是 block 的两条线
            Constraint::Length(self.tips.lines().count() as u16 + 2),
        ];

        let mut has_title = false;
        if self.title.is_some() {
            layout.insert(0, Constraint::Length(1));
            has_title = true;
        }

        let chunks = Layout::vertical(layout).split(area);

        let title_bg_color = self
            .theme
            .as_ref()
            .map(|t| t.title_bg_color())
            .unwrap_or(Color::Indexed(25));
        let title_fg_color = self
            .theme
            .as_ref()
            .map(|t| t.title_fg_color())
            .unwrap_or(Color::White);

        if let Some(title) = &self.title {
            let title = Block::new()
                .title_alignment(Alignment::Left)
                .title(title.to_string())
                .fg(title_fg_color)
                .bg(title_bg_color);

            f.render_widget(title, chunks[0]);
        }

        self.area_height = if has_title {
            chunks[1].height
        } else {
            chunks[0].height
        };

        let width = if self.max_width <= self.writer.get_length() {
            0
        } else {
            self.max_width
        };

        self.horizontal_scroll_state = self.horizontal_scroll_state.content_length(width as usize);

        self.vertical_scroll_state = self
            .vertical_scroll_state
            .content_length(self.inner_len.saturating_sub(self.area_height as usize));

        let PagerInner::Finished(ref text) = self.inner else {
            unreachable!()
        };

        let text = if let Some(text) =
            text.get(self.vertical_scroll..self.vertical_scroll + self.area_height as usize)
        {
            // 根据屏幕高度来决定显示多少行
            text
        } else {
            // 达到末尾，即剩余行数小于屏幕高度
            &text[self.vertical_scroll..]
        };

        let text = text.join("\n");
        let text = match text.to_text() {
            Ok(text) => text,
            Err(e) => {
                debug!("{e}");
                return;
            }
        };

        // 不使用 .scroll 控制上下滚动是因为它需要一整个 self.text 来计算滚动
        // 因为 Paragraph 只接受 owner, self.text 每一次都需要 clone 获取主动权
        // 当 self.text 行数一多，性能就会非常的“好”
        f.render_widget(
            Paragraph::new(text).scroll((0, self.horizontal_scroll as u16)),
            if has_title { chunks[1] } else { chunks[0] },
        );

        f.render_stateful_widget(
            Scrollbar::new(ScrollbarOrientation::VerticalRight)
                .begin_symbol(Some("↑"))
                .end_symbol(Some("↓")),
            if has_title { chunks[1] } else { chunks[0] },
            &mut self.vertical_scroll_state,
        );

        if self.mode == TuiMode::SearchInputText {
            let text = match self.tips.into_text() {
                Ok(t) => t,
                Err(e) => {
                    debug!("{e}");
                    return;
                }
            };

            let tips_area = if has_title { chunks[2] } else { chunks[1] };
            f.render_widget(
                Paragraph::new(text).block(Block::default().borders(Borders::ALL)),
                tips_area,
            );

            // Show cursor at the correct position within the search query.
            // We measure the width of the tips prefix up to the cursor position.
            let query = self.search_input.value();
            let prefix = &query[..self.search_input.visual_cursor()];
            let cursor_x = tips_area.x
                + 1
                + console::measure_text_width(&self.ui_text.searct_tips_with_query(prefix)) as u16;
            let cursor_y = tips_area.y + 1;
            f.set_cursor_position((cursor_x, cursor_y));
        } else {
            let text = match self.tips.into_text() {
                Ok(t) => t,
                Err(e) => {
                    debug!("{e}");
                    return;
                }
            };
            f.render_widget(
                Paragraph::new(text).block(Block::default().borders(Borders::ALL)),
                if has_title { chunks[2] } else { chunks[1] },
            );
        }
    }
}
