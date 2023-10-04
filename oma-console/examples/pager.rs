use oma_console::{pager::Pager, OmaConsoleError};

fn main() -> Result<(), OmaConsoleError> {
    let mut p = Pager::plain();
    let mut w = p.get_writer()?;
    w.write_all(b"QAQ\n").ok();
    w.write_all(b"PAP").ok();

    drop(w);
    p.wait_for_exit()?;

    Ok(())
}
