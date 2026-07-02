use std::{
    io::{self},
    ops::ControlFlow,
};

use chrono::{Local, TimeZone, offset::LocalResult};
use ratatui::{
    backend::Backend,
    crossterm::{
        event::{Event, KeyCode, KeyEvent, KeyModifiers},
        terminal::{disable_raw_mode, enable_raw_mode},
    },
    widgets::ScrollbarState,
};
use tui_input::backend::crossterm::to_input_request;

use crate::{subcommand::history_tui::HistorySelectTui, table::table_for_history_pending};

impl<'a> HistorySelectTui<'a> {
    pub(crate) fn handle_key_event<B: Backend>(
        &mut self,
        key: KeyEvent,
        terminal: &mut ratatui::Terminal<B>,
    ) -> io::Result<ControlFlow<Option<i64>>> {
        if (key.modifiers == KeyModifiers::CONTROL && key.code == KeyCode::Char('c'))
            || key.code == KeyCode::Esc
        {
            return Ok(ControlFlow::Break(None));
        }

        match key.code {
            KeyCode::Down => {
                self.history_list.next();
            }
            KeyCode::Up => {
                self.history_list.previous();
            }
            KeyCode::PageDown => {
                let i = self.history_list.state.selected().unwrap_or(0);
                let len = self.history_list.items.len();
                let new_index = (i + self.page_size).min(len.saturating_sub(1));
                self.history_list.state.select(Some(new_index));
            }
            KeyCode::PageUp => {
                let i = self.history_list.state.selected().unwrap_or(0);
                let new_index = i.saturating_sub(self.page_size);
                self.history_list.state.select(Some(new_index));
            }
            KeyCode::Enter => {
                let id = self
                    .history_list
                    .state
                    .selected()
                    .map(|idx| self.history_list.items[idx].id);

                let id = id.ok_or_else(|| io::Error::other("history id is none"))?;

                let entry = self.db.find_history_by_id(id).map_err(io::Error::other)?;

                let target_position = {
                    let frame = terminal.get_frame();
                    frame.area().as_position()
                };

                terminal
                    .set_cursor_position(target_position)
                    .map_err(|e| io::Error::other(e.to_string()))?;

                terminal
                    .clear()
                    .map_err(|e| io::Error::other(e.to_string()))?;

                terminal
                    .autoresize()
                    .map_err(|e| io::Error::other(e.to_string()))?;

                disable_raw_mode()?;
                terminal
                    .show_cursor()
                    .map_err(|e| io::Error::other(e.to_string()))?;

                if !self.undo {
                    table_for_history_pending(&entry.install, &entry.remove, entry.disk_size)
                        .map_err(|e| io::Error::other(e.to_string()))?;
                } else {
                    return Ok(ControlFlow::Break(Some(id)));
                }

                enable_raw_mode()?;
            }
            _ => {}
        }

        if let Some(req) = to_input_request(&Event::Key(key))
            && let Some(state_changed) = self.search_input.handle(req)
            && state_changed.value
        {
            let query = self.search_input.value();

            if query.is_empty() {
                self.history_list.items = self.all_entries.clone();
            } else {
                let contains_query_pkg = self
                    .db
                    .query_like_install_and_remove_pkgname_item(query)
                    .map_err(io::Error::other)?;

                self.history_list.items = self
                    .all_entries
                    .iter()
                    .filter(|entry| {
                        let dt = match Local.timestamp_opt(entry.time, 0) {
                            LocalResult::None => Local.timestamp_opt(0, 0).unwrap(),
                            x => x.unwrap(),
                        }
                        .format("%H:%M:%S on %Y-%m-%d")
                        .to_string();

                        entry.command.to_lowercase().contains(query)
                            || contains_query_pkg.contains(&entry.id)
                            || dt.contains(query)
                            || (query == "FAIL" && !entry.is_success)
                            || (query == "SUCCESS" && entry.is_success)
                    })
                    .cloned()
                    .collect();
            }

            // 重置滚动条和高亮位置
            self.scroll_state = ScrollbarState::new(self.history_list.items.len()).position(0);
            if self.history_list.items.is_empty() {
                self.history_list.state.select(None);
            } else {
                self.history_list.state.select(Some(0));
            }
        }

        Ok(ControlFlow::Continue(()))
    }
}
