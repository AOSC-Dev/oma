use std::io::{self, stdout};
use std::path::PathBuf;
use std::rc::Rc;
use std::time::{Duration, Instant};

use anyhow::anyhow;
use clap::Args;
use crossterm::event::{self, KeyCode, KeyModifiers};
use dialoguer::console::{self, style};
use oma_console::indicatif::HumanBytes;
use oma_console::pager::{exit_tui, prepare_create_tui};
use oma_pm::Package;
use oma_pm::apt::{AptConfig, FilterMode, OmaApt, OmaAptArgs};
use oma_pm::pkginfo::OmaPackageWithoutVersion;
use ratatui::layout::{Flex, Rect};
use ratatui::prelude::Constraint;
use ratatui::style::{Color, Style, Stylize};
use ratatui::text::{Line, Span, Text};
use ratatui::widgets::{
    Block, Borders, Clear, List, Paragraph, Scrollbar, ScrollbarOrientation, ScrollbarState,
};
use ratatui::{
    Frame, Terminal,
    layout::{Direction, Layout},
    prelude::Backend,
};
use std::io::Write;
use tabled::builder::Builder;
use tabled::settings::{Alignment, Settings};
use tracing::info;

use crate::utils::{dbus_check, is_root};
use crate::{CliExecuter, config::Config, error::OutputError};
use crate::{WRITER, fl};

const FULL: &str = "█";
const SEVEN_EIGHTHS: &str = "▉";
const THREE_QUARTERS: &str = "▊";
const FIVE_EIGHTHS: &str = "▋";
const HALF: &str = "▌";
const THREE_EIGHTHS: &str = "▍";
const ONE_QUARTER: &str = "▎";
const ONE_EIGHTH: &str = "▏";
const BAR_BLOCK_LENGTH: usize = 19;

#[derive(Debug, Args)]
pub struct SizeAnalyzer {
    /// Only display packages size details
    #[arg(short, long)]
    details: bool,
    /// Set sysroot target directory
    #[arg(from_global)]
    sysroot: PathBuf,
    /// Run oma in "dry-run" mode. Useful for testing changes and operations without making changes to the system
    #[arg(from_global)]
    dry_run: bool,
    /// Run oma do not check dbus
    #[arg(from_global)]
    no_check_dbus: bool,
    /// Set apt options
    #[arg(from_global)]
    apt_options: Vec<String>,
    /// Resolve broken dependencies in the system
    #[arg(short, long)]
    fix_broken: bool,
    /// Do not fix dpkg broken status
    #[arg(short, long)]
    no_fix_dpkg_status: bool,
    /// Install package(s) without fsync(2)
    #[arg(long)]
    force_unsafe_io: bool,
    /// Ignore repository and package dependency issues
    #[arg(long)]
    force_yes: bool,
    /// Replace configuration file(s) in the system those shipped in the package(s) to be installed (invokes `dpkg --force-confnew`)
    #[arg(long)]
    force_confnew: bool,
    /// Do not auto remove unnecessary package(s)
    #[arg(long)]
    no_autoremove: bool,
    /// Remove package(s) also remove configuration file(s), like apt purge
    #[arg(long, visible_alias = "purge")]
    remove_config: bool,
    /// Setup download threads (default as 4)
    #[arg(from_global)]
    download_threads: Option<usize>,
}

impl CliExecuter for SizeAnalyzer {
    fn execute(self, config: &Config, no_progress: bool) -> Result<i32, OutputError> {
        let SizeAnalyzer {
            sysroot,
            dry_run,
            no_check_dbus,
            apt_options,
            fix_broken,
            no_fix_dpkg_status,
            force_unsafe_io,
            force_yes,
            force_confnew,
            no_autoremove,
            remove_config,
            download_threads,
            details,
        } = self;

        let detail = if !is_root() { false } else { !details };

        let mut apt = OmaApt::new(
            vec![],
            OmaAptArgs::builder()
                .another_apt_options(apt_options)
                .dpkg_force_unsafe_io(force_unsafe_io)
                .force_yes(force_yes)
                .dpkg_force_confnew(force_confnew)
                .build(),
            false,
            AptConfig::new(),
        )?;

        let mut exit_code = 0;

        if !detail {
            let installed = installed_packages(&apt, true)
                .unwrap()
                .into_iter()
                .map(|pkg| PkgWrapper { pkg })
                .collect::<Vec<_>>();

            let total_size = get_total_installed_size(&installed);

            let res = installed.iter().map(|pkg| pkg.to_table_line(total_size));

            let mut table = Builder::default();
            res.for_each(|r| table.push_record([r.0, r.1, r.2, r.3]));

            table.push_record([
                style(HumanBytes(total_size)).green().to_string(),
                "100%".to_string(),
                make_bar(1.0, BAR_BLOCK_LENGTH),
                fl!("psa-total"),
            ]);

            let table_settings = Settings::default().with(tabled::settings::style::Style::blank());

            writeln!(
                stdout(),
                "{}",
                table.build().with(table_settings).modify(
                    tabled::settings::object::Columns::new(..1),
                    Alignment::right()
                )
            )
            .ok();
            writeln!(stdout()).ok();
            info!("{}", fl!("psa-without-root-tips"));
        } else {
            let _fds = if !no_check_dbus && !config.no_check_dbus() && !dry_run {
                Some(dbus_check(false))
            } else {
                no_check_dbus_warn();
                None
            };

            let tui = PkgSizeAnalyzer::new(&apt);
            let mut terminal =
                prepare_create_tui().map_err(|e| anyhow!("Failed to create terminal: {e}"))?;

            let remove_pkgs = tui
                .run(&mut terminal, Duration::from_millis(250))
                .map_err(|e| anyhow!("{e}"))?;

            exit_tui(&mut terminal).map_err(|e| anyhow!("{e}"))?;

            if remove_pkgs.is_empty() {
                return Ok(0);
            }

            apt.remove(
                remove_pkgs
                    .into_iter()
                    .map(|p| OmaPackageWithoutVersion {
                        raw_pkg: unsafe { p.pkg.unique() },
                    })
                    .collect::<Vec<_>>(),
                remove_config,
                no_autoremove,
            )?;

            let auth_config = auth_config(&sysroot);
            let auth_config = auth_config.as_ref();

            exit_code = CommitChanges::builder()
                .apt(apt)
                .dry_run(dry_run)
                .no_fixbroken(!fix_broken)
                .no_progress(no_progress)
                .sysroot(sysroot.to_string_lossy().to_string())
                .protect_essential(config.protect_essentials())
                .yes(false)
                .remove_config(remove_config)
                .autoremove(!no_autoremove)
                .network_thread(download_threads.unwrap_or_else(|| config.network_thread()))
                .maybe_auth_config(auth_config)
                .fix_dpkg_status(!no_fix_dpkg_status)
                .build()
                .run()?;
        }

        Ok(exit_code)
    }
}

fn installed_packages(apt: &OmaApt, small_to_big: bool) -> Result<Vec<Package<'_>>, OutputError> {
    let mut installed_packages = apt
        .filter_pkgs(&[FilterMode::Installed])?
        .collect::<Vec<_>>();

    if small_to_big {
        installed_packages.sort_unstable_by(|a, b| {
            a.installed()
                .unwrap()
                .installed_size()
                .cmp(&b.installed().unwrap().installed_size())
        });
    } else {
        installed_packages.sort_unstable_by(|a, b| {
            b.installed()
                .unwrap()
                .installed_size()
                .cmp(&a.installed().unwrap().installed_size())
        });
    }

    Ok(installed_packages)
}

use ratatui::widgets::ListState;

use super::utils::{CommitChanges, auth_config, no_check_dbus_warn};

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
            Some(i) => Some(if i < self.items.len() - 1 { i + 1 } else { i }),
            None if self.items.is_empty() => None,
            None => Some(0),
        };
        self.state.select(i);
    }

    pub fn previous(&mut self) {
        let i = match self.state.selected() {
            Some(i) => Some(i.saturating_sub(1)),
            None if self.items.is_empty() => None,
            None => Some(0),
        };
        self.state.select(i);
    }
}

struct PkgSizeAnalyzer<'a> {
    apt: &'a OmaApt,
    remove_package: StatefulList<PkgWrapper<'a>>,
    installed: StatefulList<PkgWrapper<'a>>,
    window: Window,
    total_installed_size: u64,
    popup: Option<String>,
    installed_scroll_state: ScrollbarState,
    installed_scroll: usize,
}

#[derive(PartialEq, Eq, Clone, Copy)]
enum Window {
    Installed,
    RemovePending,
}

#[inline]
fn get_percent(size: u64, total: u64) -> f64 {
    (size as f64 / total as f64) * 100.0
}

#[derive(Clone, PartialEq, Eq)]
struct PkgWrapper<'a> {
    pkg: Package<'a>,
}

impl<'a> PkgWrapper<'a> {
    fn to_installed_line(&self, total_installed_size: u64, pending_to_delete: bool) -> Line<'a> {
        let ver = self.pkg.installed().unwrap();
        let size = ver.installed_size();

        let not_allow_delete = self.is_not_allow_delete();

        let hb_fmt = format_human_size(size);
        let percent_str = format!("{:.1}%", get_percent(size, total_installed_size));

        Line::from_iter(vec![
            Span::styled(hb_fmt, Style::new().green()),
            Span::raw(" "),
            Span::styled(
                format!(
                    "{}{}",
                    " ".repeat(6usize.saturating_sub(percent_str.len())),
                    percent_str
                ),
                Style::new().gray(),
            ),
            Span::raw(" "),
            Span::styled(
                make_bar(size as f64 / total_installed_size as f64, BAR_BLOCK_LENGTH).to_string(),
                Style::new().gray(),
            ),
            Span::styled(
                self.pkg.fullname(true),
                if pending_to_delete {
                    Style::new().yellow()
                } else {
                    Style::new().gray()
                },
            ),
        ])
        .style({
            if not_allow_delete {
                Style::new().on_red()
            } else {
                Style::new()
            }
        })
    }

    fn is_not_allow_delete(&self) -> bool {
        let ver = self.pkg.installed().unwrap();

        ver.get_record("X-AOSC-Features").is_some() || self.pkg.is_essential()
    }

    fn to_table_line(&self, total_installed_size: u64) -> (String, String, String, String) {
        let ver = self.pkg.installed().unwrap();
        let size = ver.installed_size();
        let not_allow_delete = self.is_not_allow_delete();

        let human_size = style(HumanBytes(size).to_string()).green().to_string();
        let percent = get_percent(size, total_installed_size);
        let percent_str = format!("{:.1}%", percent);
        let bar = make_bar(size as f64 / total_installed_size as f64, BAR_BLOCK_LENGTH);
        let mut name = self.pkg.fullname(true);

        if not_allow_delete {
            name = style(name).red().to_string();
        }

        (human_size, percent_str, bar, name)
    }

    fn to_remove_line(&self, area_width: u16) -> Line<'static> {
        let ver = self.pkg.installed().unwrap();
        let size = ver.installed_size();
        let human_size = HumanBytes(size).to_string();
        let pkg_name = self.pkg.fullname(true);
        let name_len = pkg_name.len();
        let human_size_len = human_size.len();

        Line::from_iter(vec![
            Span::styled(pkg_name, Style::new().yellow()),
            Span::raw(format!(
                "{:-space$}",
                "",
                space = (area_width as usize)
                    .saturating_sub(name_len)
                    .saturating_sub(human_size_len),
            )),
            Span::styled(HumanBytes(size).to_string(), Style::new().red()),
        ])
    }
}

fn format_human_size(size: u64) -> String {
    let hb = HumanBytes(size).to_string();
    let needs_size = 11usize.saturating_sub(hb.len());

    format!("{}{}", " ".repeat(needs_size), hb)
}

impl<'a> PkgSizeAnalyzer<'a> {
    const PAGE_LEN: usize = 10;

    fn new(apt: &'a OmaApt) -> Self {
        Self {
            apt,
            remove_package: StatefulList::with_items(vec![]),
            installed: StatefulList::with_items(vec![]),
            window: Window::Installed,
            total_installed_size: 0,
            popup: None,
            installed_scroll_state: ScrollbarState::new(0),
            installed_scroll: 0,
        }
    }

    fn run<B: Backend>(
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
            terminal.draw(|f| self.ui(f))?;

            if event::poll(tick_rate)? {
                let event::Event::Key(key) = event::read()? else {
                    continue;
                };

                if key.modifiers == KeyModifiers::CONTROL && key.code == KeyCode::Char('c') {
                    return Ok(vec![]);
                }

                if key.modifiers == KeyModifiers::CONTROL && key.code == KeyCode::Char('a') {
                    match self.apt.filter_pkgs(&[FilterMode::AutoRemovable]) {
                        Ok(pkgs) => {
                            let pkgs = pkgs.collect::<Vec<_>>();

                            if pkgs.is_empty() {
                                self.popup = Some(fl!("tui-no-package-clean-up"));
                            } else {
                                for p in pkgs {
                                    self.remove_package.items.push(PkgWrapper { pkg: p });
                                }
                            }
                        }
                        Err(e) => {
                            self.popup = Some(e.to_string());
                        }
                    }
                }

                match key.code {
                    KeyCode::Down => match self.window {
                        Window::Installed => self.installed.next(),
                        Window::RemovePending => self.remove_package.next(),
                    },
                    KeyCode::Up => match self.window {
                        Window::Installed => self.installed.previous(),
                        Window::RemovePending => self.remove_package.previous(),
                    },
                    KeyCode::PageDown if self.window == Window::Installed => {
                        let pgdn_scroll_select =
                            self.installed.state.selected().unwrap_or(0) + Self::PAGE_LEN;

                        if pgdn_scroll_select > self.installed.items.len() {
                            self.installed_scroll_state.last();
                            self.installed_scroll = self.installed.items.len() - 1;
                            continue;
                        }

                        self.installed_scroll += Self::PAGE_LEN;
                        self.installed.state.select(Some(self.installed_scroll));
                        self.installed_scroll_state =
                            self.installed_scroll_state.position(self.installed_scroll);
                    }
                    KeyCode::PageUp if self.window == Window::Installed => {
                        let pgdn_scroll_select = self
                            .installed
                            .state
                            .selected()
                            .unwrap_or(0)
                            .saturating_sub(Self::PAGE_LEN);

                        self.installed_scroll =
                            self.installed_scroll.saturating_sub(Self::PAGE_LEN);
                        self.installed.state.select(Some(self.installed_scroll));
                        self.installed_scroll_state =
                            self.installed_scroll_state.position(pgdn_scroll_select);
                    }
                    KeyCode::Char(' ') => match self.window {
                        Window::Installed => {
                            if let Some(selected) = self.installed.state.selected() {
                                let selected = &self.installed.items[selected];
                                if let Some(pos) = self
                                    .remove_package
                                    .items
                                    .iter()
                                    .position(|p| p.pkg.index() == selected.pkg.index())
                                {
                                    self.remove_package.items.remove(pos);
                                } else if !selected.is_not_allow_delete() {
                                    self.remove_package.items.push(selected.clone());
                                } else {
                                    self.popup = Some(format!(
                                        "Not allow delete package {}",
                                        selected.pkg.fullname(true)
                                    ))
                                }
                            }
                        }
                        Window::RemovePending => {
                            if let Some(selected) = self.remove_package.state.selected() {
                                self.remove_package.items.remove(selected);
                                if self.remove_package.items.is_empty() {
                                    self.remove_package.state.select(None);
                                    self.window = Window::Installed;
                                } else {
                                    self.remove_package.previous();
                                }
                            }
                        }
                    },
                    KeyCode::Left | KeyCode::Right | KeyCode::Tab => {
                        if self.remove_package.items.is_empty() {
                            continue;
                        }

                        self.window = match self.window {
                            Window::Installed => Window::RemovePending,
                            Window::RemovePending => Window::Installed,
                        }
                    }
                    KeyCode::Char('c') if self.popup.is_some() => {
                        self.popup = None;
                    }
                    KeyCode::Esc => {
                        return Ok(self.remove_package.items);
                    }
                    _ => {}
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
                        size = HumanBytes(self.total_installed_size).to_string()
                    ))
                    .borders(Borders::ALL)
                    .style(highlight_window(self.window, Window::Installed)),
            )
            .highlight_style(Style::default().bg(Color::Rgb(59, 64, 70))),
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
                        .map(|p| p.to_remove_line(chunks[1].width - 2)),
                )
                .highlight_style(Style::default().bg(Color::Rgb(59, 64, 70)))
                .block(
                    Block::new()
                        .title(fl!(
                            "psa-remove-window",
                            count = self.remove_package.items.len(),
                            size = HumanBytes(
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

fn get_total_installed_size(installed: &[PkgWrapper]) -> u64 {
    installed
        .iter()
        .map(|p| p.pkg.installed().unwrap().installed_size())
        .sum()
}

// From https://github.com/Byron/dua-cli/blob/main/src/interactive/app/bytevis.rs
fn make_bar(percentage: f64, length: usize) -> String {
    let mut s = String::new();
    // Print the filled part of the bar
    let block_length = (length as f64 * percentage).floor() as usize;
    for _ in 0..block_length {
        s.push_str(FULL);
    }

    // Bar is done if full length is already used, continue working if not
    if block_length < length {
        let block_sections = [
            " ",
            ONE_EIGHTH,
            ONE_QUARTER,
            THREE_EIGHTHS,
            HALF,
            FIVE_EIGHTHS,
            THREE_QUARTERS,
            SEVEN_EIGHTHS,
            FULL,
        ];
        // Get the index based on how filled the remaining part is
        let index = (((length as f64 * percentage) - block_length as f64) * 8f64).round() as usize;
        s.push_str(block_sections[index]);

        // Remainder of the bar should be empty
        for _ in 0..length - block_length - 1 {
            s.push(' ');
        }
    }

    s
}

/// helper function to create a centered rect using up certain percentage of the available rect `r`
fn popup_area(area: Rect, x: u16, y: u16) -> Rect {
    let vertical = Layout::vertical([Constraint::Length(y)]).flex(Flex::Center);
    let horizontal = Layout::horizontal([Constraint::Length(x)]).flex(Flex::Center);
    let [area] = vertical.areas(area);
    let [area] = horizontal.areas(area);

    area
}
