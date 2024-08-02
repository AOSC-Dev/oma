use std::{
    default, io,
    time::{Duration, Instant},
};

use crossterm::event::{self, Event, KeyCode, KeyModifiers};
use num_enum::IntoPrimitive;
use ratatui::{
    layout::{Constraint, Layout},
    prelude::Backend,
    widgets::Tabs,
    Frame, Terminal,
};

use strum::{Display, EnumIter, FromRepr, IntoEnumIterator};

pub struct OmaTui {
    tab: OmaTuiTab,
}

#[derive(IntoPrimitive, Default, Clone, Copy, Display, FromRepr, EnumIter)]
#[repr(u8)]
enum OmaTuiTab {
    #[default]
    #[strum(to_string = "Main (1)")]
    Main = 1,
    #[strum(to_string = "Hisotry (2)")]
    History,
    #[strum(to_string = "Topics (3)")]
    Topics,
}

impl OmaTui {
    pub fn new() -> Self {
        OmaTui {
            tab: OmaTuiTab::Main,
        }
    }

    pub fn run<B: Backend>(
        &mut self,
        terminal: &mut Terminal<B>,
        tick_rate: Duration,
    ) -> io::Result<bool> {
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
                        KeyCode::Char('1') => self.tab = OmaTuiTab::Main,
                        KeyCode::Char('2') => self.tab = OmaTuiTab::History,
                        KeyCode::Char('3') => self.tab = OmaTuiTab::Topics,
                        _ => {} // Ignore other keys. If you want to handle them, add here.
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
        let chunks = Layout::default()
            .constraints([Constraint::Length(1), Constraint::Min(0)])
            .split(size);

        f.render_widget(
            Tabs::new(OmaTuiTab::iter().map(|x| x.to_string()))
                .select(u8::from(self.tab) as usize - 1),
            chunks[0],
        );

        match self.tab {
            OmaTuiTab::Main => {
                // Main tab UI
                // Render widgets here
            }
            OmaTuiTab::History => {
                // History tab UI
                // Render widgets here
            }
            OmaTuiTab::Topics => {
                // Topics tab UI
                // Render widgets here
            }
        }
    }
}
