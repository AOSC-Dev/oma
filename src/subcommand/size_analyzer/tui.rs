use std::{
    io,
    ops::ControlFlow,
    rc::Rc,
    time::{Duration, Instant},
};

use dialoguer::console;
use oma_pm::apt::OmaApt;
use ratatui::{
    Frame, Terminal,
    crossterm::event::{self},
    layout::{Constraint, Direction, Flex, Layout, Rect},
    prelude::Backend,
    style::{Color, Modifier, Style},
    text::{Line, Span, Text},
    widgets::{
        Block, Borders, Clear, List, Paragraph, Scrollbar, ScrollbarOrientation, ScrollbarState,
    },
};
use spdlog::debug;
use terminfo::{Database, capability::MaxColors};

use crate::{
    WRITER, fl,
    subcommand::size_analyzer::{
        get_total_installed_size, installed_packages,
        pkg::PkgWrapper,
        state::{StatefulList, Window},
    },
};

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

pub(crate) struct PkgSizeAnalyzer<'a> {
    pub(crate) apt: &'a OmaApt,
    pub(crate) remove_package: StatefulList<PkgWrapper<'a>>,
    pub(crate) installed: StatefulList<PkgWrapper<'a>>,
    pub(crate) window: Window,
    pub(crate) total_installed_size: u64,
    pub(crate) popup: Option<String>,
    pub(crate) installed_scroll_state: ScrollbarState,
    pub(crate) installed_scroll: usize,
    pub(crate) bg_render_mode: BgRenderMode,
}

impl<'a> PkgSizeAnalyzer<'a> {
    pub(crate) const PAGE_LEN: usize = 10;

    pub(crate) fn new(apt: &'a OmaApt) -> Self {
        let true_colors = Database::from_env()
            .inspect_err(|e| debug!("Failed to get terminfo: {e}"))
            .ok()
            .and_then(|terminfo| terminfo.get::<MaxColors>())
            .inspect(|MaxColors(n)| debug!("Terminal max colors: {n}"))
            .is_some_and(|MaxColors(n)| n >= 256);

        let no_color = std::env::var("NO_COLOR").is_ok_and(|s| s == "1" || s == "true");

        Self {
            apt,
            remove_package: StatefulList::with_items(vec![]),
            installed: StatefulList::with_items(vec![]),
            window: Window::Installed,
            total_installed_size: 0,
            popup: None,
            installed_scroll_state: ScrollbarState::new(0),
            installed_scroll: 0,
            bg_render_mode: if no_color {
                BgRenderMode::Reverse
            } else if true_colors {
                BgRenderMode::Color(Color::Rgb(59, 64, 70))
            } else {
                BgRenderMode::Color(Color::Blue)
            },
        }
    }

    pub(crate) fn run<B: Backend>(
        mut self,
        terminal: &mut Terminal<B>,
        tick_rate: Duration,
    ) -> io::Result<Vec<PkgWrapper<'a>>> {
        let mut last_tick = Instant::now();
        self.installed = StatefulList::with_items(
            installed_packages(self.apt, false)
                .unwrap()
                .into_iter()
                .map(|p| PkgWrapper { pkg: p })
                .collect::<Vec<_>>(),
        );

        self.installed_scroll_state = self
            .installed_scroll_state
            .content_length(self.installed.items.len());
        self.total_installed_size = get_total_installed_size(&self.installed.items);
        self.installed.state.select_first();

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
                    ControlFlow::Break(pkgs) => return Ok(pkgs),
                }
            }

            if last_tick.elapsed() >= tick_rate {
                last_tick = Instant::now();
            }
        }
    }

    fn ui(&mut self, f: &mut Frame) {
        let main_layout = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(1), // header
                Constraint::Min(0),    // packages
                Constraint::Length(1), // tips
            ])
            .split(f.area());

        f.render_widget(
            Block::default()
                .title(format!(
                    " {} v{} - {}",
                    fl!("oma"),
                    env!("CARGO_PKG_VERSION"),
                    fl!("packages-size-analyzer"),
                ))
                .style(if self.remove_package.items.is_empty() {
                    Style::default().bg(Color::White).fg(Color::Black)
                } else {
                    Style::default().bg(Color::Yellow).fg(Color::Black)
                }),
            main_layout[0],
        );

        let chunks = Layout::default()
            .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
            .direction(Direction::Horizontal)
            .split(main_layout[1]);

        let area = if self.remove_package.items.is_empty() {
            main_layout[1]
        } else {
            chunks[0]
        };

        f.render_stateful_widget(
            List::new(self.installed.items.iter().map(|p| {
                p.to_installed_line(
                    self.total_installed_size,
                    self.remove_package.items.contains(p),
                )
            }))
            .block(
                Block::default()
                    .title(fl!(
                        "psa-installed-window",
                        count = self.installed.items.len(),
                        size = oma_utils::human_bytes::HumanBytes(self.total_installed_size)
                            .to_string()
                    ))
                    .borders(Borders::ALL)
                    .style(highlight_window(self.window, Window::Installed)),
            )
            .highlight_style(self.bg_render_mode.to_style()),
            area,
            &mut self.installed.state,
        );

        f.render_stateful_widget(
            Scrollbar::new(ScrollbarOrientation::VerticalRight)
                .begin_symbol(Some("↑"))
                .end_symbol(Some("↓")),
            area,
            &mut self.installed_scroll_state,
        );

        if !self.remove_package.items.is_empty() {
            f.render_stateful_widget(
                List::new(
                    self.remove_package
                        .items
                        .iter()
                        .map(|p| p.to_remove_line(chunks[1].width - 2, self.bg_render_mode)),
                )
                .highlight_style(self.bg_render_mode.to_style())
                .block(
                    Block::new()
                        .title(fl!(
                            "psa-remove-window",
                            count = self.remove_package.items.len(),
                            size = oma_utils::human_bytes::HumanBytes(
                                self.remove_package
                                    .items
                                    .iter()
                                    .map(|p| p.pkg.installed().unwrap().installed_size())
                                    .sum::<u64>()
                            )
                            .to_string()
                        ))
                        .borders(Borders::ALL)
                        .style(highlight_window(self.window, Window::RemovePending)),
                ),
                chunks[1],
                &mut self.remove_package.state,
            );
        }

        render_tips(f, &main_layout);

        if let Some(popup) = &self.popup {
            let block = Block::bordered();
            let area = popup_area(
                main_layout[1],
                console::measure_text_width(popup) as u16 + 10,
                6,
            );
            let inner = block.inner(area);
            f.render_widget(Clear, area); //this clears out the background
            f.render_widget(block, area);
            f.render_widget(
                Text::from(vec![
                    Line::raw(popup),
                    Line::raw(""),
                    Line::raw(fl!("tui-continue-tips")),
                ]),
                inner,
            );
        }
    }
}

fn render_tips(f: &mut Frame<'_>, main_layout: &Rc<[Rect]>) {
    match WRITER.get_length() {
        0..=62 => {}
        63..=153 => {
            f.render_widget(
                Paragraph::new(Line::from(vec![
                    Span::raw("Quicknav: "),
                    Span::styled("TAB", Style::new().blue()),
                    Span::raw(" / "),
                    Span::styled("ESC", Style::new().blue()),
                    Span::raw(" / "),
                    Span::styled("Ctrl+A", Style::new().blue()),
                    Span::raw(" / "),
                    Span::styled("Ctrl+C", Style::new().blue()),
                ])),
                main_layout[2],
            );
        }
        154.. => {
            f.render_widget(
                Paragraph::new(Line::from(vec![
                    Span::styled("TAB", Style::new().blue()),
                    Span::raw(format!(" => {}, ", fl!("tui-start-2"))),
                    Span::styled("ESC", Style::new().blue()),
                    Span::raw(format!(" => {}, ", fl!("tui-start-4"))),
                    Span::styled("Ctrl+A", Style::new().blue()),
                    Span::raw(format!(" => {}, ", fl!("psa-autoremove"))),
                    Span::styled("Ctrl+C", Style::new().blue()),
                    Span::raw(format!(" => {}", fl!("tui-start-7"))),
                ])),
                main_layout[2],
            );
        }
    }
}

fn highlight_window(current: Window, needs: Window) -> Style {
    if current == needs {
        Style::default()
    } else {
        Style::default().fg(Color::DarkGray)
    }
}

/// helper function to create a centered rect using up certain percentage of the available rect `r`
fn popup_area(area: Rect, x: u16, y: u16) -> Rect {
    let vertical = Layout::vertical([Constraint::Length(y)]).flex(Flex::Center);
    let horizontal = Layout::horizontal([Constraint::Length(x)]).flex(Flex::Center);
    let [area] = vertical.areas(area);
    let [area] = horizontal.areas(area);

    area
}
