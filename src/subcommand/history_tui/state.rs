use std::rc::Rc;

use ratatui::widgets::TableState;

pub struct StatefulList<T> {
    pub state: TableState,
    pub items: Vec<Rc<T>>,
}

impl<T> StatefulList<T> {
    pub fn with_items(items: Vec<Rc<T>>) -> StatefulList<T> {
        StatefulList {
            state: TableState::default(),
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
