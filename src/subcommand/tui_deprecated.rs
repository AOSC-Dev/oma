use std::{cell::RefCell, io::stdout, rc::Rc};

use ansi_to_tui::IntoText;
use crossterm::{
    event::{self, KeyCode, KeyModifiers},
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
    ExecutableCommand,
};
use dialoguer::console::style;
use oma_console::{
    writer::{gen_prefix, writeln_inner, MessageType},
    WRITER,
};
use oma_history::SummaryType;
use reqwest::Client;

use crate::{
    fl,
    remove::ask_user_do_as_i_say,
    utils::{create_async_runtime, dbus_check},
};
use oma_pm::{
    apt::{AptArgsBuilder, OmaApt, OmaAptArgsBuilder},
    pkginfo::UnsafePkgInfo,
    search::{OmaSearch, SearchResult},
    PackageStatus,
};
use ratatui::{
    backend::CrosstermBackend,
    layout::{Alignment, Constraint, Direction, Layout},
    prelude::{Frame, Rect},
    style::{Color, Style, Stylize},
    text::{Line, Span, Text},
    widgets::{Block, Borders, List, ListState, Padding, Paragraph},
    Terminal,
};

use crate::{error::OutputError, utils::root};
use std::fmt::Display;

use super::utils::{lock_oma, no_check_dbus_warn, normal_commit, refresh, NormalCommitArgs};

const VERSION: &str = env!("CARGO_PKG_VERSION");

#[derive(Clone)]
pub struct FormatSearchResult(String);

impl From<&SearchResult> for FormatSearchResult {
    fn from(i: &SearchResult) -> Self {
        let mut pkg_info_line = if i.is_base {
            style(&i.name).bold().color256(141).to_string()
        } else {
            style(&i.name).bold().color256(148).to_string()
        };

        pkg_info_line.push(' ');

        if i.status == PackageStatus::Upgrade {
            pkg_info_line.push_str(&format!(
                "{} -> {}",
                style(i.old_version.as_ref().unwrap()).color256(214),
                style(&i.new_version).color256(114)
            ));
        } else {
            pkg_info_line.push_str(&style(&i.new_version).color256(114).to_string());
        }

        let mut pkg_tags = vec![];

        if i.dbg_package {
            pkg_tags.push(fl!("debug-symbol-available"));
        }

        if i.full_match {
            pkg_tags.push(fl!("full-match"))
        }

        if !pkg_tags.is_empty() {
            pkg_info_line.push(' ');
            pkg_info_line.push_str(
                &style(format!("[{}]", pkg_tags.join(",")))
                    .color256(178)
                    .to_string(),
            );
        }

        let prefix = match i.status {
            PackageStatus::Avail => style(fl!("pkg-search-avail")).dim(),
            PackageStatus::Installed => style(fl!("pkg-search-installed")).color256(72),
            PackageStatus::Upgrade => style(fl!("pkg-search-upgrade")).color256(214),
        }
        .to_string();

        let s = gen_prefix(&prefix, 10);

        let mut desc = "".to_string();

        writeln_inner(
            &i.desc,
            "",
            WRITER.get_max_len().into(),
            WRITER.get_prefix_len(),
            |t, s| match t {
                MessageType::Msg => desc.push_str(&format!("{}\n", style(s.trim()).color256(182))),
                MessageType::Prefix => desc.push_str(&gen_prefix(s, 10)),
            },
        );

        Self(format!("{s}{pkg_info_line}\n{}", desc))
    }
}

impl Display for FormatSearchResult {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

pub struct StatefulList<T> {
    pub state: ListState,
    pub items: Vec<T>,
}

impl<T> StatefulList<T> {
    pub fn with_items(items: Vec<T>) -> StatefulList<T> {
        StatefulList {
            state: ListState::default(),
            items,
        }
    }

    pub fn next(&mut self) {
        let i = match self.state.selected() {
            Some(i) => {
                if i < self.items.len() - 1 {
                    i + 1
                } else {
                    i
                }
            }
            None => 0,
        };
        self.state.select(Some(i));
    }

    pub fn previous(&mut self) {
        let i = match self.state.selected() {
            Some(i) => {
                if i == 0 {
                    0
                } else {
                    i - 1
                }
            }
            None => 0,
        };
        self.state.select(Some(i));
    }

    pub fn remove(&mut self, i: usize) {
        self.items.remove(i);

        if self.items.is_empty() {
            self.state.select(None);
        }
    }
}

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

pub fn execute(
    sysroot: String,
    no_progress: bool,
    dry_run: bool,
    network_thread: usize,
    client: Client,
    no_check_dbus: bool,
) -> Result<i32, OutputError> {
    root()?;

    let fds = if !no_check_dbus {
        let rt = create_async_runtime()?;
        dbus_check(&rt, false)?
    } else {
        no_check_dbus_warn();
        None
    };

    refresh(
        &client,
        dry_run,
        no_progress,
        network_thread,
        &sysroot,
    )?;

    stdout()
        .execute(EnterAlternateScreen)
        .map_err(|e| OutputError {
            description: "Failed to get stdout".to_string(),
            source: Some(Box::new(e)),
        })?;

    enable_raw_mode().map_err(|e| OutputError {
        description: "Failed to get terminal raw mode".to_string(),
        source: Some(Box::new(e)),
    })?;

    let mut terminal = Terminal::new(CrosstermBackend::new(stdout())).map_err(|e| OutputError {
        description: "Failed to create terminal".to_string(),
        source: Some(Box::new(e)),
    })?;

    terminal.clear().map_err(|e| OutputError {
        description: "Failed to clear terminal".to_string(),
        source: Some(Box::new(e)),
    })?;

    let oma_apt_args = OmaAptArgsBuilder::default()
        .sysroot(sysroot.clone())
        .build()?;

    let mut apt = OmaApt::new(vec![], oma_apt_args, false)?;

    let a = apt.available_action()?;
    let installed = apt.installed_packages()?;
    let searcher = OmaSearch::new(&apt.cache)?;

    let result_rc = Rc::new(RefCell::new(vec![]));
    let result_display = result_rc
        .borrow()
        .clone()
        .into_iter()
        .filter_map(|x| FormatSearchResult::from(&x).0.into_text().ok())
        .collect::<Vec<_>>();

    let input = Rc::new(RefCell::new("".to_string()));
    let mut display_list = StatefulList::with_items(result_display.clone());

    let mut cursor_position = 0;
    let mut display_pending_detail = false;

    let mut install = vec![];
    let mut remove = vec![];
    let pending_display_list: Vec<Text<'_>> = vec![];
    let mut pending_display_list = StatefulList::with_items(pending_display_list);

    let mut mode = Mode::Search;
    let mut execute_apt = true;
    let mut pending = vec![];

    loop {
        let input = input.clone();
        terminal
            .draw(|frame| {
                let main_layout = Layout::default()
                    .direction(Direction::Vertical)
                    .constraints([
                        Constraint::Length(1), // header
                        Constraint::Length(3), // search
                        Constraint::Min(0),    // packages
                        Constraint::Length(1), // tips
                    ])
                    .split(frame.size());

                frame.render_widget(
                    Block::default()
                        .title(format!(" {} v{VERSION}", fl!("oma")))
                        .style(Style::default().bg(Color::White).fg(Color::Black)),
                    main_layout[0],
                );

                let chunks = Layout::default()
                    .constraints([Constraint::Percentage(80), Constraint::Percentage(20)])
                    .direction(Direction::Horizontal)
                    .split(main_layout[2]);

                if display_pending_detail {
                    show_packages(
                        &result_rc,
                        frame,
                        &mut display_list,
                        &mode,
                        chunks[0],
                        a,
                        installed,
                    );

                    frame.render_stateful_widget(
                        List::new(pending_display_list.items.clone())
                            .block(
                                Block::default()
                                    .borders(Borders::ALL)
                                    .title(fl!("tui-pending"))
                                    .style(hightlight_window(&mode, &Mode::Pending)),
                            )
                            .highlight_style(Style::default().bg(Color::Rgb(59, 64, 70))),
                        chunks[1],
                        &mut pending_display_list.state,
                    );
                } else {
                    show_packages(
                        &result_rc,
                        frame,
                        &mut display_list,
                        &mode,
                        main_layout[2],
                        a,
                        installed,
                    );
                }

                frame.render_widget(
                    Paragraph::new(input.as_ref().to_owned().into_inner())
                        .style(Style::default())
                        .block(
                            Block::default()
                                .borders(Borders::ALL)
                                .title(fl!("tui-search"))
                                .style(hightlight_window(&mode, &Mode::Search)),
                        ),
                    main_layout[1],
                );

                if mode == Mode::Search {
                    frame.set_cursor(
                        // Draw the cursor at the current position in the input field.
                        // This position is can be controlled via the left and right arrow key
                        main_layout[1].x + cursor_position as u16 + 1,
                        // Move one line down, from the border to the input line
                        main_layout[1].y + 1,
                    );
                }

                let length = WRITER.get_length();

                match length {
                    0..=44 => {}
                    45..=130 => {
                        frame.render_widget(
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
                        frame.render_widget(
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
            })
            .map_err(|e| OutputError {
                description: "Failed to draw terminal".to_string(),
                source: Some(Box::new(e)),
            })?;

        if event::poll(std::time::Duration::from_millis(16)).unwrap_or(false) {
            if let event::Event::Key(key) = event::read().map_err(|e| OutputError {
                description: "Failed to read event key".to_string(),
                source: Some(Box::new(e)),
            })? {
                if key.modifiers == KeyModifiers::CONTROL && key.code == KeyCode::Char('c') {
                    execute_apt = false;
                    break;
                }
                match key.code {
                    KeyCode::Up => match mode {
                        Mode::Search => {}
                        Mode::Packages => {
                            if display_list
                                .state
                                .selected()
                                .map(|x| x == 0)
                                .unwrap_or(true)
                            {
                                mode = Mode::Search;
                            } else {
                                display_list.previous();
                            }
                        }
                        Mode::Pending => {
                            if pending_display_list
                                .state
                                .selected()
                                .map(|x| x == 0)
                                .unwrap_or(true)
                            {
                                mode = Mode::Search;
                            } else {
                                pending_display_list.previous();
                            }
                        }
                    },
                    KeyCode::Down => match mode {
                        Mode::Search => {
                            change_to_packages_window(&mut mode, &mut display_list);
                        }
                        Mode::Packages => {
                            display_list.next();
                        }
                        Mode::Pending => {
                            pending_display_list.next();
                        }
                    },
                    KeyCode::Esc => break,
                    KeyCode::Char(' ') => match mode {
                        Mode::Search => {}
                        Mode::Packages => {
                            let selected = display_list.state.selected();
                            if let Some(i) = selected {
                                display_pending_detail = true;
                                let name = &result_rc.borrow()[i].name;
                                if let Some(pkg) = apt.cache.get(name) {
                                    if let Some(pkg_index) = install
                                        .iter()
                                        .position(|x: &UnsafePkgInfo| x.raw_pkg.name() == name)
                                    {
                                        let pos = pending_display_list
                                            .items
                                            .iter()
                                            .position(|x| {
                                                x.to_string().starts_with(&format!(
                                                    "+ {}",
                                                    install[pkg_index].raw_pkg.name()
                                                ))
                                            })
                                            .unwrap();

                                        pending_display_list.items.remove(pos);
                                        pending_display_list.state.select(None);

                                        let pending_pos = pending
                                            .iter()
                                            .position(|x: &Operation| x.name == *name)
                                            .unwrap();
                                        pending.remove(pending_pos);
                                        install.remove(pkg_index);
                                        continue;
                                    }

                                    if let Some(pkg_index) = remove
                                        .iter()
                                        .position(|x: &UnsafePkgInfo| x.raw_pkg.name() == name)
                                    {
                                        let pos = pending_display_list
                                            .items
                                            .iter()
                                            .position(|x| {
                                                x.to_string().starts_with(&format!(
                                                    "- {}",
                                                    remove[pkg_index].raw_pkg.name()
                                                ))
                                            })
                                            .unwrap();

                                        pending_display_list.items.remove(pos);
                                        pending_display_list.state.select(None);
                                        remove.remove(pkg_index);
                                        let pending_pos = pending
                                            .iter()
                                            .position(|x: &Operation| x.name == *name)
                                            .unwrap();
                                        pending.remove(pending_pos);
                                        continue;
                                    }

                                    let cand = pkg.candidate().or(pkg.versions().next());

                                    if let Some(cand) = cand {
                                        let pkginfo = UnsafePkgInfo::new(&cand, &pkg);
                                        if !cand.is_installed() {
                                            install.push(pkginfo);
                                            pending_display_list.items.push(Text::raw(format!(
                                                "+ {} ({})",
                                                pkg.name(),
                                                cand.version()
                                            )));

                                            pending.push(Operation {
                                                name: pkg.name().to_string(),
                                                is_install: true,
                                            });
                                        } else {
                                            remove.push(pkginfo);
                                            pending_display_list
                                                .items
                                                .push(Text::raw(format!("- {}", pkg.name())));

                                            pending.push(Operation {
                                                name: pkg.name().to_string(),
                                                is_install: false,
                                            });
                                        }
                                    }
                                }
                            }
                        }
                        Mode::Pending => {
                            let selected = pending_display_list.state.selected();
                            if let Some(i) = selected {
                                pending_display_list.remove(i);
                                let removed = pending.remove(i);
                                if removed.is_install {
                                    let inst_pos = install
                                        .iter()
                                        .position(|x| x.raw_pkg.name() == removed.name)
                                        .unwrap();
                                    install.remove(inst_pos);
                                } else {
                                    let remove_pos = remove
                                        .iter()
                                        .position(|x| x.raw_pkg.name() == removed.name)
                                        .unwrap();
                                    remove.remove(remove_pos);
                                }
                                if pending_display_list.items.is_empty() {
                                    pending_display_list.state.select(None);
                                } else {
                                    pending_display_list.previous();
                                }
                            }
                        }
                    },
                    KeyCode::Char('/') => {
                        mode = Mode::Search;
                    }
                    KeyCode::Char(c) => {
                        if mode != Mode::Search {
                            continue;
                        }
                        input.borrow_mut().push(c);
                        let s = input.borrow();
                        let res = searcher.search(&s);

                        cursor_position += 1;

                        if let Ok(res) = res {
                            let res_display = res
                                .iter()
                                .filter_map(|x| FormatSearchResult::from(x).0.into_text().ok())
                                .collect::<Vec<_>>();
                            display_list = StatefulList::with_items(res_display);
                            result_rc.replace(res);
                        } else {
                            display_list = StatefulList::with_items(vec![]);
                            result_rc.borrow_mut().clear();
                        }
                    }
                    KeyCode::Tab => {
                        if display_pending_detail {
                            mode = match mode {
                                Mode::Search => Mode::Packages,
                                Mode::Packages => Mode::Pending,
                                Mode::Pending => Mode::Search,
                            };
                        } else {
                            mode = match mode {
                                Mode::Search => Mode::Packages,
                                Mode::Packages => Mode::Search,
                                Mode::Pending => Mode::Search,
                            };
                        }

                        match mode {
                            Mode::Search => {}
                            Mode::Packages => {
                                change_to_packages_window(&mut mode, &mut display_list);
                            }
                            Mode::Pending => {
                                change_to_pending_window(&mut mode, &mut pending_display_list);
                            }
                        }
                    }
                    KeyCode::Backspace => {
                        if mode != Mode::Search {
                            continue;
                        }
                        if cursor_position > 0 {
                            input.borrow_mut().pop();
                            let s = input.borrow();
                            let res = searcher.search(&s);

                            if let Ok(res) = res {
                                let res_display = res
                                    .iter()
                                    .filter_map(|x| FormatSearchResult::from(x).0.into_text().ok())
                                    .collect::<Vec<_>>();

                                display_list = StatefulList::with_items(res_display);
                                result_rc.replace(res);
                            } else {
                                display_list = StatefulList::with_items(vec![]);
                                result_rc.borrow_mut().clear();
                            }

                            cursor_position -= 1;
                        } else {
                            result_rc.replace(vec![]);
                        }
                    }
                    KeyCode::Left => match mode {
                        Mode::Search => {
                            if cursor_position > 0 {
                                cursor_position -= 1;
                            } else {
                                cursor_position = 0;
                            }
                        }
                        Mode::Packages => {
                            change_to_pending_window(&mut mode, &mut pending_display_list);
                        }
                        Mode::Pending => {
                            change_to_packages_window(&mut mode, &mut display_list);
                        }
                    },
                    KeyCode::Right => match mode {
                        Mode::Search => {
                            if cursor_position < input.borrow().len() {
                                cursor_position += 1;
                            }
                        }
                        Mode::Packages => {
                            change_to_pending_window(&mut mode, &mut pending_display_list);
                        }
                        Mode::Pending => {
                            change_to_packages_window(&mut mode, &mut display_list);
                        }
                    },
                    KeyCode::F(1) => {
                        display_pending_detail = !display_pending_detail;
                    }
                    _ => {}
                }
            }
        }
    }

    stdout()
        .execute(LeaveAlternateScreen)
        .map_err(|e| OutputError {
            description: "Failed to get stdout".to_string(),
            source: Some(Box::new(e)),
        })?;

    disable_raw_mode().map_err(|e| OutputError {
        description: "Failed to get terminal raw mode".to_string(),
        source: Some(Box::new(e)),
    })?;

    if execute_apt {
        lock_oma()?;
        apt.upgrade()?;
        apt.install(&install, false)?;
        apt.remove(&remove, false, false, |pkg| {
            ask_user_do_as_i_say(pkg).unwrap_or(false)
        })?;

        let apt_args = AptArgsBuilder::default().no_progress(no_progress).build()?;

        normal_commit(
            NormalCommitArgs {
                apt,
                dry_run,
                typ: SummaryType::Changes,
                apt_args,
                no_fixbroken: false,
                network_thread,
                no_progress,
                sysroot,
            },
            &client,
        )?;
    }

    drop(fds);

    Ok(0)
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
