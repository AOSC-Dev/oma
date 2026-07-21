use std::{
    fmt::Display,
    io,
    rc::Rc,
    time::{Duration, Instant},
};

use super::state::StatefulList;
use ansi_to_tui::IntoText;
use dialoguer::console;
use oma_pm::{apt::OmaApt, pkginfo::OmaPackage, search::SearchResult};
use ratatui::{
    crossterm::event::{self},
    style::Modifier,
};

use ratatui::{
    Frame, Terminal,
    layout::{Alignment, Constraint, Direction, Flex, Layout, Rect},
    prelude::Backend,
    style::{Color, Style},
    text::{Line, Span, Text},
    widgets::{
        Block, Borders, Clear, Gauge, List, ListItem, Padding, Paragraph, Scrollbar,
        ScrollbarOrientation, ScrollbarState,
    },
};
use spdlog::debug;
use terminfo::{Database, capability::MaxColors};

use crate::{
    WRITER, fl,
    subcommand::search::SearchResultDisplay,
    tui::{Searcher, key_binding::Control, refresh::DownloadItem, window::Mode},
};

#[derive(Clone, Copy)]
pub(crate) enum BgRenderMode {
    Color(Color),
    Reverse,
}

impl BgRenderMode {
    fn to_style(self) -> Style {
        match self {
            BgRenderMode::Color(color) => Style::default().bg(color),
            BgRenderMode::Reverse => Style::default()
                .bg(Color::Reset)
                .fg(Color::Reset)
                .add_modifier(Modifier::REVERSED),
        }
    }
}

pub struct Tui<'a> {
    pub(crate) apt: &'a OmaApt,
    pub(crate) searcher: Searcher,
    pub(crate) mode: Mode,
    pub(crate) display_pending_detail: bool,
    pub(crate) search_input: tui_input::Input,
    pub(crate) status: PackageStatus,
    pub(crate) pkg_results: Vec<SearchResult>,
    pub(crate) pkg_result_state: StatefulList<Text<'static>>,
    pub(crate) pending_result_state: StatefulList<Operation>,
    pub(crate) install: Vec<OmaPackage>,
    pub(crate) remove: Vec<OmaPackage>,
    pub(crate) result_scroll: ScrollbarState,
    pub(crate) upgrade: bool,
    pub(crate) autoremove: bool,
    pub(crate) popup: Option<String>,
    pub(crate) bg_render_mode: BgRenderMode,
}

#[derive(Clone, PartialEq, Eq)]
pub(crate) enum Operation {
    Package {
        name: String,
        version: Option<String>,
    },
    Upgrade,
    AutoRemove,
}

impl Display for Operation {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Operation::Package { name, version } => {
                if let Some(ver) = version {
                    writeln!(f, "+ {name} ({ver})")?;
                } else {
                    writeln!(f, "- {name}")?;
                }
            }
            Operation::Upgrade => writeln!(f, "{}", fl!("tui-upgrade"))?,
            Operation::AutoRemove => writeln!(f, "{}", fl!("tui-autoremove"))?,
        }

        Ok(())
    }
}

impl From<Operation> for ListItem<'_> {
    fn from(value: Operation) -> Self {
        Self::new(value.to_string())
    }
}

pub struct Task {
    pub execute_apt: bool,
    pub install: Vec<OmaPackage>,
    pub remove: Vec<OmaPackage>,
    pub upgrade: bool,
    pub autoremove: bool,
}

#[derive(Debug, Clone, Copy)]
pub(crate) struct PackageStatus {
    pub(crate) installed: usize,
    pub(crate) upgradable: usize,
    pub(crate) upgradable_but_held: usize,
    pub(crate) autoremove: usize,
}

impl PackageStatus {
    pub(crate) fn available_upgrade_package_count(&self) -> usize {
        self.upgradable - self.upgradable_but_held
    }
}

impl<'a> Tui<'a> {
    pub fn new(apt: &'a OmaApt, status: PackageStatus, searcher: Searcher) -> Self {
        let true_colors = Database::from_env()
            .inspect_err(|e| debug!("Failed to get terminfo: {e}"))
            .ok()
            .and_then(|terminfo| terminfo.get::<MaxColors>())
            .inspect(|MaxColors(n)| debug!("Terminal max colors: {n}"))
            .is_some_and(|MaxColors(n)| n >= 256);

        let no_color = std::env::var("NO_COLOR").is_ok_and(|s| s == "1" || s == "true");

        let bg_render_mode = if no_color {
            BgRenderMode::Reverse
        } else if true_colors {
            BgRenderMode::Color(Color::Rgb(59, 64, 70))
        } else {
            BgRenderMode::Color(Color::Blue)
        };

        let pkg_results = vec![];
        let pkg_result_state = StatefulList::with_items(vec![]);

        Self {
            apt,
            searcher,
            mode: Mode::Search,
            display_pending_detail: false,
            search_input: tui_input::Input::new(String::new()),
            status,
            pkg_result_state,
            pending_result_state: StatefulList::with_items(vec![]),
            pkg_results,
            install: vec![],
            remove: vec![],
            result_scroll: ScrollbarState::new(0),
            upgrade: false,
            autoremove: false,
            popup: None,
            bg_render_mode,
        }
    }

    /// Trigger a search with the current input value and update results.
    pub(crate) fn trigger_search(&mut self) {
        update_search_result(
            &self.searcher,
            self.search_input.value(),
            &mut self.pkg_result_state,
            &mut self.pkg_results,
        );
        self.result_scroll = self
            .result_scroll
            .content_length(self.pkg_result_state.items.len());
    }

    pub fn run<B: Backend>(
        mut self,
        terminal: &mut Terminal<B>,
        tick_rate: Duration,
    ) -> io::Result<Task> {
        let mut last_tick = Instant::now();
        loop {
            terminal
                .draw(|f| self.ui(f))
                .map_err(|e| io::Error::other(e.to_string()))?;

            if event::poll(tick_rate)?
                && let event::Event::Key(key) = event::read()?
            {
                match self.handle_key_binding(key) {
                    Control::Continue => continue,
                    Control::Task(task) => return Ok(task),
                    Control::Break => break,
                }
            }

            if last_tick.elapsed() >= tick_rate {
                last_tick = Instant::now();
            }
        }

        Ok(Task {
            execute_apt: !self.install.is_empty()
                || !self.remove.is_empty()
                || self.upgrade
                || self.autoremove,
            install: self.install,
            remove: self.remove,
            upgrade: self.upgrade,
            autoremove: self.autoremove,
        })
    }

    fn ui(&mut self, f: &mut Frame) {
        let main_layout = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(1), // header
                Constraint::Length(3), // search
                Constraint::Min(0),    // packages
                Constraint::Length(1), // tips
            ])
            .split(f.area());

        f.render_widget(
            Block::default()
                .title(format!(" {} v{}", fl!("oma"), env!("CARGO_PKG_VERSION")))
                .style(Style::default().bg(Color::White).fg(Color::Black)),
            main_layout[0],
        );

        let chunks = Layout::default()
            .constraints([Constraint::Percentage(80), Constraint::Percentage(20)])
            .direction(Direction::Horizontal)
            .split(main_layout[2]);

        show_packages(
            &self.pkg_results,
            f,
            &mut self.pkg_result_state,
            &self.mode,
            if self.display_pending_detail {
                chunks[0]
            } else {
                main_layout[2]
            },
            self.status,
            self.bg_render_mode,
        );

        if self.display_pending_detail {
            f.render_stateful_widget(
                List::new(self.pending_result_state.items.clone())
                    .block(
                        Block::default()
                            .borders(Borders::ALL)
                            .title(fl!("tui-pending"))
                            .style(self.mode.highlight_window(&Mode::Pending)),
                    )
                    .highlight_style(self.bg_render_mode.to_style()),
                chunks[1],
                &mut self.pending_result_state.state,
            );

            f.render_stateful_widget(
                Scrollbar::new(ScrollbarOrientation::VerticalRight)
                    .begin_symbol(Some("↑"))
                    .end_symbol(Some("↓")),
                chunks[0],
                &mut self.result_scroll,
            );
        } else {
            f.render_stateful_widget(
                Scrollbar::new(ScrollbarOrientation::VerticalRight)
                    .begin_symbol(Some("↑"))
                    .end_symbol(Some("↓")),
                main_layout[2],
                &mut self.result_scroll,
            );
        }

        let input_value = self.search_input.value();
        f.render_widget(
            Paragraph::new(input_value.to_string())
                .style(Style::default())
                .block(
                    Block::default()
                        .borders(Borders::ALL)
                        .title(fl!("tui-search"))
                        .style(self.mode.highlight_window(&Mode::Search)),
                ),
            main_layout[1],
        );

        if self.mode == Mode::Search {
            f.set_cursor_position((
                main_layout[1].x + self.search_input.visual_cursor() as u16 + 1,
                main_layout[1].y + 1,
            ));
        }

        render_tips(f, &main_layout);

        if let Some(popup) = &self.popup {
            let block = Block::bordered();
            let area = popup_area(
                main_layout[2],
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

pub(crate) fn render_tips(f: &mut Frame<'_>, main_layout: &Rc<[Rect]>) {
    match WRITER.get_length() {
        0..=62 => {}
        63..=153 => {
            f.render_widget(
                Paragraph::new(Line::from(vec![
                    Span::raw("Quicknav: "),
                    Span::styled("TAB", Style::new().blue()),
                    Span::raw(" / "),
                    Span::styled("F1", Style::new().blue()),
                    Span::raw(" / "),
                    Span::styled("ESC", Style::new().blue()),
                    Span::raw(" / "),
                    Span::styled("Space", Style::new().blue()),
                    Span::raw(" / "),
                    Span::styled("/", Style::new().blue()),
                    Span::raw(" / "),
                    Span::styled("Ctrl+U", Style::new().blue()),
                    Span::raw(" / "),
                    Span::styled("Ctrl+A", Style::new().blue()),
                    Span::raw(" / "),
                    Span::styled("Ctrl+C", Style::new().blue()),
                ])),
                main_layout[3],
            );
        }
        154.. => {
            f.render_widget(
                Paragraph::new(Line::from(vec![
                    Span::styled("TAB", Style::new().blue()),
                    Span::raw(format!(" => {}, ", fl!("tui-start-2"))),
                    Span::styled("F1", Style::new().blue()),
                    Span::raw(format!(" => {}, ", fl!("tui-start-3"))),
                    Span::styled("ESC", Style::new().blue()),
                    Span::raw(format!(" => {}, ", fl!("tui-start-4"))),
                    Span::styled("Space", Style::new().blue()),
                    Span::raw(format!(" => {}, ", fl!("tui-start-5"))),
                    Span::styled("/", Style::new().blue()),
                    Span::raw(format!(" => {}, ", fl!("tui-start-6"))),
                    Span::styled("Ctrl+U", Style::new().blue()),
                    Span::raw(format!(" => {}, ", fl!("tui-upgrade"))),
                    Span::styled("Ctrl+A", Style::new().blue()),
                    Span::raw(format!(" => {}, ", fl!("tui-autoremove"))),
                    Span::styled("Ctrl+C", Style::new().blue()),
                    Span::raw(format!(" => {}", fl!("tui-start-7"))),
                ])),
                main_layout[3],
            );
        }
    }
}

fn show_packages(
    result: &[SearchResult],
    frame: &mut Frame<'_>,
    display_list: &mut StatefulList<Text<'_>>,
    mode: &Mode,
    area: Rect,
    status: PackageStatus,
    bg_render_mode: BgRenderMode,
) {
    let u = status.available_upgrade_package_count();

    if !result.is_empty() {
        frame.render_stateful_widget(
            List::new(display_list.items.clone())
                .block(
                    Block::default()
                        .borders(Borders::ALL)
                        .title(fl!(
                            "tui-packages",
                            u = u,
                            r = status.autoremove,
                            i = status.installed
                        ))
                        .style(mode.highlight_window(&Mode::Packages)),
                )
                .highlight_style(bg_render_mode.to_style()),
            area,
            &mut display_list.state,
        );
    } else {
        frame.render_widget(
            Paragraph::new(vec![
                Line::from(fl!("tui-start-1")),
                Line::from(""),
                Line::from(vec![
                    Span::styled("TAB", Style::new().blue()),
                    Span::raw(format!(" => {}", fl!("tui-start-2"))),
                ]),
                Line::from(vec![
                    Span::styled("F1", Style::new().blue()),
                    Span::raw(format!(" => {}", fl!("tui-start-3"))),
                ]),
                Line::from(vec![
                    Span::styled("ESC", Style::new().blue()),
                    Span::raw(format!(" => {}", fl!("tui-start-4"))),
                ]),
                Line::from(vec![
                    Span::styled("Space", Style::new().blue()),
                    Span::raw(format!(" => {}", fl!("tui-start-5"))),
                ]),
                Line::from(vec![
                    Span::styled("/", Style::new().blue()),
                    Span::raw(format!(" => {}", fl!("tui-start-6"))),
                ]),
                Line::from(vec![
                    Span::styled("Ctrl+U", Style::new().blue()),
                    Span::raw(format!(" => {}", fl!("tui-upgrade"))),
                ]),
                Line::from(vec![
                    Span::styled("Ctrl+A", Style::new().blue()),
                    Span::raw(format!(" => {}", fl!("tui-autoremove"))),
                ]),
                Line::from(vec![
                    Span::styled("Ctrl+C", Style::new().blue()),
                    Span::raw(format!(" => {}", fl!("tui-start-7"))),
                ]),
                Line::from(""),
                Line::from(fl!(
                    "tui-packages",
                    u = u,
                    r = status.autoremove,
                    i = status.installed
                )),
            ])
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .style(mode.highlight_window(&Mode::Packages))
                    .padding(Padding::new(0, 0, (area.height / 2).saturating_sub(8), 0)),
            )
            .alignment(Alignment::Center),
            area,
        );
    }
}

pub(crate) fn update_search_result(
    searcher: &Searcher,
    s: &str,
    display_list: &mut StatefulList<Text<'_>>,
    result: &mut Vec<SearchResult>,
) {
    let res = searcher.search(s);

    if let Ok(res) = res {
        let res_display = res
            .iter()
            .filter_map(|x| SearchResultDisplay(x).to_string().into_text().ok())
            .collect::<Vec<_>>();

        *display_list = StatefulList::with_items(res_display);
        *result = res;
    } else {
        *display_list = StatefulList::with_items(vec![]);
        result.clear();
    }
}

/// helper function to create a centered rect using up certain percentage of the available rect `r`
pub(crate) fn popup_area(area: Rect, x: u16, y: u16) -> Rect {
    let vertical = Layout::vertical([Constraint::Length(y)]).flex(Flex::Center);
    let horizontal = Layout::horizontal([Constraint::Length(x)]).flex(Flex::Center);
    let [area] = vertical.areas(area);
    let [area] = horizontal.areas(area);

    area
}

fn centered_rect(area: Rect, percent_x: u16, percent_y: u16) -> Rect {
    let width = area.width.saturating_mul(percent_x) / 100;
    let height = area.height.saturating_mul(percent_y) / 100;
    let x = area.x + (area.width - width) / 2;
    let y = area.y + (area.height - height) / 2;
    Rect::new(x, y, width, height)
}

pub(crate) fn render_refresh_ui(f: &mut Frame, items: &[DownloadItem], status: &str) {
    let area = f.area();

    // ── Background: full TUI layout ──────────────────────────────────
    let layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1), // header
            Constraint::Length(3), // search box
            Constraint::Min(0),    // main content
            Constraint::Length(1), // tips
        ])
        .split(area);

    f.render_widget(
        Block::default()
            .title(format!(" {} v{}", fl!("oma"), env!("CARGO_PKG_VERSION")))
            .style(Style::default().bg(Color::White).fg(Color::Black)),
        layout[0],
    );

    f.render_widget(
        Paragraph::new(Line::raw("")).block(
            Block::default()
                .borders(Borders::ALL)
                .title(fl!("tui-search")),
        ),
        layout[1],
    );

    f.render_widget(Block::default().borders(Borders::ALL), layout[2]);
    render_tips(f, &layout);

    let dialog_area = centered_rect(area, 60, 60);
    f.render_widget(Clear, dialog_area);

    let dialog_block = Block::default()
        .borders(Borders::ALL)
        .title(fl!("refreshing-repo-metadata"));
    let inner = dialog_block.inner(dialog_area);
    f.render_widget(dialog_block, dialog_area);

    // Download progress inside dialog
    if !items.is_empty() {
        let gauge_w = 17u16;
        let content_area = inner;

        let rows = Layout::default()
            .direction(Direction::Vertical)
            .constraints(vec![Constraint::Length(1); items.len()])
            .split(content_area);

        for (i, item) in items.iter().enumerate() {
            if i >= rows.len() {
                break;
            }

            let pct = if item.size > 0 {
                (item.downloaded as f64 / item.size as f64 * 100.0).clamp(0.0, 100.0)
            } else {
                0.0
            };

            let cells = Layout::default()
                .direction(Direction::Horizontal)
                .constraints([Constraint::Min(0), Constraint::Length(gauge_w)])
                .split(rows[i]);

            let idx = format!("({}/{})", i + 1, item.total);
            let clean = item
                .msg
                .split_once(' ')
                .map_or(item.msg.as_str(), |(head, rest)| {
                    let prefix = head.split(':').next().unwrap_or("");
                    if rest.contains(prefix) {
                        head
                    } else {
                        item.msg.as_str()
                    }
                });
            let name_w = cells[0].width.saturating_sub(idx.len() as u16 + 2) as usize;
            let name: String = clean.chars().take(name_w).collect();

            f.render_widget(
                Paragraph::new(Line::raw(format!(" {idx} {name}"))),
                cells[0],
            );
            f.render_widget(
                Gauge::default()
                    .ratio(pct / 100.0)
                    .label(format!("{:3.0}%", pct))
                    .gauge_style(Style::default().fg(Color::Green).bg(Color::DarkGray)),
                cells[1],
            );
        }
    } else {
        // Show status text when no download items yet
        f.render_widget(Paragraph::new(Line::raw(status)), inner);
    }
}
