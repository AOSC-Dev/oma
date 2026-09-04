use std::{
    io::{self, IsTerminal, Write, stderr, stdin, stdout},
    time::Duration,
};

use oma_console::writer::Writer;
use ratatui::crossterm::{
    execute,
    terminal::{EnterAlternateScreen, enable_raw_mode},
};
use ratatui::{Terminal, backend::CrosstermBackend, restore};

use crate::oma_pager::OmaPager;
use crate::traits::{PagerExit, PagerTheme, PagerUIText};

pub enum Pager {
    Plain,
    External(Box<OmaPager>),
}

impl Pager {
    pub fn plain() -> Self {
        Self::Plain
    }

    pub fn external(
        ui_text: Box<dyn PagerUIText>,
        title: Option<String>,
        theme: Option<Box<dyn PagerTheme>>,
        yn_mode: bool,
    ) -> io::Result<Self> {
        if !stdout().is_terminal() || !stderr().is_terminal() || !stdin().is_terminal() {
            return Ok(Pager::Plain);
        }

        let app = OmaPager::new(title, theme, ui_text, yn_mode);
        let res = Pager::External(Box::new(app));

        Ok(res)
    }

    /// Get writer to writer something to pager
    pub fn get_writer(&mut self) -> io::Result<Box<dyn Write + '_>> {
        let res = match self {
            Pager::Plain => Writer::new_stdout().get_writer(),
            Pager::External(app) => {
                let res: Box<dyn Write> = Box::new(app);
                res
            }
        };

        Ok(res)
    }

    /// Wait for the pager to exit
    /// Use this function to start the pager
    pub fn wait_for_exit(self) -> io::Result<PagerExit> {
        let success = if let Pager::External(app) = self {
            let mut terminal = prepare_create_tui()?;
            let res = app.run(&mut terminal, Duration::from_millis(250))?;
            exit_tui(&mut terminal)?;

            res
        } else {
            PagerExit::NormalExit
        };

        Ok(success)
    }
}

pub fn exit_tui(terminal: &mut Terminal<CrosstermBackend<io::Stdout>>) -> io::Result<()> {
    restore();
    terminal.show_cursor()?;

    Ok(())
}

pub fn prepare_create_tui() -> io::Result<Terminal<CrosstermBackend<io::Stdout>>> {
    let hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        restore();
        hook(info);
    }));

    execute!(stdout(), EnterAlternateScreen)?;
    enable_raw_mode()?;

    let backend = CrosstermBackend::new(stdout());
    let mut terminal = Terminal::new(backend)?;

    terminal.clear()?;

    Ok(terminal)
}
