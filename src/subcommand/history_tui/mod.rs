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
    layout::{Constraint, Direction, Layout},
    style::{Color, Style, Stylize},
    text::{Line, Span},
    widgets::{
        Block, Borders, Cell, Paragraph, Row, Scrollbar, ScrollbarOrientation, ScrollbarState,
        Table, Wrap,
    },
};
use spdlog::debug;
use terminfo::{Database, capability::MaxColors};

use crate::{error::OutputError, subcommand::history_tui::state::StatefulList};

pub struct HistorySelectTui<'a> {
    history: StatefulList<'a, HistoryEntry>,
    scroll_state: ScrollbarState,
    first_selected: usize,
    page_size: usize,
    select_color: Color,
}

impl<'a> HistorySelectTui<'a> {
    pub fn new(entries: &'a [HistoryEntry], first_selected: usize) -> Result<Self, OutputError> {
        let len = entries.len();
        let true_colors = Database::from_env()
            .inspect_err(|e| debug!("Failed to get terminfo: {e}"))
            .ok()
            .and_then(|terminfo| terminfo.get::<MaxColors>())
            .inspect(|MaxColors(n)| debug!("Terminal max colors: {n}"))
            .is_some_and(|MaxColors(n)| n >= 256);

        Ok(Self {
            history: StatefulList::with_items(entries),
            scroll_state: ScrollbarState::new(len),
            first_selected,
            page_size: 0,
            select_color: if true_colors {
                Color::Rgb(59, 64, 70)
            } else {
                Color::Blue
            },
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
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
            .split(f.area());

        let cmd_column_width = ((main_layout[0].width as f32 * 0.4) as usize).saturating_sub(3);
        self.page_size = main_layout[0].height.saturating_sub(4) as usize;

        let items = self.history.items.iter().map(|item| {
            let cmd_display = if item.command.len() > cmd_column_width && cmd_column_width > 3 {
                format!("{:.width$}...", item.command, width = cmd_column_width - 3)
            } else {
                item.command.clone()
            };

            Row::new(vec![
                Cell::new(cmd_display),
                if item.is_success {
                    Cell::new("✓").style(Color::Green).bold()
                } else {
                    Cell::new("X").style(Color::Red).bold()
                },
                {
                    let mut operation = vec![];
                    if item.install_count != 0 {
                        operation.push(Span::from("I").style(Color::Green).bold());
                    }
                    if item.remove_count != 0 {
                        operation.push(Span::from("R").style(Color::Red).bold());
                    }
                    if item.upgrade_count != 0 {
                        operation.push(Span::from("U").style(Color::Cyan).bold());
                    }
                    if item.downgrade_count != 0 {
                        operation.push(Span::from("D").style(Color::Yellow).bold());
                    }
                    if item.reinstall_count != 0 {
                        operation.push(Span::from("Re").style(Color::Gray).bold());
                    }

                    let line: Line = operation
                        .into_iter()
                        .enumerate()
                        .flat_map(|(i, span)| {
                            if i > 0 {
                                vec![Span::raw(","), span]
                            } else {
                                vec![span]
                            }
                        })
                        .collect();

                    Cell::from(line)
                },
                {
                    let dt = match Local.timestamp_opt(item.time, 0) {
                        LocalResult::None => Local.timestamp_opt(0, 0).unwrap(),
                        x => x.unwrap(),
                    };

                    Cell::new(dt.format("%H:%M:%S on %Y-%m-%d").to_string())
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
                Row::new(vec!["Command", "Status", "Ops", "Date"])
                    .style(Style::default().fg(Color::Yellow))
                    .bottom_margin(1),
            )
            // 设置外部边框
            .block(Block::default().title("History List").borders(Borders::ALL))
            .row_highlight_style(Style::new().bg(self.select_color));

        f.render_stateful_widget(table, main_layout[0], &mut self.history.state);
        f.render_stateful_widget(
            Scrollbar::new(ScrollbarOrientation::VerticalRight)
                .begin_symbol(Some("↑"))
                .end_symbol(Some("↓")),
            main_layout[0],
            &mut self.scroll_state,
        );

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
