use ratatui::{style::Style, text::Text};

use crate::tui::{render::Operation, state::StatefulList};

#[derive(PartialEq, Eq)]
pub(crate) enum Mode {
    Search,
    Packages,
    Pending,
}

impl Mode {
    #[inline]
    pub(crate) fn highlight_window(&self, another: &Self) -> Style {
        if self == another {
            Style::default().bold()
        } else {
            Style::default()
        }
    }

    pub(crate) fn change_to_packages_window(
        &mut self,
        display_list: &mut StatefulList<Text<'static>>,
    ) {
        *self = Mode::Packages;
        if display_list.state.selected().is_none() && !display_list.items.is_empty() {
            display_list.state.select(Some(0));
        }
    }

    pub(crate) fn change_to_pending_window(
        &mut self,
        pending_display_list: &mut StatefulList<Operation>,
    ) {
        *self = Mode::Pending;
        if pending_display_list.state.selected().is_none() && !pending_display_list.items.is_empty()
        {
            pending_display_list.state.select(Some(0));
        }
    }
}
