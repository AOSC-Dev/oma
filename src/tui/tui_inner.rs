use std::{
    fmt::Display,
    io,
    ops::ControlFlow,
    rc::Rc,
    time::{Duration, Instant},
};

use super::state::StatefulList;
use ansi_to_tui::IntoText;
use crossterm::event::{self, KeyCode, KeyModifiers};
use dialoguer::console;
use oma_pm::{
    apt::OmaApt,
    pkginfo::OmaPackage,
    search::{IndiciumSearch, OmaSearch, SearchResult},
};

use ratatui::{
    layout::{Alignment, Constraint, Direction, Flex, Layout, Rect},
    prelude::Backend,
    style::{Color, Style, Stylize},
    text::{Line, Span, Text},
    widgets::{
        Block, Borders, Clear, List, ListItem, Padding, Paragraph, Scrollbar, ScrollbarOrientation,
        ScrollbarState,
    },
    Frame, Terminal,
};

use crate::{fl, utils::SearchResultDisplay, WRITER};

#[derive(PartialEq, Eq)]
enum Mode {
    Search,
    Packages,
    Pending,
}

#[derive(Clone, PartialEq, Eq)]
enum Operation {
    Package {
        name: String,
        version: Option<String>,
    },
    Upgrade,
    AutoRemove,
}

pub struct Tui<'a> {
    apt: &'a OmaApt,
    searcher: IndiciumSearch<'a>,
    mode: Mode,
    input_cursor_position: usize,
    display_pending_detail: bool,
    input: String,
    upgradable: usize,
    autoremovable: usize,
    installed: usize,
    pkg_results: Vec<SearchResult>,
    pkg_result_state: StatefulList<Text<'static>>,
    pending_result_state: StatefulList<Operation>,
    install: Vec<OmaPackage>,
    remove: Vec<OmaPackage>,
    result_scroll: ScrollbarState,
    upgrade: bool,
    autoremove: bool,
    popup: Option<String>,
}

impl Display for Operation {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Operation::Package { name, version } => {
                if let Some(ref ver) = version {
                    writeln!(f, "+ {} ({})", name, ver)?;
                } else {
                    writeln!(f, "- {}", name)?;
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

impl<'a> Tui<'a> {
    pub fn new(
        apt: &'a OmaApt,
        installed: usize,
        upgradable: usize,
        autoremovable: usize,
        searcher: IndiciumSearch<'a>,
    ) -> Self {
        let pkg_results = vec![];
        let pkg_result_state = StatefulList::with_items(vec![]);

        Self {
            apt,
            searcher,
            mode: Mode::Search,
            input_cursor_position: 0,
            display_pending_detail: false,
            input: String::new(),
            upgradable,
            autoremovable,
            installed,
            pkg_result_state,
            pending_result_state: StatefulList::with_items(vec![]),
            pkg_results,
            install: vec![],
            remove: vec![],
            result_scroll: ScrollbarState::new(0),
            upgrade: false,
            autoremove: false,
            popup: None,
        }
    }

    pub fn run<B: Backend>(
        mut self,
        terminal: &mut Terminal<B>,
        tick_rate: Duration,
    ) -> io::Result<Task> {
        let mut last_tick = Instant::now();
        loop {
            terminal.draw(|f| self.ui(f))?;

            if event::poll(tick_rate)? {
                if let event::Event::Key(key) = event::read()? {
                    if self.popup.is_some() {
                        match key.code {
                            KeyCode::Char('c') => {
                                self.popup = None;
                                continue;
                            }
                            _ => continue,
                        }
                    }

                    if key.modifiers == KeyModifiers::CONTROL {
                        match key.code {
                            KeyCode::Char('c') => {
                                return Ok(Task {
                                    execute_apt: false,
                                    install: vec![],
                                    remove: vec![],
                                    upgrade: false,
                                    autoremove: false,
                                });
                            }
                            KeyCode::Char('u') => {
                                if let Some(pos) = self
                                    .pending_result_state
                                    .items
                                    .iter()
                                    .position(|x| *x == Operation::Upgrade)
                                {
                                    self.display_pending_detail = true;
                                    self.upgrade = false;
                                    self.pending_result_state.items.remove(pos);
                                } else {
                                    if self.upgradable == 0 {
                                        self.popup = Some(fl!("tui-no-system-update"));
                                        continue;
                                    }
                                    self.display_pending_detail = true;
                                    self.upgrade = true;
                                    self.pending_result_state.items.push(Operation::Upgrade);
                                }
                            }
                            KeyCode::Char('a') => {
                                if let Some(pos) = self
                                    .pending_result_state
                                    .items
                                    .iter()
                                    .position(|x| *x == Operation::AutoRemove)
                                {
                                    self.autoremove = false;
                                    self.pending_result_state.items.remove(pos);
                                } else {
                                    if self.autoremovable == 0 {
                                        self.popup = Some(fl!("tui-no-package-clean-up"));
                                        continue;
                                    }
                                    self.display_pending_detail = true;
                                    self.autoremove = true;
                                    self.pending_result_state.items.push(Operation::AutoRemove);
                                }
                            }
                            _ => {}
                        }

                        continue;
                    }

                    match key.code {
                        KeyCode::Up => self.handle_up(),
                        KeyCode::Down => self.handle_down(),
                        KeyCode::Esc => break,
                        KeyCode::Char(' ') => {
                            if let ControlFlow::Break(_) = self.handle_space() {
                                continue;
                            }
                        }
                        KeyCode::Char('/') => self.mode = Mode::Search,
                        KeyCode::Char(c) => {
                            if self.mode != Mode::Search {
                                continue;
                            }

                            self.handle_input_text(c);
                        }
                        KeyCode::Tab => self.handle_tab(),
                        KeyCode::Backspace => {
                            if self.mode != Mode::Search {
                                continue;
                            }

                            if let ControlFlow::Break(_) = self.handle_input_backspace() {
                                continue;
                            }
                        }
                        KeyCode::Delete => {
                            if self.mode != Mode::Search {
                                continue;
                            }

                            if let ControlFlow::Break(_) = self.handle_input_delete() {
                                continue;
                            }
                        }
                        KeyCode::Left => self.handle_left(),
                        KeyCode::Right => self.handle_right(),
                        KeyCode::F(1) => self.display_pending_detail = !self.display_pending_detail,
                        _ => {}
                    }
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

    fn handle_right(&mut self) {
        match self.mode {
            Mode::Search => {
                if self.input_cursor_position < self.input.len() {
                    self.input_cursor_position += 1;
                }
            }
            Mode::Packages => {
                change_to_pending_window(&mut self.mode, &mut self.pending_result_state);
            }
            Mode::Pending => {
                change_to_packages_window(&mut self.mode, &mut self.pkg_result_state);
            }
        }
    }

    fn handle_left(&mut self) {
        match self.mode {
            Mode::Search => {
                self.input_cursor_position = self.input_cursor_position.saturating_sub(1);
            }
            Mode::Packages => {
                change_to_pending_window(&mut self.mode, &mut self.pending_result_state);
            }
            Mode::Pending => {
                change_to_packages_window(&mut self.mode, &mut self.pkg_result_state);
            }
        }
    }

    fn handle_input_delete(&mut self) -> ControlFlow<()> {
        if self.input_cursor_position > self.input.len() - 1 {
            return ControlFlow::Break(());
        }

        delete_inner(
            &mut self.input,
            self.input_cursor_position,
            self.input_cursor_position + 1,
        );

        update_search_result(
            &self.searcher,
            &self.input,
            &mut self.pkg_result_state,
            &mut self.pkg_results,
        );

        self.result_scroll = self
            .result_scroll
            .content_length(self.pkg_result_state.items.len());

        ControlFlow::Continue(())
    }

    fn handle_input_backspace(&mut self) -> ControlFlow<()> {
        if self.input_cursor_position == 0 {
            self.pkg_results = vec![];
            return ControlFlow::Break(());
        }

        let from_left_to_current_index = self.input_cursor_position - 1;

        delete_inner(
            &mut self.input,
            from_left_to_current_index,
            self.input_cursor_position,
        );

        self.input_cursor_position = self.input_cursor_position.saturating_sub(1);

        update_search_result(
            &self.searcher,
            &self.input,
            &mut self.pkg_result_state,
            &mut self.pkg_results,
        );

        self.result_scroll = self
            .result_scroll
            .content_length(self.pkg_result_state.items.len());

        ControlFlow::Continue(())
    }

    fn handle_tab(&mut self) {
        if self.display_pending_detail {
            self.mode = match self.mode {
                Mode::Search => Mode::Packages,
                Mode::Packages => Mode::Pending,
                Mode::Pending => Mode::Search,
            };
        } else {
            self.mode = match self.mode {
                Mode::Search => Mode::Packages,
                Mode::Packages => Mode::Search,
                Mode::Pending => Mode::Search,
            };
        }

        match self.mode {
            Mode::Search => {}
            Mode::Packages => {
                change_to_packages_window(&mut self.mode, &mut self.pkg_result_state);
            }
            Mode::Pending => {
                change_to_pending_window(&mut self.mode, &mut self.pending_result_state);
            }
        }
    }

    fn handle_input_text(&mut self, c: char) {
        let byte_index = self
            .input
            .char_indices()
            .map(|(i, _)| i)
            .nth(self.input_cursor_position)
            .unwrap_or(self.input.len());

        self.input.insert(byte_index, c);
        self.input_cursor_position = self.input_cursor_position.saturating_add(1);

        let s = &self.input;
        let res = self.searcher.search(s);

        if let Ok(res) = res {
            let res_display = res
                .iter()
                .filter_map(|x| SearchResultDisplay(x).to_string().into_text().ok())
                .collect::<Vec<_>>();
            self.pkg_result_state = StatefulList::with_items(res_display);
            self.pkg_results = res;

            self.result_scroll = self
                .result_scroll
                .content_length(self.pkg_result_state.items.len());
        } else {
            self.pkg_result_state = StatefulList::with_items(vec![]);
            self.pkg_results.clear();
        }
    }

    fn handle_space(&mut self) -> ControlFlow<()> {
        match self.mode {
            Mode::Search => {}
            Mode::Packages => {
                let selected = self.pkg_result_state.state.selected();
                if let Some(i) = selected {
                    self.display_pending_detail = true;
                    let name = &self.pkg_results[i].name;
                    if let Some(pkg) = self.apt.cache.get(name) {
                        if let Some(pkg_index) = self
                            .install
                            .iter()
                            .position(|x: &OmaPackage| x.raw_pkg.fullname(true) == *name)
                        {
                            let pos = self
                                .pending_result_state
                                .items
                                .iter()
                                .position(|x| {
                                    if let Operation::Package { name, version } = x {
                                        *name == self.install[pkg_index].raw_pkg.fullname(true)
                                            && version.is_some()
                                    } else {
                                        false
                                    }
                                })
                                .unwrap();

                            self.pending_result_state.items.remove(pos);
                            self.pending_result_state.state.select(None);

                            self.install.remove(pkg_index);

                            return ControlFlow::Break(());
                        }

                        if let Some(pkg_index) = self
                            .remove
                            .iter()
                            .position(|x: &OmaPackage| x.raw_pkg.fullname(true) == *name)
                        {
                            let pos = self
                                .pending_result_state
                                .items
                                .iter()
                                .position(|x| {
                                    if let Operation::Package { name, version } = x {
                                        *name == self.remove[pkg_index].raw_pkg.fullname(true)
                                            && version.is_some()
                                    } else {
                                        false
                                    }
                                })
                                .unwrap();

                            self.pending_result_state.items.remove(pos);
                            self.pending_result_state.state.select(None);

                            self.remove.remove(pkg_index);

                            return ControlFlow::Break(());
                        }

                        let cand = pkg.candidate().or(pkg.versions().next());

                        if let Some(cand) = cand {
                            let pkginfo = OmaPackage::new(&cand, &pkg);
                            if !cand.is_installed() {
                                self.install.push(pkginfo.unwrap());
                                let op = Operation::Package {
                                    name: pkg.fullname(true),
                                    version: Some(cand.version().to_string()),
                                };

                                self.pending_result_state.items.push(op);
                            } else {
                                let op = Operation::Package {
                                    name: pkg.fullname(true),
                                    version: None,
                                };
                                self.remove.push(pkginfo.unwrap());
                                self.pending_result_state.items.push(op);
                            }
                        }
                    }
                }
            }
            Mode::Pending => {
                let selected = self.pending_result_state.state.selected();
                if let Some(i) = selected {
                    let removed = self.pending_result_state.items.remove(i);

                    match removed {
                        Operation::Package { name, version } => {
                            if version.is_some() {
                                let inst_pos = self
                                    .install
                                    .iter()
                                    .position(|x| x.raw_pkg.fullname(true) == name)
                                    .unwrap();
                                self.install.remove(inst_pos);
                            } else {
                                let remove_pos = self
                                    .remove
                                    .iter()
                                    .position(|x| x.raw_pkg.fullname(true) == name)
                                    .unwrap();
                                self.remove.remove(remove_pos);
                            }
                            if self.pending_result_state.items.is_empty() {
                                self.pending_result_state.state.select(None);
                            } else {
                                self.pending_result_state.previous();
                            }
                        }
                        Operation::Upgrade => self.upgrade = false,
                        Operation::AutoRemove => self.autoremove = false,
                    }
                }
            }
        }

        ControlFlow::Continue(())
    }

    fn handle_down(&mut self) {
        match self.mode {
            Mode::Search => {
                change_to_packages_window(&mut self.mode, &mut self.pkg_result_state);
            }
            Mode::Packages => {
                self.pkg_result_state.next();
                self.result_scroll = self
                    .result_scroll
                    .position(self.pkg_result_state.state.selected().unwrap_or(0));
            }
            Mode::Pending => {
                self.pending_result_state.next();
            }
        }
    }

    fn handle_up(&mut self) {
        match self.mode {
            Mode::Search => {}
            Mode::Packages => {
                if self
                    .pkg_result_state
                    .state
                    .selected()
                    .map(|x| x == 0)
                    .unwrap_or(true)
                {
                    self.mode = Mode::Search;
                } else {
                    self.pkg_result_state.previous();
                    self.result_scroll = self
                        .result_scroll
                        .position(self.pkg_result_state.state.selected().unwrap_or(0));
                }
            }
            Mode::Pending => {
                if self
                    .pending_result_state
                    .state
                    .selected()
                    .map(|x| x == 0)
                    .unwrap_or(true)
                {
                    self.mode = Mode::Search;
                } else {
                    self.pending_result_state.previous();
                }
            }
        }
    }

    fn ui(&mut self, f: &mut Frame) {
        let input = self.input.clone();
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
            (self.upgradable, self.autoremovable),
            self.installed,
        );

        if self.display_pending_detail {
            f.render_stateful_widget(
                List::new(self.pending_result_state.items.clone())
                    .block(
                        Block::default()
                            .borders(Borders::ALL)
                            .title(fl!("tui-pending"))
                            .style(highlight_window(&self.mode, &Mode::Pending)),
                    )
                    .highlight_style(Style::default().bg(Color::Rgb(59, 64, 70))),
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

        f.render_widget(
            Paragraph::new(input).style(Style::default()).block(
                Block::default()
                    .borders(Borders::ALL)
                    .title(fl!("tui-search"))
                    .style(highlight_window(&self.mode, &Mode::Search)),
            ),
            main_layout[1],
        );

        if self.mode == Mode::Search {
            f.set_cursor_position((
                // Draw the cursor at the current position in the input field.
                // This position is can be controlled via the left and right arrow key
                main_layout[1].x + self.input_cursor_position as u16 + 1,
                // Move one line down, from the border to the input line
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

fn render_tips(f: &mut Frame<'_>, main_layout: &Rc<[Rect]>) {
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
    upgradable_and_autoremovable: (usize, usize),
    installed: usize,
) {
    if !result.is_empty() {
        frame.render_stateful_widget(
            List::new(display_list.items.clone())
                .block(
                    Block::default()
                        .borders(Borders::ALL)
                        .title(fl!(
                            "tui-packages",
                            u = upgradable_and_autoremovable.0,
                            r = upgradable_and_autoremovable.1,
                            i = installed
                        ))
                        .style(highlight_window(mode, &Mode::Packages)),
                )
                .highlight_style(Style::default().bg(Color::Rgb(59, 64, 70))),
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
                    u = upgradable_and_autoremovable.0,
                    r = upgradable_and_autoremovable.1,
                    i = installed
                )),
            ])
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .style(highlight_window(mode, &Mode::Packages))
                    .padding(Padding::new(0, 0, (area.height / 2).saturating_sub(8), 0)),
            )
            .alignment(Alignment::Center),
            area,
        );
    }
}

fn highlight_window(mode: &Mode, right: &Mode) -> Style {
    if mode == right {
        Style::default().bold()
    } else {
        Style::default()
    }
}

fn change_to_packages_window(mode: &mut Mode, display_list: &mut StatefulList<Text<'static>>) {
    *mode = Mode::Packages;
    if display_list.state.selected().is_none() && !display_list.items.is_empty() {
        display_list.state.select(Some(0));
    }
}

fn change_to_pending_window(mode: &mut Mode, pending_display_list: &mut StatefulList<Operation>) {
    *mode = Mode::Pending;
    if pending_display_list.state.selected().is_none() && !pending_display_list.items.is_empty() {
        pending_display_list.state.select(Some(0));
    }
}

fn update_search_result(
    searcher: &IndiciumSearch<'_>,
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

fn delete_inner(input: &mut String, before: usize, after: usize) {
    // Method "remove" is not used on the saved text for deleting the selected char.
    // Reason: Using remove on String works on bytes instead of the chars.
    // Using remove would require special care because of char boundaries.
    // Getting all characters before the selected character.
    let before_char_to_delete = input.chars().take(before).collect::<String>();

    // Getting all characters after selected character.
    let after_char_to_delete = input.chars().skip(after).collect::<String>();

    *input = before_char_to_delete + &after_char_to_delete;
}

/// helper function to create a centered rect using up certain percentage of the available rect `r`
fn popup_area(area: Rect, x: u16, y: u16) -> Rect {
    let vertical = Layout::vertical([Constraint::Length(y)]).flex(Flex::Center);
    let horizontal = Layout::horizontal([Constraint::Length(x)]).flex(Flex::Center);
    let [area] = vertical.areas(area);
    let [area] = horizontal.areas(area);

    area
}
