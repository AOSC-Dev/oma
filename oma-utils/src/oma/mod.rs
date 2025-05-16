use std::{
    io::{Error, IsTerminal, stderr, stdin, stdout},
    path::PathBuf,
    sync::LazyLock,
};

type IOResult<T> = std::io::Result<T>;
static LOCK: LazyLock<PathBuf> = LazyLock::new(|| PathBuf::from("/run/lock/oma.lock"));

/// lock oma
pub fn lock_oma_inner() -> IOResult<()> {
    if !LOCK.is_file() {
        std::fs::create_dir_all("/run/lock")?;
        std::fs::File::create(&*LOCK)?;
        return Ok(());
    }

    Err(Error::other(""))
}

/// Unlock oma
pub fn unlock_oma() -> IOResult<()> {
    if LOCK.is_file() {
        std::fs::remove_file(&*LOCK)?;
    }

    Ok(())
}

/// terminal bell character
pub fn terminal_ring() {
    if !stdout().is_terminal() || !stderr().is_terminal() || !stdin().is_terminal() {
        return;
    }

    eprint!("\x07"); // bell character
}
