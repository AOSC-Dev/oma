use std::{
    io::{self, BufRead, ErrorKind, IsTerminal, Write, stderr, stdin, stdout},
    time::{Duration, Instant},
};

use aho_corasick::{AhoCorasick, BuildError};
use ansi_to_tui::IntoText;
use crossterm::{
    event::{self, Event, KeyCode, KeyModifiers},
    execute,
    terminal::{EnterAlternateScreen, enable_raw_mode},
};
use ratatui::{
    Frame, Terminal,
    backend::{Backend, CrosstermBackend},
    layout::{Alignment, Constraint, Layout},
    restore,
    style::{Color, Stylize},
    text::Text,
    widgets::{Block, Paragraph, Scrollbar, ScrollbarOrientation, ScrollbarState},
};
use termbg::Theme;
use tracing::debug;

use crate::{print::OmaColorFormat, writer::Writer};

pub enum Pager<'a> {
    Plain,
    External(Box<OmaPager<'a>>),
}

impl<'a> Pager<'a> {
    pub fn plain() -> Self {
        Self::Plain
    }

    pub fn external(
        ui_text: &'a dyn PagerUIText,
        title: Option<String>,
        color_format: &'a OmaColorFormat,
    ) -> io::Result<Self> {
        if !stdout().is_terminal() || !stderr().is_terminal() || !stdin().is_terminal() {
            return Ok(Pager::Plain);
        }

        let app = OmaPager::new(title, color_format, ui_text);
        let res = Pager::External(Box::new(app));

        Ok(res)
    }

    /// Get writer to writer something to pager
    pub fn get_writer(&mut self) -> io::Result<Box<dyn Write + '_>> {
        let res = match self {
            Pager::Plain => Writer::new_stdout().get_writer(),
            Pager::External(app) => {
                let res: Box<dyn Write> = Box::new(app);
                res
            }
        };

        Ok(res)
    }

    /// Wait for the pager to exit
    /// Use this function to start the pager
    pub fn wait_for_exit(self) -> io::Result<PagerExit> {
        let success = if let Pager::External(app) = self {
            let mut terminal = prepare_create_tui()?;
            let res = app.run(&mut terminal, Duration::from_millis(250))?;
            exit_tui(&mut terminal)?;

            res
        } else {
            PagerExit::NormalExit
        };

        Ok(success)
    }
}

pub fn exit_tui(terminal: &mut Terminal<CrosstermBackend<io::Stdout>>) -> io::Result<()> {
    restore();
    terminal.show_cursor()?;

    Ok(())
}

pub fn prepare_create_tui() -> io::Result<Terminal<CrosstermBackend<io::Stdout>>> {
    let hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        restore();
        hook(info);
    }));

    execute!(stdout(), EnterAlternateScreen)?;
    enable_raw_mode()?;

    let backend = CrosstermBackend::new(stdout());
    let mut terminal = Terminal::new(backend)?;

    terminal.clear()?;

    Ok(terminal)
}

enum PagerInner {
    Working(Vec<u8>),
    Finished(Vec<String>),
}

/// `OmaPager` is a structure that implements a pager displaying text-based content in a terminal UI.
pub struct OmaPager<'a> {
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
    tips: String,
    /// An optional title for the pager.
    title: Option<String>,
    /// The length of the inner content.
    inner_len: usize,
    /// A reference to the color format used for the pager's theme.
    theme: &'a OmaColorFormat,
    /// A vector containing the indices of search results.
    search_results: Vec<usize>,
    /// The index of the current search result being displayed.
    current_result_index: usize,
    /// The current mode of the pager, which can be either `Normal`, `Search` and `SearchInputText`.
    mode: TuiMode,
    /// A reference to a trait object that provides UI text for the pager.
    ui_text: &'a dyn PagerUIText,
    /// A terminal writer to print oma-style message
    writer: Writer,
}

impl Write for OmaPager<'_> {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        match self.inner {
            PagerInner::Working(ref mut v) => v.extend_from_slice(buf),
            PagerInner::Finished(_) => {
                return Err(io::Error::new(ErrorKind::Other, "write is finished"));
            }
        }

        Ok(buf.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

pub trait PagerUIText {
    fn normal_tips(&self) -> String;
    fn search_tips_with_result(&self) -> String;
    fn searct_tips_with_query(&self, query: &str) -> String;
    fn search_tips_with_empty(&self) -> String;
    fn search_tips_not_found(&self) -> String;
}

#[derive(PartialEq, Eq)]
enum TuiMode {
    Search,
    SearchInputText,
    Noemal,
}

pub enum PagerExit {
    NormalExit,
    Sigint,
    DryRun,
}

impl From<PagerExit> for i32 {
    fn from(value: PagerExit) -> Self {
        match value {
            PagerExit::NormalExit => 0,
            PagerExit::Sigint => 130,
            PagerExit::DryRun => 0,
        }
    }
}

impl<'a> OmaPager<'a> {
    pub fn new(
        title: Option<String>,
        theme: &'a OmaColorFormat,
        ui_text: &'a dyn PagerUIText,
    ) -> Self {
        Self {
            inner: PagerInner::Working(vec![]),
            vertical_scroll_state: ScrollbarState::new(0),
            horizontal_scroll_state: ScrollbarState::new(0),
            vertical_scroll: 0,
            horizontal_scroll: 0,
            area_height: 0,
            max_width: 0,
            tips: ui_text.normal_tips(),
            title,
            inner_len: 0,
            theme,
            search_results: Vec::new(),
            current_result_index: 0,
            mode: TuiMode::Noemal,
            ui_text,
            writer: Writer::default(),
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
            return Err(io::Error::new(ErrorKind::Other, "write is finished"));
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

        let mut query = String::new();

        let mut last_tick = Instant::now();
        // Start the loop, waiting for the keyboard interrupts.
        loop {
            terminal.draw(|f| self.ui(f))?;
            let timeout = tick_rate.saturating_sub(last_tick.elapsed());
            if crossterm::event::poll(timeout)? {
                match event::read()? {
                    Event::Key(key) => {
                        if key.modifiers == KeyModifiers::CONTROL && key.code == KeyCode::Char('c')
                        {
                            return Ok(PagerExit::Sigint);
                        }
                        match key.code {
                            KeyCode::Char(c) if c == 'q' || c == 'Q' => {
                                if self.mode == TuiMode::SearchInputText {
                                    query.push(c);
                                    self.tips = self.ui_text.searct_tips_with_query(&query);
                                    continue;
                                }
                                return Ok(PagerExit::NormalExit);
                            }
                            KeyCode::Down => {
                                self.down();
                            }
                            KeyCode::Up => {
                                self.up();
                            }
                            KeyCode::Left => {
                                self.left();
                            }
                            KeyCode::Right => {
                                self.right();
                            }
                            KeyCode::Char('j') => {
                                if self.mode == TuiMode::SearchInputText {
                                    query.push('j');
                                    self.tips = self.ui_text.searct_tips_with_query(&query);
                                    continue;
                                }
                                self.down();
                            }
                            KeyCode::Char('k') => {
                                if self.mode == TuiMode::SearchInputText {
                                    query.push('k');
                                    self.tips = self.ui_text.searct_tips_with_query(&query);
                                    continue;
                                }
                                self.up();
                            }
                            KeyCode::Char('h') => {
                                if self.mode == TuiMode::SearchInputText {
                                    query.push('h');
                                    self.tips = self.ui_text.searct_tips_with_query(&query);
                                    continue;
                                }
                                self.left();
                            }
                            KeyCode::Char('l') => {
                                if self.mode == TuiMode::SearchInputText {
                                    query.push('l');
                                    self.tips = self.ui_text.searct_tips_with_query(&query);
                                    continue;
                                }
                                self.right();
                            }
                            KeyCode::Char('g') => {
                                if self.mode == TuiMode::SearchInputText {
                                    query.push('g');
                                    self.tips = self.ui_text.searct_tips_with_query(&query);
                                    continue;
                                }
                                self.goto_begin();
                            }
                            KeyCode::Char('G') => {
                                if self.mode == TuiMode::SearchInputText {
                                    query.push('G');
                                    self.tips = self.ui_text.searct_tips_with_query(&query);
                                    continue;
                                }
                                self.goto_end();
                            }
                            KeyCode::Enter => {
                                if self.mode != TuiMode::SearchInputText {
                                    continue;
                                }
                                if query.trim().is_empty() {
                                    self.tips = self.ui_text.search_tips_with_empty();
                                } else {
                                    self.search_results = self.search(&query);
                                    if self.search_results.is_empty() {
                                        self.tips = self.ui_text.search_tips_not_found();
                                    } else {
                                        self.current_result_index = 0;
                                        self.jump_to(
                                            self.search_results[self.current_result_index],
                                        );
                                        self.tips = self.ui_text.search_tips_with_result();
                                    }
                                }
                                self.mode = TuiMode::Search;
                            }
                            KeyCode::Esc => {
                                if self.mode != TuiMode::Noemal {
                                    self.mode = TuiMode::Noemal;
                                }
                                // clear highlight
                                self.clear_highlight();
                                // clear search tips
                                self.tips = self.ui_text.normal_tips();
                            }
                            KeyCode::Backspace => {
                                if self.mode == TuiMode::SearchInputText {
                                    query.pop();
                                    // update tips with search patterns
                                    self.tips = self.ui_text.searct_tips_with_query(&query);
                                }
                            }
                            KeyCode::Char('/') => {
                                if self.mode != TuiMode::SearchInputText {
                                    self.clear_highlight();
                                    self.mode = TuiMode::SearchInputText;
                                    // update tips with search patterns
                                    self.tips = self.ui_text.searct_tips_with_query(&query);
                                } else {
                                    query.push('/');
                                    self.tips = self.ui_text.searct_tips_with_query(&query);
                                    continue;
                                }
                            }
                            KeyCode::Char('n') => match self.mode {
                                TuiMode::Search => {
                                    if !self.search_results.is_empty() {
                                        self.current_result_index = (self.current_result_index + 1)
                                            % self.search_results.len();
                                        self.jump_to(
                                            self.search_results[self.current_result_index],
                                        );
                                    }
                                }
                                TuiMode::SearchInputText => {
                                    query.push('n');
                                    self.tips = self.ui_text.searct_tips_with_query(&query);
                                    continue;
                                }
                                TuiMode::Noemal => continue,
                            },
                            KeyCode::Char('N') => match self.mode {
                                TuiMode::Search => {
                                    if !self.search_results.is_empty() {
                                        if self.current_result_index == 0 {
                                            self.current_result_index =
                                                self.search_results.len() - 1;
                                        } else {
                                            self.current_result_index -= 1;
                                        }
                                        self.jump_to(
                                            self.search_results[self.current_result_index],
                                        );
                                    }
                                }
                                TuiMode::SearchInputText => {
                                    query.push('N');
                                    self.tips = self.ui_text.searct_tips_with_query(&query);
                                    continue;
                                }
                                TuiMode::Noemal => continue,
                            },
                            KeyCode::Char(c) if c == 'u' || c == 'U' => {
                                if self.mode == TuiMode::SearchInputText {
                                    query.push(c);
                                    self.tips = self.ui_text.searct_tips_with_query(&query);
                                    continue;
                                }
                                self.page_up();
                            }
                            KeyCode::Char(c) if c == 'd' || c == 'D' => {
                                if self.mode == TuiMode::SearchInputText {
                                    query.push(c);
                                    self.tips = self.ui_text.searct_tips_with_query(&query);
                                    continue;
                                }
                                self.page_down();
                            }
                            KeyCode::Char(input_char) => {
                                if self.mode == TuiMode::SearchInputText {
                                    query.push(input_char);
                                    // update tips with search patterns
                                    self.tips = self.ui_text.searct_tips_with_query(&query);
                                }
                            }
                            KeyCode::PageUp => {
                                self.page_up();
                            }
                            KeyCode::PageDown => {
                                self.page_down();
                            }
                            KeyCode::End => {
                                self.goto_end();
                            }
                            KeyCode::Home => {
                                self.goto_begin();
                            }
                            _ => {}
                        }
                    }
                    _ => continue,
                }
            }
            if last_tick.elapsed() >= tick_rate {
                last_tick = Instant::now();
            }
        }
    }

    fn page_down(&mut self) {
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

    fn page_up(&mut self) {
        self.vertical_scroll = self
            .vertical_scroll
            .saturating_sub(self.area_height as usize);
        self.vertical_scroll_state = self.vertical_scroll_state.position(self.vertical_scroll);
    }

    fn goto_end(&mut self) {
        self.vertical_scroll = self.inner_len.saturating_sub(self.area_height.into());
        self.vertical_scroll_state = self.vertical_scroll_state.position(self.vertical_scroll);
    }

    fn goto_begin(&mut self) {
        self.vertical_scroll = 0;
        self.vertical_scroll_state = self.vertical_scroll_state.position(0);
    }

    fn right(&mut self) {
        let width = self.writer.get_length();

        if self.max_width <= self.horizontal_scroll as u16 + width {
            return;
        }

        self.horizontal_scroll = self.horizontal_scroll.saturating_add((width / 4).into());
        self.horizontal_scroll_state = self
            .horizontal_scroll_state
            .position(self.horizontal_scroll);
    }

    fn left(&mut self) {
        let width = self.writer.get_length();
        self.horizontal_scroll = self.horizontal_scroll.saturating_sub((width / 4).into());
        self.horizontal_scroll_state = self
            .horizontal_scroll_state
            .position(self.horizontal_scroll);
    }

    fn up(&mut self) {
        self.vertical_scroll = self.vertical_scroll.saturating_sub(1);
        self.vertical_scroll_state = self.vertical_scroll_state.position(self.vertical_scroll);
    }

    fn down(&mut self) {
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

    /// Jump to line
    fn jump_to(&mut self, line: usize) {
        self.vertical_scroll = line;
        self.vertical_scroll_state = self.vertical_scroll_state.position(self.vertical_scroll);
    }

    fn clear_highlight(&mut self) {
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
        let mut layout = vec![Constraint::Min(0), Constraint::Length(1)];

        let mut has_title = false;
        if self.title.is_some() {
            layout.insert(0, Constraint::Length(1));
            has_title = true;
        }

        let chunks = Layout::vertical(layout).split(area);

        let color = self.theme.theme;

        let title_bg_color = match color {
            Some(Theme::Dark) => Color::Indexed(25),
            Some(Theme::Light) => Color::Indexed(189),
            None => Color::Indexed(25),
        };

        let title_fg_color = match color {
            Some(Theme::Dark) => Color::White,
            Some(Theme::Light) => Color::Black,
            None => Color::White,
        };

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

        f.render_widget(
            Paragraph::new(
                Text::from(self.tips.clone())
                    .bg(Color::White)
                    .fg(Color::Black),
            ),
            if has_title { chunks[2] } else { chunks[1] },
        );
    }
}

struct Highlight<'a> {
    pattern: &'a str,
    ac: AhoCorasick,
}

impl<'a> Highlight<'a> {
    fn new(pattern: &'a str) -> Result<Self, BuildError> {
        Ok(Self {
            ac: AhoCorasick::new([pattern])?,
            pattern,
        })
    }

    fn replace(&self, input: &str) -> String {
        self.ac
            .replace_all(input, &[format!("\x1b[47m{}\x1b[49m", self.pattern)])
    }
}

struct ClearHighlight(AhoCorasick);

impl ClearHighlight {
    fn new() -> Self {
        Self(AhoCorasick::new(["\x1b[47m", "\x1b[49m"]).unwrap())
    }

    fn replace(&self, input: &str) -> String {
        self.0.replace_all(input, &["", ""])
    }
}
