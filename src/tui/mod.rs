mod state;

use std::{
    cell::{Ref, RefCell},
    ops::ControlFlow,
    rc::Rc,
    time::{Duration, Instant},
};

use ansi_to_tui::IntoText;
use crossterm::event::{self, KeyCode, KeyModifiers};
use oma_console::{
    indicatif::ProgressBar,
    pager::{exit_tui, prepare_create_tui},
    pb::spinner_style,
    WRITER,
};
use oma_history::SummaryType;
use oma_pm::{
    apt::{AptArgs, AptConfig, OmaApt, OmaAptArgs},
    pkginfo::PkgInfo,
    search::{IndiciumSearch, OmaSearch, SearchResult},
};
use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    prelude::Backend,
    style::{Color, Style, Stylize},
    text::{Line, Span, Text},
    widgets::{Block, Borders, List, Padding, Paragraph},
    Frame, Terminal,
};
use reqwest::Client;
use rustix::io;
use state::StatefulList;

use crate::{
    error::OutputError,
    fl,
    subcommand::utils::{lock_oma, no_check_dbus_warn, CommitRequest, RefreshRequest},
    utils::{create_async_runtime, dbus_check, root, SearchResultDisplay},
};

#[derive(PartialEq, Eq)]
enum Mode {
    Search,
    Packages,
    Pending,
}

struct Operation {
    name: String,
    is_install: bool,
}

pub struct Tui<'a> {
    apt: &'a OmaApt,
    searcher: IndiciumSearch<'a>,
    mode: Mode,
    input_cursor_position: usize,
    display_pending_detail: bool,
    input: Rc<RefCell<String>>,
    action: (usize, usize),
    installed: usize,
    pkg_results: Rc<RefCell<Vec<SearchResult>>>,
    pkg_result_state: StatefulList<Text<'static>>,
    pending_result_state: StatefulList<Text<'static>>,
    install: Vec<PkgInfo>,
    remove: Vec<PkgInfo>,
    pending: Vec<Operation>,
}

impl<'a> Tui<'a> {
    pub fn new(
        apt: &'a OmaApt,
        action: (usize, usize),
        installed: usize,
        searcher: IndiciumSearch<'a>,
    ) -> Self {
        Self {
            apt,
            searcher,
            mode: Mode::Search,
            input_cursor_position: 0,
            display_pending_detail: false,
            input: Rc::new(RefCell::new("".to_string())),
            action,
            installed,
            pkg_result_state: StatefulList::with_items(vec![]),
            pending_result_state: StatefulList::with_items(vec![]),
            pkg_results: Rc::new(RefCell::new(vec![])),
            install: vec![],
            remove: vec![],
            pending: vec![],
        }
    }

    pub fn run<B: Backend>(
        mut self,
        terminal: &mut Terminal<B>,
        tick_rate: Duration,
    ) -> io::Result<(bool, Vec<PkgInfo>, Vec<PkgInfo>)> {
        let mut last_tick = Instant::now();
        loop {
            terminal.draw(|f| self.ui(f)).unwrap();
            if event::poll(tick_rate).unwrap_or(false) {
                if let event::Event::Key(key) = event::read().unwrap() {
                    if key.modifiers == KeyModifiers::CONTROL && key.code == KeyCode::Char('c') {
                        return Ok((false, vec![], vec![]));
                    }
                    match key.code {
                        KeyCode::Up => {
                            self.handle_up();
                        }
                        KeyCode::Down => {
                            self.handle_down();
                        }
                        KeyCode::Esc => break,
                        KeyCode::Char(' ') => {
                            if let ControlFlow::Break(_) = self.handle_space() {
                                continue;
                            }
                        }
                        KeyCode::Char('/') => {
                            self.mode = Mode::Search;
                        }
                        KeyCode::Char(c) => {
                            if self.mode != Mode::Search {
                                continue;
                            }

                            self.handle_input_text(c);
                        }
                        KeyCode::Tab => {
                            self.handle_tab();
                        }
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
                        KeyCode::Left => {
                            self.handle_left();
                        }
                        KeyCode::Right => {
                            self.handle_right();
                        }
                        KeyCode::F(1) => {
                            self.display_pending_detail = !self.display_pending_detail;
                        }
                        _ => {}
                    }
                }
            }

            if last_tick.elapsed() >= tick_rate {
                last_tick = Instant::now();
            }
        }

        Ok((true, self.install, self.remove))
    }

    fn handle_right(&mut self) {
        match self.mode {
            Mode::Search => {
                if self.input_cursor_position < self.input.borrow().len() {
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
        if self.input_cursor_position > self.input.borrow().len() - 1 {
            return ControlFlow::Break(());
        }

        delete_inner(
            &self.input,
            self.input_cursor_position,
            self.input_cursor_position + 1,
        );

        let s = self.input.borrow();

        update_search_result(
            &self.searcher,
            s,
            &mut self.pkg_result_state,
            &self.pkg_results,
        );

        ControlFlow::Continue(())
    }

    fn handle_input_backspace(&mut self) -> ControlFlow<()> {
        if self.input_cursor_position == 0 {
            self.pkg_results.replace(vec![]);
            return ControlFlow::Break(());
        }

        let from_left_to_current_index = self.input_cursor_position - 1;

        delete_inner(
            &self.input,
            from_left_to_current_index,
            self.input_cursor_position,
        );

        self.input_cursor_position = self.input_cursor_position.saturating_sub(1);

        let s = self.input.borrow();

        update_search_result(
            &self.searcher,
            s,
            &mut self.pkg_result_state,
            &self.pkg_results,
        );

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
            .borrow()
            .char_indices()
            .map(|(i, _)| i)
            .nth(self.input_cursor_position)
            .unwrap_or(self.input.borrow().len());

        self.input.borrow_mut().insert(byte_index, c);
        self.input_cursor_position = self.input_cursor_position.saturating_add(1);

        let s = self.input.borrow();
        let res = self.searcher.search(&s);

        if let Ok(res) = res {
            let res_display = res
                .iter()
                .filter_map(|x| SearchResultDisplay(x).to_string().into_text().ok())
                .collect::<Vec<_>>();
            self.pkg_result_state = StatefulList::with_items(res_display);
            self.pkg_results.replace(res);
        } else {
            self.pkg_result_state = StatefulList::with_items(vec![]);
            self.pkg_results.borrow_mut().clear();
        }
    }

    fn handle_space(&mut self) -> ControlFlow<()> {
        match self.mode {
            Mode::Search => {}
            Mode::Packages => {
                let selected = self.pkg_result_state.state.selected();
                if let Some(i) = selected {
                    self.display_pending_detail = true;
                    let name = &self.pkg_results.borrow()[i].name;
                    if let Some(pkg) = self.apt.cache.get(name) {
                        if let Some(pkg_index) = self
                            .install
                            .iter()
                            .position(|x: &PkgInfo| x.raw_pkg.name() == name)
                        {
                            let pos = self
                                .pending_result_state
                                .items
                                .iter()
                                .position(|x| {
                                    x.to_string().starts_with(&format!(
                                        "+ {}",
                                        self.install[pkg_index].raw_pkg.name()
                                    ))
                                })
                                .unwrap();

                            self.pending_result_state.items.remove(pos);
                            self.pending_result_state.state.select(None);

                            let pending_pos = self
                                .pending
                                .iter()
                                .position(|x: &Operation| x.name == *name)
                                .unwrap();
                            self.pending.remove(pending_pos);
                            self.install.remove(pkg_index);
                            return ControlFlow::Break(());
                        }

                        if let Some(pkg_index) = self
                            .remove
                            .iter()
                            .position(|x: &PkgInfo| x.raw_pkg.name() == name)
                        {
                            let pos = self
                                .pending_result_state
                                .items
                                .iter()
                                .position(|x| {
                                    x.to_string().starts_with(&format!(
                                        "- {}",
                                        self.remove[pkg_index].raw_pkg.name()
                                    ))
                                })
                                .unwrap();

                            self.pending_result_state.items.remove(pos);
                            self.pending_result_state.state.select(None);
                            self.remove.remove(pkg_index);
                            let pending_pos = self
                                .pending
                                .iter()
                                .position(|x: &Operation| x.name == *name)
                                .unwrap();
                            self.pending.remove(pending_pos);
                            return ControlFlow::Break(());
                        }

                        let cand = pkg.candidate().or(pkg.versions().next());

                        if let Some(cand) = cand {
                            let pkginfo = PkgInfo::new(&cand, &pkg);
                            if !cand.is_installed() {
                                self.install.push(pkginfo.unwrap());
                                self.pending_result_state.items.push(Text::raw(format!(
                                    "+ {} ({})",
                                    pkg.name(),
                                    cand.version()
                                )));

                                self.pending.push(Operation {
                                    name: pkg.name().to_string(),
                                    is_install: true,
                                });
                            } else {
                                self.remove.push(pkginfo.unwrap());
                                self.pending_result_state
                                    .items
                                    .push(Text::raw(format!("- {}", pkg.name())));

                                self.pending.push(Operation {
                                    name: pkg.fullname(true).to_string(),
                                    is_install: false,
                                });
                            }
                        }
                    }
                }
            }
            Mode::Pending => {
                let selected = self.pending_result_state.state.selected();
                if let Some(i) = selected {
                    self.pending_result_state.remove(i);
                    let removed = self.pending.remove(i);
                    if removed.is_install {
                        let inst_pos = self
                            .install
                            .iter()
                            .position(|x| x.raw_pkg.name() == removed.name)
                            .unwrap();
                        self.install.remove(inst_pos);
                    } else {
                        let remove_pos = self
                            .remove
                            .iter()
                            .position(|x| x.raw_pkg.name() == removed.name)
                            .unwrap();
                        self.remove.remove(remove_pos);
                    }
                    if self.pending_result_state.items.is_empty() {
                        self.pending_result_state.state.select(None);
                    } else {
                        self.pending_result_state.previous();
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
                .title(format!(" {} v{}", env!("CARGO_PKG_VERSION"), fl!("oma")))
                .style(Style::default().bg(Color::White).fg(Color::Black)),
            main_layout[0],
        );

        let chunks = Layout::default()
            .constraints([Constraint::Percentage(80), Constraint::Percentage(20)])
            .direction(Direction::Horizontal)
            .split(main_layout[2]);

        self.pkg_result_state = StatefulList::with_items(
            self.pkg_results
                .borrow()
                .clone()
                .into_iter()
                .filter_map(|x| SearchResultDisplay(&x).to_string().into_text().ok())
                .collect(),
        );

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
            self.action,
            self.installed,
        );

        f.render_widget(
            Paragraph::new(input.as_ref().to_owned().into_inner())
                .style(Style::default())
                .block(
                    Block::default()
                        .borders(Borders::ALL)
                        .title(fl!("tui-search"))
                        .style(hightlight_window(&self.mode, &Mode::Search)),
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

        render_tips(f, main_layout);
    }
}

fn render_tips(f: &mut Frame<'_>, main_layout: Rc<[Rect]>) {
    match WRITER.get_length() {
        0..=44 => {}
        45..=130 => {
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
                    Span::styled("Ctrl+C", Style::new().blue()),
                ])),
                main_layout[3],
            );
        }
        131.. => {
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
                    Span::styled("Ctrl+C", Style::new().blue()),
                    Span::raw(format!(" => {}", fl!("tui-start-7"))),
                ])),
                main_layout[3],
            );
        }
    }
}

fn show_packages(
    result_rc: &Rc<RefCell<Vec<SearchResult>>>,
    frame: &mut Frame<'_>,
    display_list: &mut StatefulList<Text<'_>>,
    mode: &Mode,
    area: Rect,
    action: (usize, usize),
    installed: usize,
) {
    if !result_rc.borrow().is_empty() {
        frame.render_stateful_widget(
            List::new(display_list.items.clone())
                .block(
                    Block::default()
                        .borders(Borders::ALL)
                        .title(fl!(
                            "tui-packages",
                            u = action.0,
                            r = action.1,
                            i = installed
                        ))
                        .style(hightlight_window(mode, &Mode::Packages)),
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
                    Span::styled("Ctrl+C", Style::new().blue()),
                    Span::raw(format!(" => {}", fl!("tui-start-7"))),
                ]),
                Line::from(""),
                Line::from(fl!(
                    "tui-packages",
                    u = action.0,
                    r = action.1,
                    i = installed
                )),
            ])
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title(fl!("tui-start"))
                    .style(hightlight_window(mode, &Mode::Packages))
                    .padding(Padding::new(0, 0, area.height / 2 - 8, 0)),
            )
            .alignment(Alignment::Center),
            area,
        );
    }
}

fn hightlight_window(mode: &Mode, right: &Mode) -> Style {
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

fn change_to_pending_window(
    mode: &mut Mode,
    pending_display_list: &mut StatefulList<Text<'static>>,
) {
    *mode = Mode::Pending;
    if pending_display_list.state.selected().is_none() && !pending_display_list.items.is_empty() {
        pending_display_list.state.select(Some(0));
    }
}

fn update_search_result(
    searcher: &IndiciumSearch<'_>,
    s: Ref<'_, String>,
    display_list: &mut StatefulList<Text<'_>>,
    result_rc: &Rc<RefCell<Vec<SearchResult>>>,
) {
    let res = searcher.search(&s);

    if let Ok(res) = res {
        let res_display = res
            .iter()
            .filter_map(|x| SearchResultDisplay(&x).to_string().into_text().ok())
            .collect::<Vec<_>>();

        *display_list = StatefulList::with_items(res_display);
        result_rc.replace(res);
    } else {
        *display_list = StatefulList::with_items(vec![]);
        result_rc.borrow_mut().clear();
    }
}

fn delete_inner(input: &Rc<RefCell<String>>, before: usize, after: usize) {
    // Method "remove" is not used on the saved text for deleting the selected char.
    // Reason: Using remove on String works on bytes instead of the chars.
    // Using remove would require special care because of char boundaries.
    // Getting all characters before the selected character.
    let before_char_to_delete = input.borrow().chars().take(before).collect::<String>();

    // Getting all characters after selected character.
    let after_char_to_delete = input.borrow().chars().skip(after).collect::<String>();

    *input.borrow_mut() = before_char_to_delete + &after_char_to_delete;
}

pub struct TuiArgs {
    pub sysroot: String,
    pub no_progress: bool,
    pub dry_run: bool,
    pub network_thread: usize,
    pub client: Client,
    pub no_check_dbus: bool,
}

pub fn execute(tui: TuiArgs) -> Result<i32, OutputError> {
    root()?;

    let TuiArgs {
        sysroot,
        no_progress,
        dry_run,
        network_thread,
        client,
        no_check_dbus,
    } = tui;

    let fds = if !no_check_dbus {
        let rt = create_async_runtime()?;
        Some(dbus_check(&rt, false)?)
    } else {
        no_check_dbus_warn();
        None
    };

    let apt_config = AptConfig::new();

    RefreshRequest {
        client: &client,
        dry_run,
        no_progress,
        limit: network_thread,
        sysroot: &sysroot,
        _refresh_topics: true,
        config: &apt_config,
    }
    .run()?;

    let oma_apt_args = OmaAptArgs::builder().sysroot(sysroot.clone()).build();

    let mut apt = OmaApt::new(vec![], oma_apt_args, false, apt_config)?;

    let (sty, inv) = spinner_style();
    let pb = ProgressBar::new_spinner().with_style(sty);
    pb.enable_steady_tick(inv);
    pb.set_message(fl!("reading-database"));

    let a = apt.available_action()?;
    let installed = apt.installed_packages()?;

    let searcher = IndiciumSearch::new(&apt.cache, |n| {
        pb.set_message(fl!("reading-database-with-count", count = n));
    })?;
    pb.finish_and_clear();

    let mut terminal = prepare_create_tui().unwrap();

    let tui = Tui::new(&apt, a, installed, searcher);
    let (execute_apt, install, remove) =
        tui.run(&mut terminal, Duration::from_millis(250)).unwrap();

    exit_tui(&mut terminal).unwrap();

    let mut code = 0;

    if execute_apt {
        lock_oma()?;
        apt.upgrade()?;
        apt.install(&install, false)?;
        apt.remove(&remove, false, false)?;

        let apt_args = AptArgs::builder().no_progress(no_progress).build();

        code = CommitRequest {
            apt,
            dry_run,
            typ: SummaryType::Changes,
            apt_args,
            no_fixbroken: false,
            network_thread,
            no_progress,
            sysroot,
            fix_dpkg_status: true,
            protect_essential: true,
            client: &client,
        }
        .run()?;
    }

    drop(fds);

    Ok(code)
}
