use std::ops::ControlFlow;

use ratatui::crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use crate::subcommand::history_tui::HistorySelectTui;

impl<'a> HistorySelectTui<'a> {
    const PAGE_LEN: usize = 10;
    pub(crate) fn handle_key_event(&mut self, key: KeyEvent) -> ControlFlow<Option<usize>> {
        if (key.modifiers == KeyModifiers::CONTROL && key.code == KeyCode::Char('c'))
            || key.code == KeyCode::Esc
        {
            return ControlFlow::Break(None);
        }

        match key.code {
            KeyCode::Down => {
                self.history.next();
            }
            KeyCode::Up => {
                self.history.previous();
            }
            KeyCode::PageDown => {
                let pgdn_scroll_select =
                    self.history.state.selected().unwrap_or(0) + Self::PAGE_LEN;

                if pgdn_scroll_select > self.history.items.len() {
                    self.scroll_state.last();
                    return ControlFlow::Continue(());
                }

                self.history.state.select(Some(pgdn_scroll_select));
                self.scroll_state = self.scroll_state.position(pgdn_scroll_select);
            }
            KeyCode::PageUp => {
                let pgdn_scroll_select = self
                    .history
                    .state
                    .selected()
                    .unwrap_or(0)
                    .saturating_sub(Self::PAGE_LEN);

                self.history.state.select(Some(pgdn_scroll_select));
                self.scroll_state = self.scroll_state.position(pgdn_scroll_select);
            }
            KeyCode::Char(' ') | KeyCode::Enter => {
                return ControlFlow::Break(self.history.state.selected());
            }
            _ => {}
        }

        ControlFlow::Continue(())
    }
}
