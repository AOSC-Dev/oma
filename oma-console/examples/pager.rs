use oma_console::{
    pager::{OmaPager, Pager, PagerUIText},
    print::OmaColorFormat,
};
use ratatui::crossterm::style::Stylize;
use std::{io, time::Duration};

struct OmaPagerUIText;

impl PagerUIText for OmaPagerUIText {
    fn normal_tips(&self) -> String {
        "QAQ".to_string()
    }

    fn search_tips_with_result(&self) -> String {
        "Press Esc to exit search, press N or n to jump to the prev or next match.".to_string()
    }

    fn searct_tips_with_query(&self, query: &str) -> String {
        format!("Search: {}", query)
    }

    fn search_tips_with_empty(&self) -> String {
        "Search pattern cannot be empty (Press /)".to_string()
    }

    fn search_tips_not_found(&self) -> String {
        "Pattern not found (Press /)".to_string()
    }
}

fn main() -> io::Result<()> {
    let cf = OmaColorFormat::new(true, Duration::from_millis(100));
    let pager = OmaPager::new(Some("QAQ".to_string()), &cf, Box::new(OmaPagerUIText));
    let mut p = Pager::External(Box::new(pager));
    let mut w = p.get_writer()?;
    w.write_all("QAQ\n".cyan().to_string().as_bytes()).ok();
    w.write_all(b"PAP").ok();

    drop(w);
    p.wait_for_exit()?;

    Ok(())
}
