use std::ops::ControlFlow;

use oma_pm::oma_apt::PackageSort;
use ratatui::crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use crate::{
    fl,
    subcommand::size_analyzer::{pkg::PkgWrapper, state::Window, tui::PkgSizeAnalyzer},
};

impl<'a> PkgSizeAnalyzer<'a> {
    pub(crate) fn handle_key_event(&mut self, key: KeyEvent) -> ControlFlow<Vec<PkgWrapper<'a>>> {
        if key.modifiers == KeyModifiers::CONTROL && key.code == KeyCode::Char('c') {
            return ControlFlow::Break(vec![]);
        }

        if key.modifiers == KeyModifiers::CONTROL && key.code == KeyCode::Char('a') {
            let sort = PackageSort::default().auto_removable();
            let pkgs = self.apt.cache.packages(&sort);
            let pkgs = pkgs.collect::<Vec<_>>();

            if pkgs.is_empty() {
                self.popup = Some(fl!("tui-no-package-clean-up"));
            } else {
                for p in pkgs {
                    self.remove_package.items.push(PkgWrapper { pkg: p });
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
                    return ControlFlow::Continue(());
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

                self.installed_scroll = self.installed_scroll.saturating_sub(Self::PAGE_LEN);
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
                    return ControlFlow::Continue(());
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
                return ControlFlow::Break(std::mem::take(&mut self.remove_package.items));
            }
            _ => {}
        }

        ControlFlow::Continue(())
    }
}
