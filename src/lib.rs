use action::unlock_oma;
use lazy_static::lazy_static;
use nix::sys::signal;
use std::sync::atomic::{AtomicBool, AtomicI32, Ordering};

pub mod action;
mod checksum;
pub mod cli;
mod contents;
mod db;
mod download;
mod formatter;
mod pager;
mod pkg;
mod utils;
mod verify;

pub static SUBPROCESS: AtomicI32 = AtomicI32::new(-1);
pub static ALLOWCTRLC: AtomicBool = AtomicBool::new(false);
pub static LOCKED: AtomicBool = AtomicBool::new(false);

lazy_static! {
    pub static ref WRITER: cli::Writer = cli::Writer::new();
}

pub fn single_handler() {
    // Kill subprocess
    let subprocess_pid = SUBPROCESS.load(Ordering::Relaxed);
    let allow_ctrlc = ALLOWCTRLC.load(Ordering::Relaxed);
    if subprocess_pid > 0 {
        let pid = nix::unistd::Pid::from_raw(subprocess_pid);
        signal::kill(pid, signal::SIGTERM).expect("Failed to kill child process.");
        if !allow_ctrlc {
            info!("User aborted the operation");
        }
    }

    // Dealing with lock
    if LOCKED.load(Ordering::Relaxed) {
        unlock_oma().expect("Failed to unlock instance.");
    }

    // Show cursor before exiting.
    // This is not a big deal so we won't panic on this.
    let _ = WRITER.show_cursor();
    std::process::exit(2);
}
