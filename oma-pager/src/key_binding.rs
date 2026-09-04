use ratatui::crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use tui_input::InputRequest;

use crate::oma_pager::{OmaPager, TuiMode};
use crate::traits::PagerExit;

/// The result of handling a key binding.
pub enum Control {
    /// Continue the pager loop.
    Continue,
    /// Exit the pager with the given exit status.
    Exit(PagerExit),
}

impl OmaPager {
    pub(crate) fn handle_key_binding(&mut self, key: KeyEvent) -> Control {
        if key.modifiers == KeyModifiers::CONTROL && key.code == KeyCode::Char('c') {
            return Control::Exit(PagerExit::Sigint);
        }

        if self.mode == TuiMode::SearchInputText {
            match key.code {
                KeyCode::Esc => {
                    self.exit_search_mode();
                }
                KeyCode::Enter => {
                    let query = self.search_input.value().to_string();
                    if query.trim().is_empty() {
                        self.tips = self.ui_text.search_tips_with_empty();
                    } else {
                        self.perform_search(&query);
                    }
                    self.mode = TuiMode::Search;
                }
                _ => {
                    match key.code {
                        KeyCode::Char(c) => {
                            self.search_input.handle(InputRequest::InsertChar(c));
                        }
                        KeyCode::Backspace => {
                            self.search_input.handle(InputRequest::DeletePrevChar);
                        }
                        KeyCode::Delete => {
                            self.search_input.handle(InputRequest::DeleteNextChar);
                        }
                        KeyCode::Left => {
                            self.search_input.handle(InputRequest::GoToPrevChar);
                        }
                        KeyCode::Right => {
                            self.search_input.handle(InputRequest::GoToNextChar);
                        }
                        KeyCode::Home => {
                            self.search_input.handle(InputRequest::GoToStart);
                        }
                        KeyCode::End => {
                            self.search_input.handle(InputRequest::GoToEnd);
                        }
                        _ => {}
                    }
                    self.tips = self
                        .ui_text
                        .searct_tips_with_query(self.search_input.value());
                }
            }
            return Control::Continue;
        }

        if key.modifiers == KeyModifiers::CONTROL {
            match key.code {
                KeyCode::Char('c') => return Control::Exit(PagerExit::Sigint),
                KeyCode::Char('p') => self.up(),
                KeyCode::Char('n') => self.down(),
                _ => {}
            }
            return Control::Continue;
        }

        match key.code {
            KeyCode::Char('/') => {
                self.clear_highlight();
                self.mode = TuiMode::SearchInputText;
                self.tips = self
                    .ui_text
                    .searct_tips_with_query(self.search_input.value());
            }
            KeyCode::Esc => {
                if self.mode != TuiMode::Normal {
                    self.exit_search_mode();
                }
            }
            KeyCode::Enter => {
                self.down();
            }
            KeyCode::Down | KeyCode::Char('j') => self.down(),
            KeyCode::Up | KeyCode::Char('k') => self.up(),
            KeyCode::Left | KeyCode::Char('h') => self.left(),
            KeyCode::Right | KeyCode::Char('l') => self.right(),
            KeyCode::PageUp
            | KeyCode::Char('u')
            | KeyCode::Char('U')
            | KeyCode::Char('b')
            | KeyCode::Char('B') => self.page_up(),
            KeyCode::PageDown
            | KeyCode::Char('d')
            | KeyCode::Char('D')
            | KeyCode::Char(' ')
            | KeyCode::Char('f')
            | KeyCode::Char('F') => self.page_down(),
            KeyCode::Home | KeyCode::Char('g') => self.goto_begin(),
            KeyCode::End | KeyCode::Char('G') => self.goto_end(),
            KeyCode::Char('n') => {
                if self.yn_mode {
                    return Control::Exit(PagerExit::Sigint);
                }
                if self.mode == TuiMode::Search && !self.search_results.is_empty() {
                    self.current_result_index =
                        (self.current_result_index + 1) % self.search_results.len();
                    self.jump_to(self.search_results[self.current_result_index]);
                }
            }
            KeyCode::Char('N') => {
                if self.mode == TuiMode::Search && !self.search_results.is_empty() {
                    if self.current_result_index == 0 {
                        self.current_result_index = self.search_results.len() - 1;
                    } else {
                        self.current_result_index -= 1;
                    }
                    self.jump_to(self.search_results[self.current_result_index]);
                }
            }
            KeyCode::Char(c) if c == 'q' || c == 'Q' => {
                if !self.yn_mode {
                    return Control::Exit(PagerExit::NormalExit);
                } else {
                    return Control::Exit(PagerExit::Sigint);
                }
            }
            KeyCode::Char('y') => {
                if self.yn_mode {
                    return Control::Exit(PagerExit::NormalExit);
                } else {
                    self.up();
                }
            }
            _ => {}
        }

        Control::Continue
    }
}
