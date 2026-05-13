mod key_binding;
mod state;

use std::{
    io,
    time::{Duration, Instant},
};

use chrono::{Local, LocalResult, TimeZone};
use oma_history::HistoryEntry;
use ratatui::{
    Frame, Terminal,
    backend::Backend,
    crossterm::event,
    layout::{Constraint, Layout},
    style::{Color, Style},
    widgets::{Block, Borders, Row, ScrollbarState, Table},
};

use crate::{error::OutputError, subcommand::history_tui::state::StatefulList};

pub struct HistorySelectTui<'a> {
    history: StatefulList<'a, HistoryEntry>,
    scroll_state: ScrollbarState,
    first_selected: usize,
}

impl<'a> HistorySelectTui<'a> {
    pub fn new(entries: &'a [HistoryEntry], first_selected: usize) -> Result<Self, OutputError> {
        let len = entries.len();
        Ok(Self {
            history: StatefulList::with_items(entries),
            scroll_state: ScrollbarState::new(len),
            first_selected,
        })
    }

    pub fn run<B: Backend>(
        mut self,
        terminal: &mut Terminal<B>,
        tick_rate: Duration,
    ) -> io::Result<Option<usize>> {
        let mut last_tick = Instant::now();
        self.history.state.select(Some(self.first_selected));

        loop {
            terminal
                .draw(|f| self.ui(f))
                .map_err(|e| io::Error::other(e.to_string()))?;

            if event::poll(tick_rate)? {
                let event::Event::Key(key) = event::read()? else {
                    continue;
                };

                match self.handle_key_event(key) {
                    std::ops::ControlFlow::Continue(()) => continue,
                    std::ops::ControlFlow::Break(selected) => return Ok(selected),
                };
            }

            if last_tick.elapsed() >= tick_rate {
                last_tick = Instant::now();
            }
        }
    }

    fn ui(&mut self, f: &mut Frame) {
        let main_layout = Layout::default()
            .direction(ratatui::layout::Direction::Horizontal)
            .constraints([Constraint::Length(120)])
            .split(f.area());

        let items = self.history.items.iter().map(|item| {
            Row::new(vec![
                item.command.clone(),
                if item.is_success { "✅" } else { "❌" }.to_string(),
                {
                    let mut operation = vec![];
                    if item.install_count != 0 {
                        operation.push("I");
                    }
                    if item.remove_count != 0 {
                        operation.push("R");
                    }
                    if item.upgrade_count != 0 {
                        operation.push("U");
                    }
                    if item.downgrade_count != 0 {
                        operation.push("D");
                    }
                    if item.reinstall_count != 0 {
                        operation.push("Re");
                    }
                    operation.join(",")
                },
                {
                    let dt = match Local.timestamp_opt(item.time, 0) {
                        LocalResult::None => Local.timestamp_opt(0, 0).unwrap(),
                        x => x.unwrap(),
                    };

                    dt.format("%H:%M:%S on %Y-%m-%d").to_string()
                },
            ])
        });

        let widths = [Constraint::Percentage(25); 4];

        let table = Table::new(items, widths)
            .header(
                Row::new(vec!["Command", "Status", "Operations", "Date"])
                    .style(Style::default().fg(Color::Yellow))
                    .bottom_margin(1),
            )
            // 设置外部边框
            .block(Block::default().title("History List").borders(Borders::ALL))
            .row_highlight_style(Style::new().bg(Color::Blue));

        f.render_stateful_widget(table, main_layout[0], &mut self.history.state);
    }
}
