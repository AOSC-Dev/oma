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
            KeyCode::Char(' ') | KeyCode::Enter => {
                return ControlFlow::Break(self.history.state.selected());
            }
            _ => {}
        }

        ControlFlow::Continue(())
    }
}
