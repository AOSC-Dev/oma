use crate::{error::OutputError, fl};
use console::{Key, Term};
use std::io;

// User's action after reviewing the information displayed with the pager.
pub enum PromptAction {
    Continue, // User accepted this "transcation"
    Review,   // User wants to review it again
    Abort,    // Something is wrong, quit
}

// Allows io::Error to be passed to oma.
impl From<io::Error> for OutputError {
    fn from(_value: io::Error) -> Self {
        Self {
            description: fl!("failed-to-read-answer-for-prompt"),
            source: None,
        }
    }
}

pub fn ask_prompt(prompt_text: &str, default: PromptAction) -> Result<PromptAction, OutputError> {
    let term = Term::stderr();
    term.write_str(prompt_text)?;
    term.write_line("")?;
    if !term.is_term() {
        // This is not a interactive terminal
        return Ok(default);
    }
    let action = loop {
        let input = term.read_key()?;
        match input {
            Key::Char('y') | Key::Char('Y') => break PromptAction::Continue,
            Key::Char('n') | Key::Char('N') => break PromptAction::Abort,
	    // User has a chance to review it again (and again and again ...)
            Key::Char('r') | Key::Char('R') => break PromptAction::Review,
            _ => {
                continue;
            }
        }
    };
    Ok(action)
}
