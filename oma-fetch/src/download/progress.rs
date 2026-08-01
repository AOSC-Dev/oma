//! Progress reporting and per-attempt download state.

use flume::Sender;

use crate::{CompressType, Event};

use super::error::SingleDownloadError;

/// Tracks this file's progress events and the net bytes reported to the global
/// progress bar, so a failed attempt can undo them before retrying.
///
/// Events are sent synchronously to the internal channel, so the per-file bar
/// is cleared automatically via [`Drop`] even on early return or panic.
pub(crate) struct ProgressReporter {
    tx: Sender<Event>,
    index: usize,
    total: usize,
    reported: u64,
}

impl ProgressReporter {
    pub(crate) fn new(tx: &Sender<Event>, index: usize, total: usize) -> Self {
        Self {
            tx: tx.clone(),
            index,
            total,
            reported: 0,
        }
    }

    /// Net bytes this attempt has reported to the global progress bar.
    pub(crate) fn reported(&self) -> u64 {
        self.reported
    }

    /// Keep `reported` in sync with `downloaded_size` (the net contribution).
    pub(crate) fn set_reported(&mut self, reported: u64) {
        self.reported = reported;
    }

    /// Show the indeterminate spinner before the total size is known.
    pub(crate) fn spinner(&self, msg: &str) {
        let _ = self.tx.send(Event::NewProgressSpinner {
            index: self.index,
            msg: msg.to_string(),
            total: self.total,
        });
    }

    /// (Re)create the determinate progress bar.
    pub(crate) fn bar(&self, msg: &str, size: u64) {
        let _ = self.tx.send(Event::NewProgressBar {
            index: self.index,
            total: self.total,
            msg: msg.to_string(),
            size,
        });
    }

    /// Increment the per-file progress bar.
    pub(crate) fn inc(&self, n: u64) {
        let _ = self.tx.send(Event::ProgressInc {
            index: self.index,
            size: n,
        });
    }

    /// Finish and clear the per-file progress bar.
    pub(crate) fn done(&self) {
        let _ = self.tx.send(Event::ProgressDone(self.index));
    }

    /// Add bytes to the global progress bar.
    pub(crate) fn add(&self, n: u64) {
        let _ = self.tx.send(Event::GlobalProgressAdd(n));
    }

    /// Remove bytes from the global progress bar.
    pub(crate) fn sub(&self, n: u64) {
        let _ = self.tx.send(Event::GlobalProgressSub(n));
    }
}

impl Drop for ProgressReporter {
    fn drop(&mut self) {
        // Always clear the per-file bar, even on early return or panic.
        self.done();
    }
}

/// Mutable state of one HTTP download attempt: where we are in the file, how
/// much the server told us, and the bar bookkeeping needed to re-render it.
pub(crate) struct DownloadState {
    pub(crate) downloaded_size: u64,
    pub(crate) old_downloaded_size: u64,
    pub(crate) total_size: Option<u64>,
    pub(crate) old_total_size: Option<u64>,
    pub(crate) first_request: bool,
}

impl DownloadState {
    pub(crate) fn new() -> Self {
        Self {
            downloaded_size: 0,
            old_downloaded_size: 0,
            total_size: None,
            old_total_size: None,
            first_request: true,
        }
    }

    /// Loop condition: keep downloading until we've reached the known total.
    pub(crate) fn in_progress(&self) -> bool {
        match self.total_size {
            Some(total) => self.downloaded_size < total,
            None => true,
        }
    }

    /// Start a request attempt: remember the old total for bar comparison, and
    /// restart from offset 0 when resume is disallowed or the content is
    /// compressed.
    pub(crate) fn begin_attempt(&mut self, allow_resume: bool, file_type: CompressType) {
        self.old_total_size = self.total_size;
        if !allow_resume || file_type != CompressType::None {
            self.downloaded_size = 0;
        }
    }

    /// Give up on the current offset and download from the beginning.
    pub(crate) fn restart(&mut self) {
        self.downloaded_size = 0;
    }
}

/// Outcome of sending one HTTP request.
pub(crate) enum RequestOutcome {
    /// Response received, proceed to the next phase.
    Ready(reqwest::Response),
    /// The server rejected our range; restart the download loop.
    Restart,
    /// Fatal request error.
    Fatal(SingleDownloadError),
}

/// Outcome of reconciling a response with our resume offset.
#[derive(Debug, PartialEq, Eq)]
pub(crate) enum ResumeOutcome {
    /// Response accepted, continue to the next phase.
    Proceed,
    /// The server didn't honor our range; restart the download loop.
    Restart,
}
