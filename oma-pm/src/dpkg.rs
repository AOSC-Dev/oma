use std::fmt;
use std::fs::File;
use std::io::{ErrorKind, Read, Write};
use std::mem::zeroed;
use std::num::ParseFloatError;
use std::os::fd::{FromRawFd, RawFd};

use mio::event::Iter;
use mio::unix::SourceFd;
use mio::{Events, Interest, Poll, Token};
use nix::libc::{winsize, TIOCGWINSZ, TIOCSWINSZ};
use nix::sys::signal::{self, SigHandler};
use nix::sys::wait::{waitpid, WaitPidFlag, WaitStatus};
use nix::unistd::{close, Pid};
use nix::{ioctl_read_bad, ioctl_write_ptr_bad};
use tracing::debug;

static mut CHILD_FD: i32 = 0;
const STDIN_FD: i32 = 0;

// Define the ioctl read call for TIOCGWINSZ
ioctl_read_bad!(tiocgwinsz, TIOCGWINSZ, winsize);
// Define the ioctl write call for TIOCSWINSZ
ioctl_write_ptr_bad!(tiocswinsz, TIOCSWINSZ, winsize);

/// Get Terminal Size from stdin
pub unsafe fn get_winsize() -> nix::Result<winsize> {
    let mut ws: winsize = unsafe { zeroed() };
    tiocgwinsz(STDIN_FD, &mut ws)?;
    Ok(ws)
}

extern "C" fn sigwinch_passthrough(_: i32) {
    unsafe {
        // Get Terminal Size from stdin.
        let ws = get_winsize().unwrap();
        // Set Terminal Size for pty.
        tiocswinsz(CHILD_FD, &ws).unwrap();
    }
}

enum PtyStr<'a> {
    Str(&'a str),
    None,
    Eof,
}

#[derive(Debug, thiserror::Error)]
pub enum ReadDpkgStatusError {
    #[error(transparent)]
    Errno(#[from] nix::errno::Errno),
    #[error(transparent)]
    Io(#[from] std::io::Error),
    #[error("dpkg is exit non-zero: {0}")]
    Dpkg(i32),
    #[error(transparent)]
    Utf8Error(#[from] std::str::Utf8Error),
    #[error(transparent)]
    ParseFloatError(#[from] ParseFloatError),
}

pub struct Pty {
    status: File,
    pty: File,
    stdin: File,
    status_buf: [u8; 4096],
    pty_buf: [u8; 4096],
    poll: Poll,
    events: Events,
    tokens: [(Token, Interest); 3],
}

impl fmt::Debug for Pty {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.debug_struct("Pty")
            .field("stdin_ready", &self.stdin_ready())
            .field("pty_ready", &self.ready())
            .field("status_ready", &self.status_ready())
            .finish()
    }
}

impl Pty {
    pub fn new(writefd: RawFd, statusfd: RawFd, master: RawFd) -> Result<Pty, ReadDpkgStatusError> {
        // This is for the Parent, close the write end of the pipe.
        close(writefd)?;

        let tokens = [
            (Token(0), Interest::READABLE),
            (Token(master as usize), Interest::READABLE),
            (Token(statusfd as usize), Interest::READABLE),
        ];

        // Create a poll instance
        let poll = Poll::new()?;
        let events = Events::with_capacity(3);

        for token in tokens {
            poll.registry()
                .register(&mut SourceFd(&(token.0 .0 as i32)), token.0, token.1)?;
        }

        unsafe {
            CHILD_FD = master;
            signal::signal(signal::SIGWINCH, SigHandler::Handler(sigwinch_passthrough))?;

            Ok(Pty {
                status: File::from_raw_fd(statusfd),
                pty: File::from_raw_fd(master),
                stdin: File::from_raw_fd(0),
                status_buf: [0u8; 4096],
                pty_buf: [0u8; 4096],
                poll,
                events,
                tokens,
            })
        }
    }

    fn read_master(&mut self, cb: impl Fn(&str)) -> Result<bool, ReadDpkgStatusError> {
        match read_fd(&mut self.pty, &mut self.pty_buf)? {
            PtyStr::Str(string) => {
                for line in string.lines() {
                    // dprog!(config, progress, "pty", "{line:?}");
                    debug!("pty: {line}");

                    if line.trim().is_empty() || check_spam(line) {
                        continue;
                    }

                    // Occasionally there is a line which comes through
                    if line.ends_with('\r') {
                        continue;
                    }

                    // Sometimes just a percentage comes through "35%"
                    if line.chars().nth(2).is_some_and(|c| c == '%') {
                        continue;
                    }

                    cb(line);
                }
                Ok(true)
            }
            PtyStr::None => Ok(true),
            PtyStr::Eof => Ok(false),
        }
    }

    fn read_status(&mut self, cb: impl Fn(&DpkgStatus)) -> Result<bool, ReadDpkgStatusError> {
        match read_fd(&mut self.status, &mut self.status_buf)? {
            PtyStr::Str(string) => {
                for line in string.lines() {
                    let status = DpkgStatus::try_from(line)?;
                    cb(&status);
                }
                Ok(true)
            }
            PtyStr::None => Ok(true),
            PtyStr::Eof => Ok(false),
        }
    }

    /// Checks the status of the child, polls Fds and checks if they're ready.
    fn poll(&mut self, child: Pid) -> Result<bool, ReadDpkgStatusError> {
        // Wait for the child process to finish and get its exit code
        let wait_status = waitpid(child, Some(WaitPidFlag::WNOHANG))?;
        if let WaitStatus::Exited(_, exit_code) = wait_status {
            if exit_code != 0 {
                return Err(ReadDpkgStatusError::Dpkg(exit_code));
            }
        }

        // When resizing the terminal poll will be Error Interrupted
        // Just wait until that's not the case.
        while let Err(e) = self.poll.poll(&mut self.events, None) {
            if let ErrorKind::Interrupted = e.kind() {
                continue;
            }
            return Err(e.into());
        }

        Ok(!self.is_read_closed())
    }

    fn is_read_closed(&self) -> bool {
        self.events
            .iter()
            .any(|e| e.token() == self.tokens[1].0 && e.is_read_closed())
    }

    fn events(&self) -> Iter<'_> {
        self.events.iter()
    }

    /// Stdin Fd is ready to be read.
    fn stdin_ready(&self) -> bool {
        self.io_ready(0)
    }

    /// Pty master Fd is ready to be read.
    fn ready(&self) -> bool {
        self.io_ready(1)
    }

    /// Status Fd is ready to be read.
    fn status_ready(&self) -> bool {
        self.io_ready(2)
    }

    /// Helper function for the ready checkers above.
    fn io_ready(&self, i: usize) -> bool {
        self.events().any(|e| e.token() == self.tokens[i].0)
    }

    fn stdin_to_pty(&mut self) -> Result<bool, ReadDpkgStatusError> {
        let mut buffer = [0u8; 4096];
        match read_fd(&mut self.stdin, &mut buffer)? {
            PtyStr::Str(input) => {
                write!(self.pty, "{input}")?;
                Ok(true)
            }
            PtyStr::None => Ok(true),
            PtyStr::Eof => Ok(false),
        }
    }

    pub fn listen_to_child(
        &mut self,
        child: Pid,
        status_handler: impl Fn(&DpkgStatus),
        line_handler: impl Fn(&str),
    ) -> Result<bool, ReadDpkgStatusError> {
        if !self.poll(child)? {
            return Ok(false);
        }

        debug!("{:#?}", self);

        if self.status_ready() && !self.read_status(status_handler)? {
            return Ok(false);
        }

        if self.ready() {
            return self.read_master(line_handler);
        }

        if self.stdin_ready() {
            return self.stdin_to_pty();
        }

        Ok(true)
    }
}

fn check_spam(line: &str) -> bool {
    [
        "Nothing to fetch",
        "(Reading database",
        "Selecting previously unselected package",
        "Preparing to unpack",
    ]
    .iter()
    .any(|spam| line.contains(spam))
}

fn read_fd<'a>(file: &mut File, buffer: &'a mut [u8]) -> Result<PtyStr<'a>, ReadDpkgStatusError> {
    let sized_buf = match file.read(buffer) {
        Ok(0) => return Ok(PtyStr::Eof),
        Ok(num) => &buffer[..num],
        Err(ref e) if e.kind() == ErrorKind::WouldBlock => return Ok(PtyStr::None),
        Err(ref e) if e.raw_os_error().is_some_and(|code| code == 5 || code == 4) => {
            return Ok(PtyStr::Eof)
        }
        Err(e) => return Err(e.into()),
    };

    Ok(PtyStr::Str(std::str::from_utf8(sized_buf)?))
}

#[derive(Debug, Default)]
pub enum DpkgStatusType {
    Status,
    Error,
    ConfFile,
    #[default]
    None,
}

impl From<&str> for DpkgStatusType {
    fn from(value: &str) -> Self {
        match value {
            "pmstatus" => Self::Status,
            "pmerror" => Self::Error,
            "pmconffile" => Self::ConfFile,
            _ => unreachable!(),
        }
    }
}

#[derive(Debug, Default)]
pub struct DpkgStatus {
    pub status_type: DpkgStatusType,
    pub pkg_name: String,
    pub percent: u64,
    _status: String,
}

impl TryFrom<&str> for DpkgStatus {
    type Error = ReadDpkgStatusError;

    fn try_from(value: &str) -> std::result::Result<Self, Self::Error> {
        let status: Vec<&str> = value.split(':').collect();

        Ok(DpkgStatus {
            status_type: DpkgStatusType::from(status[0]),
            pkg_name: status[1].into(),
            percent: status[2].parse::<f64>()? as u64,
            _status: status[3].into(),
        })
    }
}
