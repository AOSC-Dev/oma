use crossterm::style::Stylize;
use oma_console::pager::{OmaPager, Pager};
use std::io;

fn main() -> io::Result<()> {
    let pager = OmaPager::new("PAP", Some("QAQ"));
    let mut p = Pager::External(pager);
    let mut w = p.get_writer()?;
    w.write_all("QAQ\n".cyan().to_string().as_bytes()).ok();
    w.write_all(b"PAP").ok();

    drop(w);
    p.wait_for_exit()?;

    Ok(())
}
