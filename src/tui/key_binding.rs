use std::ops::ControlFlow;

use oma_pm::pkginfo::OmaPackage;
use ratatui::crossterm::event::KeyModifiers;
use ratatui::crossterm::event::{KeyCode, KeyEvent};
use tui_input::InputRequest;

use crate::fl;
use crate::tui::render::Operation;
use crate::tui::render::Tui;
use crate::tui::render::{Task, update_search_result};
use crate::tui::window::Mode;

pub enum Control {
    Continue,
    Task(Task),
    Break,
}

impl Tui<'_> {
    fn trigger_search(&mut self) {
        let s = self.search_input.value();
        update_search_result(
            &self.searcher,
            s,
            &mut self.pkg_result_state,
            &mut self.pkg_results,
        );
        self.result_scroll = self
            .result_scroll
            .content_length(self.pkg_result_state.items.len());
    }

    pub fn handle_key_binding(&mut self, key: KeyEvent) -> Control {
        if self.popup.is_some() {
            match key.code {
                KeyCode::Char('c') => {
                    self.popup = None;
                    return Control::Continue;
                }
                _ => return Control::Continue,
            }
        }

        if key.modifiers == KeyModifiers::ALT {
            if let KeyCode::Char('d') = key.code {
                self.search_input.handle(InputRequest::DeleteNextWord);
                self.trigger_search();
            }
            return Control::Continue;
        }

        if key.modifiers == KeyModifiers::CONTROL {
            match key.code {
                KeyCode::Right => {
                    self.search_input.handle(InputRequest::GoToNextWord);
                }
                KeyCode::Left => {
                    self.search_input.handle(InputRequest::GoToPrevWord);
                }
                KeyCode::Char('p') => self.handle_up(false),
                KeyCode::Char('n') => self.handle_down(false),
                KeyCode::Char('w') => {
                    self.search_input.handle(InputRequest::DeletePrevWord);
                    self.trigger_search();
                }
                KeyCode::Char('c') => {
                    return Control::Task(Task {
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
                        if self.status.available_upgrade_package_count() == 0 {
                            self.popup = Some(fl!("tui-no-system-update"));
                            return Control::Continue;
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
                        if self.status.autoremove == 0 {
                            self.popup = Some(fl!("tui-no-package-clean-up"));
                            return Control::Continue;
                        }
                        self.display_pending_detail = true;
                        self.autoremove = true;
                        self.pending_result_state.items.push(Operation::AutoRemove);
                    }
                }
                _ => {}
            }

            return Control::Continue;
        }

        match key.code {
            KeyCode::Up => self.handle_up(false),
            KeyCode::Down => self.handle_down(false),
            KeyCode::Esc => return Control::Break,
            KeyCode::Char(' ') => {
                if let ControlFlow::Break(_) = self.handle_space() {
                    return Control::Continue;
                }
            }
            KeyCode::Char('/') => {
                self.mode = Mode::Search;
                self.trigger_search();
            }
            KeyCode::Tab => self.handle_tab(),
            KeyCode::F(1) => self.display_pending_detail = !self.display_pending_detail,
            KeyCode::PageDown => self.handle_down(true),
            KeyCode::PageUp => self.handle_up(true),
            KeyCode::Char(c) if self.mode == Mode::Search => {
                self.search_input.handle(InputRequest::InsertChar(c));
                self.trigger_search();
            }
            KeyCode::Backspace if self.mode == Mode::Search => {
                self.search_input.handle(InputRequest::DeletePrevChar);
                self.trigger_search();
            }
            KeyCode::Delete if self.mode == Mode::Search => {
                self.search_input.handle(InputRequest::DeleteNextChar);
                self.trigger_search();
            }
            KeyCode::Left => self.handle_left(),
            KeyCode::Right => self.handle_right(),
            _ => {}
        }

        Control::Continue
    }

    fn handle_left(&mut self) {
        match self.mode {
            Mode::Search => {
                self.search_input.handle(InputRequest::GoToPrevChar);
            }
            Mode::Packages => {
                self.mode
                    .change_to_pending_window(&mut self.pending_result_state);
            }
            Mode::Pending => {
                self.mode
                    .change_to_packages_window(&mut self.pkg_result_state);
            }
        }
    }

    fn handle_right(&mut self) {
        match self.mode {
            Mode::Search => {
                self.search_input.handle(InputRequest::GoToNextChar);
            }
            Mode::Packages => {
                self.mode
                    .change_to_pending_window(&mut self.pending_result_state);
            }
            Mode::Pending => {
                self.mode
                    .change_to_packages_window(&mut self.pkg_result_state);
            }
        }
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
                self.mode
                    .change_to_packages_window(&mut self.pkg_result_state);
            }
            Mode::Pending => {
                self.mode
                    .change_to_pending_window(&mut self.pending_result_state);
            }
        }
    }

    fn handle_space(&mut self) -> ControlFlow<()> {
        match self.mode {
            Mode::Search => {
                self.search_input.handle(InputRequest::InsertChar(' '));
                self.trigger_search();
            }
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
                                    if let Operation::Package { name, .. } = x {
                                        *name == self.remove[pkg_index].raw_pkg.fullname(true)
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

    fn handle_down(&mut self, is_pgdn: bool) {
        match self.mode {
            Mode::Search if !is_pgdn => {
                self.mode
                    .change_to_packages_window(&mut self.pkg_result_state);
            }
            Mode::Packages => {
                if is_pgdn {
                    self.pkg_result_state.page_down();
                } else {
                    self.pkg_result_state.next();
                }
                self.result_scroll = self
                    .result_scroll
                    .position(self.pkg_result_state.state.selected().unwrap_or(0));
            }
            Mode::Pending => {
                if is_pgdn {
                    self.pending_result_state.page_down();
                } else {
                    self.pending_result_state.next();
                }
            }
            _ => {}
        }
    }

    fn handle_up(&mut self, is_pgup: bool) {
        match self.mode {
            Mode::Search => {}
            Mode::Packages => {
                if self
                    .pkg_result_state
                    .state
                    .selected()
                    .map(|x| x == 0)
                    .unwrap_or(true)
                    && !is_pgup
                {
                    self.mode = Mode::Search;
                } else {
                    if is_pgup {
                        self.pkg_result_state.page_up();
                    } else {
                        self.pkg_result_state.previous();
                    }
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
                    && !is_pgup
                {
                    self.mode = Mode::Search;
                } else if is_pgup {
                    self.pending_result_state.page_up();
                } else {
                    self.pending_result_state.previous();
                }
            }
        }
    }
}
