use oma_console::pager::Pager;
use std::io;

fn main() -> io::Result<()> {
    let mut p = Pager::plain();
    let mut w = p.get_writer()?;
    w.write_all(b"QAQ\n").ok();
    w.write_all(b"PAP").ok();

    drop(w);
    p.wait_for_exit()?;

    Ok(())
}
