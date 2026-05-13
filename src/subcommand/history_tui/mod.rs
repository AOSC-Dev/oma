mod key_binding;
mod state;

use std::{
    io,
    ops::ControlFlow,
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
    text::Line,
    widgets::{Block, Borders, Paragraph, Row, ScrollbarState, Table, Wrap},
};

use crate::{WRITER, error::OutputError, subcommand::history_tui::state::StatefulList};

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
                    ControlFlow::Continue(()) => continue,
                    ControlFlow::Break(selected) => return Ok(selected),
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
            .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
            .split(f.area());

        let cmd_len = (WRITER.get_max_len() as f32 * 0.5 * 0.4) as usize;

        let items = self.history.items.iter().map(|item| {
            let cmd = textwrap::wrap(&item.command, cmd_len)
                .iter()
                .next()
                .map(|s| {
                    // 3 是 ... 的长度
                    // 2 是左右两边边框的长度
                    // 2 是 Command 左右两边空格的长度
                    if s.len() <= cmd_len - 3 - 2 - 2 {
                        s.to_string()
                    } else {
                        format!("{s}...")
                    }
                })
                .unwrap_or_else(|| item.command.clone());

            Row::new(vec![
                cmd,
                if item.is_success { "Succeed" } else { "Failed" }.to_string(),
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

        let widths = [
            Constraint::Percentage(40),
            Constraint::Percentage(10),
            Constraint::Percentage(10),
            Constraint::Percentage(40),
        ];

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

        let selected = self.history.state.selected().unwrap_or(0);
        let entry = &self.history.items[selected];
        let mut display = vec![];

        if entry.is_success {
            display.push(Line::raw("Installation Succeeded"));
        } else {
            display.push(Line::raw("Installation Failed"));
        }

        let mut operations = vec![];
        if entry.install_count > 0 {
            operations.push(format!("{} installed", entry.install_count));
        }

        if entry.upgrade_count > 0 {
            operations.push(format!("{} upgraded", entry.upgrade_count));
        }

        if entry.downgrade_count > 0 {
            operations.push(format!("{} downgraded", entry.downgrade_count));
        }

        if entry.remove_count > 0 {
            operations.push(format!("{} removed", entry.remove_count));
        }

        if entry.reinstall_count > 0 {
            operations.push(format!("{} reinstalled", entry.reinstall_count));
        }

        if !operations.is_empty() {
            display.push(Line::raw(operations.join(", ")));
        }

        if !entry.command.is_empty() {
            display.push(Line::raw(""));
            display.push(Line::raw("Command Line:"));
            display.push(Line::raw(&entry.command));
        }

        f.render_widget(
            Paragraph::new(display)
                .block(Block::bordered())
                .wrap(Wrap { trim: true }),
            main_layout[1],
        );
    }
}
