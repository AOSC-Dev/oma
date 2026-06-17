mod key_binding;
mod state;

use std::{
    borrow::Cow,
    io,
    ops::ControlFlow,
    rc::Rc,
    time::{Duration, Instant},
};

use chrono::{Local, LocalResult, TimeZone};
use oma_history::HistoryEntry;
use ratatui::{
    Frame, Terminal,
    backend::Backend,
    crossterm::event,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style, Stylize},
    text::{Line, Span},
    widgets::{
        Block, Borders, Cell, Paragraph, Row, Scrollbar, ScrollbarOrientation, ScrollbarState,
        Table, Wrap,
    },
};
use spdlog::debug;
use terminfo::{Database, capability::MaxColors};
use tui_input::Input;

use crate::{WRITER, error::OutputError, fl, subcommand::history_tui::state::StatefulList};

#[derive(Clone, Copy)]
pub(crate) enum BgRenderMode {
    Color(Color),
    Reverse,
}

impl BgRenderMode {
    pub(crate) fn to_style(self) -> Style {
        match self {
            BgRenderMode::Color(color) => Style::default().bg(color),
            BgRenderMode::Reverse => Style::default()
                .bg(Color::Reset)
                .fg(Color::Reset)
                .add_modifier(Modifier::REVERSED),
        }
    }
}

pub struct HistorySelectTui<'a> {
    history_list: StatefulList<HistoryEntry>,
    all_entries: Vec<Rc<HistoryEntry>>,
    scroll_state: ScrollbarState,
    page_size: usize,
    bg_render_mode: BgRenderMode,
    undo: bool,
    search_input: Input,
    db: &'a oma_history::History,
}

impl<'a> HistorySelectTui<'a> {
    pub fn new(history_db: &'a oma_history::History, undo: bool) -> Result<Self, OutputError> {
        let entries = history_db
            .list()?
            .into_iter()
            .filter(|e| if undo { !e.is_undo } else { true })
            .map(Rc::new)
            .collect::<Vec<_>>();

        let len = entries.len();
        let true_colors = Database::from_env()
            .inspect_err(|e| debug!("Failed to get terminfo: {e}"))
            .ok()
            .and_then(|terminfo| terminfo.get::<MaxColors>())
            .inspect(|MaxColors(n)| debug!("Terminal max colors: {n}"))
            .is_some_and(|MaxColors(n)| n >= 256);

        let no_color = std::env::var("NO_COLOR").is_ok_and(|s| s == "1" || s == "true");

        let mut history_list = StatefulList::with_items(entries.clone());
        history_list.state.select(Some(0));

        Ok(Self {
            history_list,
            all_entries: entries,
            scroll_state: ScrollbarState::new(len),
            page_size: 0,
            bg_render_mode: if no_color {
                BgRenderMode::Reverse
            } else if true_colors {
                BgRenderMode::Color(Color::Rgb(59, 64, 70))
            } else {
                BgRenderMode::Color(Color::Blue)
            },
            undo,
            search_input: Input::new(String::new()),
            db: history_db,
        })
    }

    pub fn run<B: Backend>(
        mut self,
        terminal: &mut Terminal<B>,
        tick_rate: Duration,
    ) -> io::Result<Option<i64>> {
        let mut last_tick = Instant::now();
        loop {
            terminal
                .draw(|f| self.ui(f))
                .map_err(|e| io::Error::other(e.to_string()))?;

            if event::poll(tick_rate)? {
                let event::Event::Key(key) = event::read()? else {
                    continue;
                };

                match self.handle_key_event(key, terminal)? {
                    ControlFlow::Continue(()) => continue,
                    ControlFlow::Break(id) => return Ok(id),
                };
            }

            if last_tick.elapsed() >= tick_rate {
                last_tick = Instant::now();
            }
        }
    }

    fn ui(&mut self, f: &mut Frame) {
        let layout = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Min(0),
                Constraint::Length(3),
                Constraint::Length(1),
            ])
            .split(f.area());

        let main_layout = Layout::default()
            .direction({
                if WRITER.get_length() <= 117 {
                    Direction::Vertical
                } else {
                    Direction::Horizontal
                }
            })
            .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
            .split(layout[0]);

        let cmd_column_width = ((main_layout[0].width as f32 * 0.4) as usize).saturating_sub(3);
        self.page_size = main_layout[0].height.saturating_sub(4) as usize;

        let items = self.history_list.items.iter().map(|item| {
            let cmd_display: Cow<str> =
                if item.command.len() > cmd_column_width && cmd_column_width > 3 {
                    format!("{:.width$}...", item.command, width = cmd_column_width - 3).into()
                } else {
                    item.command.as_str().into()
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
                        operation.push(Span::from("Re").style(Color::Blue).bold());
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
                Row::new(vec![
                    fl!("history-tui-command"),
                    fl!("history-tui-status"),
                    fl!("history-tui-ops"),
                    fl!("history-tui-date"),
                ])
                .style(Style::default().fg(Color::Yellow))
                .bottom_margin(1),
            )
            .block(
                Block::default()
                    .title(vec![
                        Span::raw("I").style(Color::Green).bold(),
                        Span::raw("/"),
                        Span::raw("R").style(Color::Red).bold(),
                        Span::raw("/"),
                        Span::raw("U").style(Color::Cyan).bold(),
                        Span::raw("/"),
                        Span::raw("D").style(Color::Yellow).bold(),
                        Span::raw("/"),
                        Span::raw("Re").style(Color::Blue).bold(),
                        Span::raw(" => "),
                        Span::raw(fl!("install")).style(Color::Green).bold(),
                        Span::raw("/"),
                        Span::raw(fl!("remove")).style(Color::Red).bold(),
                        Span::raw("/"),
                        Span::raw(fl!("upgrade")).style(Color::Cyan).bold(),
                        Span::raw("/"),
                        Span::raw(fl!("downgrade")).style(Color::Yellow).bold(),
                        Span::raw("/"),
                        Span::raw(fl!("reinstall")).style(Color::Blue).bold(),
                    ])
                    .borders(Borders::ALL),
            )
            .row_highlight_style(self.bg_render_mode.to_style());

        f.render_stateful_widget(table, main_layout[0], &mut self.history_list.state);
        f.render_stateful_widget(
            Scrollbar::new(ScrollbarOrientation::VerticalRight)
                .begin_symbol(Some("↑"))
                .end_symbol(Some("↓")),
            main_layout[0],
            &mut self.scroll_state,
        );

        let selected = self.history_list.state.selected();
        let mut display = vec![];

        if let Some(selected) = selected {
            let entry = &self.history_list.items[selected];

            if entry.is_success {
                display.push(Line::raw(fl!("history-tui-installation-succeeded")));
            } else {
                display.push(Line::raw(fl!("history-tui-installation-failed")));
            }

            let mut operations = vec![];
            if entry.install_count > 0 {
                operations.push(format!("{} {}", entry.install_count, fl!("installed")));
            }

            if entry.upgrade_count > 0 {
                operations.push(format!("{} {}", entry.upgrade_count, fl!("upgraded")));
            }

            if entry.downgrade_count > 0 {
                operations.push(format!("{} {}", entry.downgrade_count, fl!("downgraded")));
            }

            if entry.remove_count > 0 {
                operations.push(format!("{} {}", entry.remove_count, fl!("removed")));
            }

            if entry.reinstall_count > 0 {
                operations.push(format!("{} {}", entry.reinstall_count, fl!("reinstalled")));
            }

            if !operations.is_empty() {
                display.push(Line::raw(operations.join(", ")));
            }

            if !entry.command.is_empty() {
                display.push(Line::raw(""));
                display.push(Line::raw(fl!("history-tui-command-line")));
                display.push(Line::raw(&entry.command));
            }
        }

        f.render_widget(
            Paragraph::new(display)
                .block(Block::bordered())
                .wrap(Wrap { trim: true }),
            main_layout[1],
        );

        let value = self.search_input.value();

        // 3. 渲染搜索框组件
        let search_block = Paragraph::new(value).block(
            Block::default()
                .borders(Borders::ALL)
                .title(" Search ")
                .border_style(Style::default().fg(Color::Cyan)),
        );
        f.render_widget(search_block, layout[1]);

        // 4. 【核心】让终端的光标显示在正确的位置上
        // layout[1] 内部扣除边框后的实际输入起点是 x + 1, y + 1
        // self.search_input.visual_scroll() 能够自动处理文本过长时的滚动显示
        let cursor_x = layout[1].x + 1 + (self.search_input.visual_cursor() as u16);
        let cursor_y = layout[1].y + 1;

        // 设置光标位置，Ratatui 会在渲染完成后自动让终端光标闪烁
        f.set_cursor_position((cursor_x, cursor_y));

        render_tips(f, layout, self.undo);
    }
}

fn render_tips(f: &mut Frame<'_>, layout: Rc<[Rect]>, undo: bool) {
    match WRITER.get_length() {
        0..=37 => {}
        38..=65 => {
            f.render_widget(
                Paragraph::new(Line::from(vec![
                    Span::raw("Quicknav: "),
                    Span::styled("Enter", Style::new().blue()),
                    Span::raw(" / "),
                    Span::styled("Space", Style::new().blue()),
                    Span::raw(" / "),
                    Span::styled("Ctrl+C", Style::new().blue()),
                    Span::raw(" / "),
                    Span::styled("Esc", Style::new().blue()),
                ])),
                layout[2],
            );
        }
        66.. if !undo => {
            f.render_widget(
                Paragraph::new(Line::from(vec![
                    Span::styled("Enter/Space", Style::new().blue()),
                    Span::raw(format!(" => {}, ", fl!("history-detail"))),
                    Span::styled("ESC/Ctrl+C", Style::new().blue()),
                    Span::raw(format!(" => {}", fl!("tui-start-7"))),
                ])),
                layout[2],
            );
        }
        66.. => {
            f.render_widget(
                Paragraph::new(Line::from(vec![
                    Span::styled("Enter/Space", Style::new().blue()),
                    Span::raw(format!(" => {}, ", fl!("undo-detail"))),
                    Span::styled("ESC/Ctrl+C", Style::new().blue()),
                    Span::raw(format!(" => {}", fl!("tui-start-7"))),
                ])),
                layout[2],
            );
        }
    }
}
