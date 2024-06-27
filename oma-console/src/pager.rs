use std::{
    ffi::OsStr,
    fmt::Display,
    io::{self, ErrorKind, Write},
    sync::atomic::AtomicI32,
    time::{Duration, Instant},
};

use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{
    backend::{Backend, CrosstermBackend},
    layout::{Alignment, Constraint, Layout},
    style::Stylize,
    text::{Line, ToLine},
    widgets::{Block, Paragraph, Scrollbar, ScrollbarOrientation, ScrollbarState},
    Frame, Terminal,
};

use crate::writer::Writer;

pub static SUBPROCESS: AtomicI32 = AtomicI32::new(-1);

pub enum Pager {
    Plain,
    External(OmaPager),
}

impl Pager {
    pub fn plain() -> Self {
        Self::Plain
    }

    pub fn external<D: Display + AsRef<OsStr>>(tips: D) -> io::Result<Self> {
        let app = OmaPager::new();
        let res = Pager::External(app);

        Ok(res)
    }

    /// Get pager name (like less)
    pub fn pager_name(&mut self) -> Option<&str> {
        match self {
            Pager::Plain => None,
            Pager::External(_) => Some("oma"),
        }
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
            enable_raw_mode()?;
            let mut stdout = io::stdout();
            execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
            let backend = CrosstermBackend::new(stdout);
            let mut terminal = Terminal::new(backend)?;
            let tick_rate = Duration::from_millis(250);

            let res = app.run(&mut terminal, tick_rate);

            disable_raw_mode()?;
            execute!(
                terminal.backend_mut(),
                LeaveAlternateScreen,
                DisableMouseCapture
            )?;
            terminal.show_cursor()?;

            res.is_ok()
        } else {
            true
        };

        Ok(success)
    }
}

pub struct OmaPager {
    inner: String,
    vertical_scroll_state: ScrollbarState,
    horizontal_scroll_state: ScrollbarState,
    vertical_scroll: usize,
    horizontal_scroll: usize,
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
    pub fn new() -> Self {
        Self {
            inner: String::new(),
            vertical_scroll_state: ScrollbarState::new(0),
            horizontal_scroll_state: ScrollbarState::new(0),
            vertical_scroll: 0,
            horizontal_scroll: 0,
        }
    }

    pub fn run<B: Backend>(
        &mut self,
        terminal: &mut Terminal<B>,
        tick_rate: Duration,
    ) -> io::Result<()> {
        let mut last_tick = Instant::now();
        loop {
            terminal.draw(|f| self.ui(f))?;
            let timeout = tick_rate.saturating_sub(last_tick.elapsed());
            if crossterm::event::poll(timeout)? {
                if let Event::Key(key) = event::read()? {
                    match key.code {
                        KeyCode::Char('q') => return Ok(()),
                        KeyCode::Char('j') | KeyCode::Down => {
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
                            self.horizontal_scroll = self.horizontal_scroll.saturating_sub(1);
                            self.horizontal_scroll_state = self
                                .horizontal_scroll_state
                                .position(self.horizontal_scroll);
                        }
                        KeyCode::Char('l') | KeyCode::Right => {
                            self.horizontal_scroll = self.horizontal_scroll.saturating_add(1);
                            self.horizontal_scroll_state = self
                                .horizontal_scroll_state
                                .position(self.horizontal_scroll);
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
        let size = f.size();
        let chunks = Layout::vertical([
            Constraint::Length(1),
            Constraint::Min(0),
            Constraint::Length(1),
        ])
        .split(size);

        let inner = self.inner.lines().collect::<Vec<_>>();
        let text: Vec<Line> = inner.iter().map(|x| x.to_line()).collect();

        let weight = inner.iter().map(|x| x.len()).max().unwrap_or(1);

        self.vertical_scroll_state = self.vertical_scroll_state.content_length(text.len());
        self.horizontal_scroll_state = self.horizontal_scroll_state.content_length(weight);

        let title = Block::new()
            .title_alignment(Alignment::Left)
            .title("oma".bold());
        f.render_widget(title, chunks[0]);

        let paragraph = Paragraph::new(text.clone())
            .gray()
            .block(Block::bordered().gray().title("Review".bold()))
            .scroll((self.vertical_scroll as u16, 0));
        f.render_widget(paragraph, chunks[1]);
        f.render_stateful_widget(
            Scrollbar::new(ScrollbarOrientation::VerticalRight)
                .begin_symbol(Some("↑"))
                .end_symbol(Some("↓")),
            chunks[1],
            &mut self.vertical_scroll_state,
        );
    }
}

#[test]
fn test_oma_pager() {
    use crossterm::{
        event::{DisableMouseCapture, EnableMouseCapture},
        execute,
        terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
    };
    use ratatui::prelude::CrosstermBackend;

    enable_raw_mode().unwrap();
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture).unwrap();
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend).unwrap();
    let tick_rate = Duration::from_millis(250);
    let mut app = OmaPager::new();
    app.write_all(b"Hello\noma").unwrap();
    app.run(&mut terminal, tick_rate).unwrap();

    // restore terminal
    disable_raw_mode().unwrap();
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )
    .unwrap();
    terminal.show_cursor().unwrap();
}
