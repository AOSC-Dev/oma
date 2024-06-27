use std::{
    ffi::OsStr,
    fmt::Display,
    io::{self, ErrorKind, Write},
    sync::atomic::AtomicI32,
    time::{Duration, Instant},
};

use ansi_to_tui::IntoText;
use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyModifiers},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{
    backend::{Backend, CrosstermBackend},
    layout::{Alignment, Constraint, Layout},
    style::{Color, Stylize},
    text::Text,
    widgets::{Block, Paragraph, Scrollbar, ScrollbarOrientation, ScrollbarState},
    Frame, Terminal,
};

use crate::{writer::Writer, WRITER};

pub static SUBPROCESS: AtomicI32 = AtomicI32::new(-1);

pub enum Pager {
    Plain,
    External(OmaPager),
}

impl Pager {
    pub fn plain() -> Self {
        Self::Plain
    }

    pub fn external<D: Display + AsRef<OsStr>>(tips: D, title: Option<&str>) -> io::Result<Self> {
        let app = OmaPager::new(tips, title);
        let res = Pager::External(app);

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

    /// Wait pager to exit
    pub fn wait_for_exit(&mut self) -> io::Result<bool> {
        let success = if let Pager::External(app) = self {
            let mut terminal = prepare_create_tui()?;
            let res = app.run(&mut terminal, Duration::from_millis(250))?;
            exit_tui(&mut terminal)?;

            res
        } else {
            true
        };

        Ok(success)
    }
}

pub fn exit_tui(terminal: &mut Terminal<CrosstermBackend<io::Stdout>>) -> io::Result<()> {
    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;

    terminal.show_cursor()?;

    Ok(())
}

pub fn prepare_create_tui() -> io::Result<Terminal<CrosstermBackend<io::Stdout>>> {
    enable_raw_mode()?;

    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;

    let backend = CrosstermBackend::new(stdout);
    let terminal = Terminal::new(backend)?;

    Ok(terminal)
}

pub struct OmaPager {
    inner: String,
    vertical_scroll_state: ScrollbarState,
    horizontal_scroll_state: ScrollbarState,
    vertical_scroll: usize,
    horizontal_scroll: usize,
    text: Option<Text<'static>>,
    area_heigh: u16,
    max_width: u16,
    tips: String,
    title: Option<String>,
    inner_len: usize,
}

impl Write for OmaPager {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        let s = std::str::from_utf8(buf).map_err(|e| io::Error::new(ErrorKind::InvalidInput, e))?;
        self.inner.push_str(s);

        Ok(buf.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

impl OmaPager {
    pub fn new(tips: impl Display + AsRef<OsStr>, title: Option<&str>) -> Self {
        Self {
            inner: String::new(),
            vertical_scroll_state: ScrollbarState::new(0),
            horizontal_scroll_state: ScrollbarState::new(0),
            vertical_scroll: 0,
            horizontal_scroll: 0,
            text: None,
            area_heigh: 0,
            max_width: 0,
            tips: tips.to_string(),
            title: title.map(|x| x.to_string()),
            inner_len: 0,
        }
    }

    pub fn run<B: Backend>(
        &mut self,
        terminal: &mut Terminal<B>,
        tick_rate: Duration,
    ) -> io::Result<bool> {
        let text = self
            .inner
            .into_text()
            .map_err(|e| io::Error::new(ErrorKind::InvalidInput, e))?;

        self.text = Some(text);

        let mut last_tick = Instant::now();
        loop {
            terminal.draw(|f| self.ui(f))?;
            let timeout = tick_rate.saturating_sub(last_tick.elapsed());
            if crossterm::event::poll(timeout)? {
                if let Event::Key(key) = event::read()? {
                    if key.modifiers == KeyModifiers::CONTROL && key.code == KeyCode::Char('c') {
                        return Ok(false);
                    }
                    match key.code {
                        KeyCode::Char('q') => return Ok(true),
                        KeyCode::Char('j') | KeyCode::Down => {
                            if self
                                .vertical_scroll
                                .saturating_add(self.area_heigh as usize)
                                >= self.inner_len
                            {
                                continue;
                            }
                            self.vertical_scroll = self.vertical_scroll.saturating_add(1);
                            self.vertical_scroll_state =
                                self.vertical_scroll_state.position(self.vertical_scroll);
                        }
                        KeyCode::Char('k') | KeyCode::Up => {
                            self.vertical_scroll = self.vertical_scroll.saturating_sub(1);
                            self.vertical_scroll_state =
                                self.vertical_scroll_state.position(self.vertical_scroll);
                        }
                        KeyCode::Char('h') | KeyCode::Left => {
                            let width = WRITER.get_length();
                            self.horizontal_scroll =
                                self.horizontal_scroll.saturating_sub((width / 4).into());
                            self.horizontal_scroll_state = self
                                .horizontal_scroll_state
                                .position(self.horizontal_scroll);
                        }
                        KeyCode::Char('l') | KeyCode::Right => {
                            let width = WRITER.get_length();
                            if self.max_width <= self.horizontal_scroll as u16 + width {
                                continue;
                            }
                            self.horizontal_scroll =
                                self.horizontal_scroll.saturating_add((width / 4).into());
                            self.horizontal_scroll_state = self
                                .horizontal_scroll_state
                                .position(self.horizontal_scroll);
                        }
                        KeyCode::Char('g') => {
                            self.vertical_scroll = 0;
                            self.vertical_scroll_state =
                                self.vertical_scroll_state.position(self.vertical_scroll);
                        }
                        KeyCode::Char('G') => {
                            self.vertical_scroll = self.inner_len.saturating_sub(1);
                            self.vertical_scroll_state =
                                self.vertical_scroll_state.position(self.vertical_scroll);
                        }
                        KeyCode::PageUp => {
                            self.vertical_scroll = self
                                .vertical_scroll
                                .saturating_sub(self.area_heigh as usize);
                            self.vertical_scroll_state =
                                self.vertical_scroll_state.position(self.vertical_scroll);
                        }
                        KeyCode::PageDown => {
                            let pos = self
                                .vertical_scroll
                                .saturating_add(self.area_heigh as usize);
                            if pos <= self.inner_len {
                                self.vertical_scroll = pos;
                            } else {
                                continue;
                            }
                            self.vertical_scroll_state =
                                self.vertical_scroll_state.position(self.vertical_scroll);
                        }
                        _ => {}
                    }
                }
            }
            if last_tick.elapsed() >= tick_rate {
                last_tick = Instant::now();
            }
        }
    }

    fn ui(&mut self, f: &mut Frame) {
        let area = f.area();
        let mut layout = vec![Constraint::Min(0), Constraint::Length(1)];

        let mut has_title = false;
        if self.title.is_some() {
            layout.insert(0, Constraint::Length(1));
            has_title = true;
        }

        let chunks = Layout::vertical(layout).split(area);

        let inner = self.inner.lines().collect::<Vec<_>>();

        let width = inner.iter().map(|x| x.chars().count()).max().unwrap_or(1);

        self.inner_len = inner.len();
        self.vertical_scroll_state = self
            .vertical_scroll_state
            .content_length(self.inner_len.saturating_sub(self.area_heigh as usize));

        self.vertical_scroll_state = self
            .vertical_scroll_state
            .viewport_content_length(self.inner_len.saturating_sub(self.area_heigh as usize));

        let width = if width <= WRITER.get_length().into() {
            0
        } else {
            width
        };

        self.horizontal_scroll_state = self.horizontal_scroll_state.content_length(width);

        self.max_width = width as u16;

        if let Some(title) = &self.title {
            let title = Block::new()
                .title_alignment(Alignment::Left)
                .title(title.to_string())
                .fg(Color::White)
                .bg(Color::Indexed(25));

            f.render_widget(title, chunks[0]);
        }

        f.render_widget(
            Paragraph::new(self.text.clone().unwrap())
                .scroll((self.vertical_scroll as u16, self.horizontal_scroll as u16)),
            if has_title { chunks[1] } else { chunks[0] },
        );

        f.render_stateful_widget(
            Scrollbar::new(ScrollbarOrientation::VerticalRight)
                .begin_symbol(Some("↑"))
                .end_symbol(Some("↓")),
            if has_title { chunks[1] } else { chunks[0] },
            &mut self.vertical_scroll_state,
        );

        self.area_heigh = if has_title {
            chunks[1].height
        } else {
            chunks[0].height
        };

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
