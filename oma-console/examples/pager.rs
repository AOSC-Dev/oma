use crossterm::style::Stylize;
use oma_console::{
    pager::{OmaPager, Pager},
    print::OmaColorFormat,
};
use std::{io, time::Duration};

fn main() -> io::Result<()> {
    let pager = OmaPager::new(
        "PAP",
        Some("QAQ"),
        &OmaColorFormat::new(true, Duration::from_millis(100)),
    );
    let mut p = Pager::External(pager);
    let mut w = p.get_writer()?;
    w.write_all("QAQ\n".cyan().to_string().as_bytes()).ok();
    w.write_all(b"PAP").ok();

    drop(w);
    p.wait_for_exit()?;

    Ok(())
}
