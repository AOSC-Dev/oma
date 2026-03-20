use std::{
    io::{IsTerminal, stderr, stdin, stdout},
    process::exit,
};

use oma_console::pager::PagerExit;

pub struct ExitHandle {
    ring: bool,
    status: ExitStatus,
}

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub enum ExitStatus {
    Success,
    Fail,
    Other(i32),
}

impl Default for ExitHandle {
    fn default() -> Self {
        Self {
            ring: false,
            status: ExitStatus::Success,
        }
    }
}

impl From<PagerExit> for ExitStatus {
    fn from(value: PagerExit) -> Self {
        match value {
            PagerExit::NormalExit => ExitStatus::Success,
            x => ExitStatus::Other(x.into()),
        }
    }
}

#[allow(dead_code)]
impl ExitHandle {
    pub fn ring(mut self, ring: bool) -> Self {
        self.ring = ring;
        self
    }

    pub fn status(mut self, status: ExitStatus) -> Self {
        self.status = status;
        self
    }

    pub fn get_status(&self) -> ExitStatus {
        self.status
    }

    pub fn handle(self, config_ring: bool) -> ! {
        if self.ring && config_ring {
            terminal_ring();
        }

        match self.status {
            ExitStatus::Success => exit(0),
            ExitStatus::Fail => exit(1),
            ExitStatus::Other(status) => exit(status),
        }
    }
}

pub fn terminal_ring() {
    if !stdout().is_terminal() || !stderr().is_terminal() || !stdin().is_terminal() {
        return;
    }

    eprint!("\x07"); // bell character
}
