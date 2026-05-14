use std::ops::ControlFlow;

use ratatui::crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use crate::subcommand::history_tui::HistorySelectTui;

impl<'a> HistorySelectTui<'a> {
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
                let i = self.history.state.selected().unwrap_or(0);
                let len = self.history.items.len();
                let new_index = (i + self.page_size).min(len.saturating_sub(1));
                self.history.state.select(Some(new_index));
            }
            KeyCode::PageUp => {
                let i = self.history.state.selected().unwrap_or(0);
                let new_index = i.saturating_sub(self.page_size);
                self.history.state.select(Some(new_index));
            }
            KeyCode::Char(' ') | KeyCode::Enter => {
                return ControlFlow::Break(self.history.state.selected());
            }
            _ => {}
        }

        ControlFlow::Continue(())
    }
}
