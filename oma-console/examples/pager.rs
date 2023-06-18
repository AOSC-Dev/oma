use oma_console::{pager::Pager, OmaConsoleError};

fn main() -> Result<(), OmaConsoleError> {
    let mut p = Pager::new(false, "Press [q] to end review, [Ctrl-c] to abort, [PgUp/Dn], arrow keys, or mouse wheel to scroll.")?;
    let mut w = p.get_writer()?;
    w.write_all(b"QAQ\n").ok();
    w.write_all(b"PAP").ok();

    drop(w);
    p.wait_for_exit()?;

    Ok(())
}
