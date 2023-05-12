use std::process::exit;
use std::sync::Arc;

use anyhow::{anyhow, Result};

use cli::Writer;
use indicatif::MultiProgress;
use nix::sys::signal;
use once_cell::sync::{Lazy, OnceCell};
use std::sync::atomic::{AtomicBool, AtomicI32, Ordering};
use time::macros::offset;
use time::{OffsetDateTime, UtcOffset};
use utils::{get_arch_name, unlock_oma};

use crate::cli::CommandMatcher;
use crate::cli::OmaCommandRunner;

mod args;
mod checksum;
mod cli;
mod contents;
mod db;
mod download;
mod formatter;
mod oma;
mod pager;
mod pkg;

#[cfg(feature = "aosc")]
mod topics;

mod utils;
mod verify;

static SUBPROCESS: AtomicI32 = AtomicI32::new(-1);
static ALLOWCTRLC: AtomicBool = AtomicBool::new(false);
static LOCKED: AtomicBool = AtomicBool::new(false);
static AILURUS: AtomicBool = AtomicBool::new(false);
static WRITER: Lazy<Writer> = Lazy::new(Writer::new);
static DRYRUN: AtomicBool = AtomicBool::new(false);
static TIME_OFFSET: Lazy<UtcOffset> =
    Lazy::new(|| UtcOffset::local_offset_at(OffsetDateTime::UNIX_EPOCH).unwrap_or(offset!(UTC)));
static ARGS: Lazy<String> = Lazy::new(|| std::env::args().collect::<Vec<_>>().join(" "));
static MB: Lazy<Arc<MultiProgress>> = Lazy::new(|| Arc::new(MultiProgress::new()));

static ARCH: OnceCell<String> = OnceCell::new();

fn main() {
    // 初始化时区偏移量，这个操作不能在多线程环境下运行
    let _ = *TIME_OFFSET;

    ctrlc::set_handler(single_handler).expect(
        "Oma could not initialize SIGINT handler.\n\nPlease restart your installation environment.",
    );

    let code = match try_main() {
        Ok(exit_code) => exit_code,
        Err(e) => {
            if !e.to_string().is_empty() {
                error!("{e}");
            }
            e.chain().skip(1).for_each(|cause| {
                due_to!("{}", cause);
            });
            unlock_oma().ok();
            1
        }
    };

    unlock_oma().ok();
    eprint!("\x07"); // bell character

    exit(code);
}

fn try_main() -> Result<i32> {
    let _ = ARCH
        .get_or_try_init(get_arch_name)
        .map_err(|e| anyhow!("Can not run dpkg --print-architecture, why: {e}"))?;
    
    tracing::info!("Running oma with args: {}", *ARGS);
    let code = OmaCommandRunner::new().run()?;

    Ok(code)
}

fn single_handler() {
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
