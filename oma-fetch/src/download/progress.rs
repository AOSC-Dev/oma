//! Progress reporting and per-attempt download state.

use flume::Sender;

use crate::{CompressType, Event};

/// Tracks this file's progress events and the net bytes reported to the global
/// progress bar, so a failed attempt can undo them before retrying.
///
/// Events are sent synchronously to the internal channel, so on failure the
/// per-file bar is cleared and the global bytes undone automatically via
/// [`Drop`], even on early return or panic.
pub(crate) struct ProgressReporter {
    tx: Sender<Event>,
    index: usize,
    total: usize,
    /// Current bar position: the download offset this attempt has reached,
    /// which also equals the net bytes reported to the global progress bar
    /// (the two always move together, see [`Self::set_position`]).
    position: u64,
    /// Total size the bar was last (re)created with.
    last_total: Option<u64>,
    /// Whether a determinate view has been shown (vs. the initial spinner).
    has_determinate: bool,
    /// Whether the download finished successfully: `Drop` then clears the
    /// per-file bar without undoing the global bytes.
    finished: bool,
}

impl ProgressReporter {
    pub(crate) fn new(tx: &Sender<Event>, index: usize, total: usize) -> Self {
        Self {
            tx: tx.clone(),
            index,
            total,
            position: 0,
            last_total: None,
            has_determinate: false,
            finished: false,
        }
    }

    /// Mark the download as finished successfully: clear the per-file bar
    /// without undoing the global bytes (and skip the failure cleanup in
    /// [`Drop`]).
    pub(crate) fn finish(&mut self) {
        self.finished = true;
        self.done(0);
    }

    /// Account for bytes already reported to the global bar by code outside
    /// this reporter (e.g. the checksum helper while seeding a resume file),
    /// without emitting any events.
    pub(crate) fn set_position(&mut self, offset: u64) {
        self.position = offset;
    }

    /// Bring the bars up to date with `offset` (the absolute download
    /// offset), (re)creating the per-file bar when needed and deriving the
    /// delta to the global bar. This is the only place the bars advance.
    pub(crate) fn update(&mut self, msg: &str, offset: u64, total_size: Option<u64>) {
        if self.last_total != total_size || offset < self.position || !self.has_determinate {
            // recreate the bar if:
            // 1. total size changed
            // 2. offset moved backwards
            // 3. no determinate bar yet (the previous view was a spinner)
            self.done(self.position);
            self.start_determinate(msg, total_size.unwrap_or(0));
            // the fresh bar starts at 0; move both bars to `offset` (the
            // `done` cancels the global side when the offset was pre-seeded)
            self.advance(offset);
        } else if offset > self.position {
            self.advance(offset - self.position);
        }
        self.position = offset;
        self.last_total = total_size;
    }

    /// Announce this download item as indeterminate, before its total size is
    /// known.
    pub(crate) fn start_indeterminate(&self, msg: &str) {
        let _ = self.tx.send(Event::Indeterminate {
            index: self.index,
            msg: msg.to_string(),
            total: self.total,
        });
    }

    /// Announce this download item as determinate with the given total size,
    /// marking it as the current view so a following [`Self::update`] advances
    /// it instead of rebuilding.
    pub(crate) fn start_determinate(&mut self, msg: &str, size: u64) {
        self.has_determinate = true;
        self.last_total = Some(size);
        let _ = self.tx.send(Event::Determinate {
            index: self.index,
            total: self.total,
            msg: msg.to_string(),
            size,
        });
    }

    /// Advance the per-file progress bar by `n`; consumers advance the
    /// global bar in response to the same event. Used by helpers that report
    /// progress without tracking it in [`Self::position`] (e.g. the checksum
    /// helper while seeding a resume file).
    pub(super) fn advance(&self, n: u64) {
        let _ = self.tx.send(Event::Advance {
            index: self.index,
            size: n,
        });
    }

    /// Finish and clear the per-file progress bar, undoing `sub` bytes from
    /// the global bar (0 when the download succeeded).
    fn done(&self, sub: u64) {
        let _ = self.tx.send(Event::Cleared {
            index: self.index,
            sub,
        });
    }
}

impl Drop for ProgressReporter {
    fn drop(&mut self) {
        if self.finished {
            return;
        }
        // Failure or early return: clear the per-file bar and undo the bytes
        // this attempt reported to the global bar.
        self.done(self.position);
    }
}

/// Mutable state of one HTTP download attempt: where we are in the file and
/// how much the server told us. Progress bar bookkeeping lives in
/// [`ProgressReporter`], not here.
pub(crate) struct DownloadState {
    /// Bytes logically present in the destination file. Doubles as the HTTP
    /// resume offset and the file seek offset: the two coincide because
    /// compressed downloads always restart from offset 0 (see
    /// [`Self::begin_attempt`]).
    pub(crate) downloaded_size: u64,
    /// Offset at the start of the current body segment (the end of the
    /// previous body, or the seeded resume offset). Used to detect segment
    /// transitions (hasher refresh) and stalled downloads.
    pub(crate) prev_size: u64,
    /// Total download size, when the server provides one.
    pub(crate) total_size: Option<u64>,
}

impl DownloadState {
    pub(crate) fn new() -> Self {
        Self {
            downloaded_size: 0,
            prev_size: 0,
            total_size: None,
        }
    }

    /// Loop condition: keep downloading until we've reached the known total.
    pub(crate) fn in_progress(&self) -> bool {
        match self.total_size {
            Some(total) => self.downloaded_size < total,
            None => true,
        }
    }

    /// Start a request attempt: restart from offset 0 when resume is
    /// disallowed or the content is compressed, so the offset stays equal to
    /// both the HTTP resume offset and the file offset.
    pub(crate) fn begin_attempt(&mut self, allow_resume: bool, file_type: CompressType) {
        if !allow_resume || file_type != CompressType::None {
            self.downloaded_size = 0;
        }
    }

    /// Give up on the current offset and download from the beginning.
    pub(crate) fn restart(&mut self) {
        self.downloaded_size = 0;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::CompressType;

    /// A reporter bound to a fresh channel; `events()` drains what it sent.
    fn reporter() -> (ProgressReporter, flume::Receiver<Event>) {
        let (tx, rx) = flume::unbounded();
        (ProgressReporter::new(&tx, 3, 10), rx)
    }

    fn events(rx: &flume::Receiver<Event>) -> Vec<Event> {
        rx.drain().collect()
    }

    /// Project an event onto a comparable `(kind, index, bytes)` triple so
    /// tests can `assert_eq!` without `Event: PartialEq` (it embeds
    /// [`SingleDownloadError`], which isn't).
    fn triple(e: &Event) -> (&'static str, usize, Option<u64>) {
        match e {
            Event::Cleared { index, sub } => ("cleared", *index, Some(*sub)),
            Event::Indeterminate { index, .. } => ("indeterminate", *index, None),
            Event::Determinate { index, size, .. } => ("determinate", *index, Some(*size)),
            Event::Advance { index, size } => ("advance", *index, Some(*size)),
            other => panic!("unexpected event {other:?}"),
        }
    }

    #[test]
    fn start_indeterminate_announces_item() {
        let (progress, rx) = reporter();
        progress.start_indeterminate("pkg");
        let events = events(&rx);
        assert_eq!(events.len(), 1);
        assert!(matches!(
            &events[0],
            Event::Indeterminate { index: 3, total: 10, msg } if msg == "pkg"
        ));
    }

    #[test]
    fn update_creates_bar_and_advances_on_first_call() {
        let (mut progress, rx) = reporter();
        progress.update("pkg", 100, Some(1000));
        assert_eq!(
            events(&rx).iter().map(triple).collect::<Vec<_>>(),
            vec![
                ("cleared", 3, Some(0)),
                ("determinate", 3, Some(1000)),
                ("advance", 3, Some(100)),
            ]
        );
    }

    #[test]
    fn update_advances_by_delta() {
        let (mut progress, rx) = reporter();
        progress.update("pkg", 100, Some(1000));
        events(&rx);
        progress.update("pkg", 250, Some(1000));
        assert_eq!(
            events(&rx).iter().map(triple).collect::<Vec<_>>(),
            vec![("advance", 3, Some(150))]
        );
    }

    #[test]
    fn update_recreates_bar_when_offset_moves_backwards() {
        let (mut progress, rx) = reporter();
        progress.update("pkg", 100, Some(1000));
        events(&rx);
        progress.update("pkg", 40, Some(1000));
        assert_eq!(
            events(&rx).iter().map(triple).collect::<Vec<_>>(),
            vec![
                ("cleared", 3, Some(100)),
                ("determinate", 3, Some(1000)),
                ("advance", 3, Some(40)),
            ]
        );
    }

    #[test]
    fn update_recreates_bar_when_total_changes() {
        let (mut progress, rx) = reporter();
        progress.update("pkg", 100, Some(1000));
        events(&rx);
        progress.update("pkg", 120, Some(2000));
        assert_eq!(
            events(&rx).iter().map(triple).collect::<Vec<_>>(),
            vec![
                ("cleared", 3, Some(100)),
                ("determinate", 3, Some(2000)),
                ("advance", 3, Some(120)),
            ]
        );
    }

    #[test]
    fn finish_keeps_global_bytes() {
        let (mut progress, rx) = reporter();
        progress.update("pkg", 100, Some(1000));
        events(&rx);
        progress.finish();
        drop(progress); // must not emit anything extra
        assert_eq!(
            events(&rx).iter().map(triple).collect::<Vec<_>>(),
            vec![("cleared", 3, Some(0))]
        );
    }

    #[test]
    fn drop_undoes_reported_bytes_on_failure() {
        let (mut progress, rx) = reporter();
        progress.update("pkg", 100, Some(1000));
        events(&rx);
        drop(progress);
        assert_eq!(
            events(&rx).iter().map(triple).collect::<Vec<_>>(),
            vec![("cleared", 3, Some(100))]
        );
    }

    #[test]
    fn set_position_is_undone_on_drop() {
        let (mut progress, rx) = reporter();
        progress.set_position(50);
        drop(progress);
        assert_eq!(
            events(&rx).iter().map(triple).collect::<Vec<_>>(),
            vec![("cleared", 3, Some(50))]
        );
    }

    #[test]
    fn in_progress_until_total_reached() {
        let mut state = DownloadState::new();
        assert!(state.in_progress()); // no known total yet

        state.total_size = Some(100);
        state.downloaded_size = 50;
        assert!(state.in_progress());
        state.downloaded_size = 100;
        assert!(!state.in_progress());
    }

    #[test]
    fn begin_attempt_keeps_offset_when_resume_allowed() {
        let mut state = DownloadState::new();
        state.total_size = Some(100);
        state.downloaded_size = 42;
        state.begin_attempt(true, CompressType::None);
        assert_eq!(state.downloaded_size, 42);
    }

    #[test]
    fn begin_attempt_restarts_when_resume_disallowed() {
        let mut state = DownloadState::new();
        state.downloaded_size = 42;
        state.begin_attempt(false, CompressType::None);
        assert_eq!(state.downloaded_size, 0);
    }

    #[test]
    fn begin_attempt_restarts_for_compressed() {
        let mut state = DownloadState::new();
        state.downloaded_size = 42;
        state.begin_attempt(true, CompressType::Xz);
        assert_eq!(state.downloaded_size, 0);
    }

    #[test]
    fn restart_zeroes_offset() {
        let mut state = DownloadState::new();
        state.downloaded_size = 99;
        state.restart();
        assert_eq!(state.downloaded_size, 0);
    }
}
