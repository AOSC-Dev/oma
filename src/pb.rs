use std::{
    borrow::Cow,
    cell::OnceCell,
    io::{self, IsTerminal, Write},
    ops::Deref,
    sync::LazyLock,
    time::{Duration, Instant},
};

use ahash::{HashMap, RandomState};
use anyhow::Chain;
use oma_console::{
    indicatif::{MultiProgress, ProgressBar as IndicatifProgressBar, ProgressStyle},
    pb::{global_progress_bar_style, progress_bar_style, spinner_style},
    print::Action,
};
use oma_fetch::{Event, SingleDownloadError};
use reqwest::StatusCode;

use crate::{WRITER, fl, install_progress::OSC94, msg, root::is_root};
use crate::{color_formatter, error::OutputError};
use oma_refresh::db::Event as RefreshEvent;

use oma_utils::human_bytes::HumanBytes;
use spdlog::{error, info, warn};

/// The global `MultiProgress` every progress bar is attached to. When no bar
/// is active, [`MultiProgress::println`] falls back to printing the line
/// directly to the terminal.
static GLOBAL_MP: LazyLock<MultiProgress> = LazyLock::new(MultiProgress::new);

/// A progress bar that is automatically attached to the global `MultiProgress`
/// when created, so callers never touch the underlying `MultiProgress`
/// directly (mirroring `tracing-indicatif`). All operations delegate to the
/// wrapped indicatif bar via [`Deref`]; when progress is disabled a hidden
/// no-op bar is returned instead, so callers never have to handle an `Option`.
pub struct ProgressBar {
    inner: IndicatifProgressBar,
}

impl ProgressBar {
    /// Create a spinner progress bar and attach it to the global
    /// `MultiProgress`. When `enabled` is false a hidden no-op bar is returned
    /// instead.
    pub fn new_spinner(msg: impl Into<Cow<'static, str>>, enabled: bool) -> Self {
        if !enabled {
            return Self::hidden();
        }

        // Mount before configuring: drawing a bar before it is added to the
        // `MultiProgress` writes straight to the terminal, which the
        // `MultiProgress` cannot track or undo.
        let pb = GLOBAL_MP.add(IndicatifProgressBar::new_spinner());
        let (sty, inv) = spinner_style();
        pb.set_style(sty);
        pb.enable_steady_tick(inv);
        pb.set_message(msg);

        Self { inner: pb }
    }

    /// Create a determinate progress bar with a custom style and attach it to
    /// the global `MultiProgress`. When `enabled` is false a hidden no-op bar
    /// is returned instead.
    pub fn new(len: u64, style: ProgressStyle, enabled: bool) -> Self {
        if !enabled {
            return Self::hidden();
        }

        let pb = GLOBAL_MP.add(IndicatifProgressBar::new(len));
        pb.set_style(style);

        Self { inner: pb }
    }

    /// Create a spinner progress bar and insert it at a specific position,
    /// used by the multi-bar download/refresh renderer to keep ordering.
    pub(crate) fn insert_spinner(at: usize, msg: impl Into<Cow<'static, str>>) -> Self {
        let pb = GLOBAL_MP.insert(at, IndicatifProgressBar::new_spinner());
        let (sty, inv) = spinner_style();
        pb.set_style(sty);
        pb.enable_steady_tick(inv);
        pb.set_message(msg);

        Self { inner: pb }
    }

    /// Create a determinate progress bar and insert it at a specific position.
    pub(crate) fn insert_bar(at: usize, len: u64, style: ProgressStyle) -> Self {
        let pb = GLOBAL_MP.insert(at, IndicatifProgressBar::new(len));
        pb.set_style(style);

        Self { inner: pb }
    }

    /// Finish this bar and remove it from the global `MultiProgress`. Safe to
    /// call more than once (e.g. from an explicit call site and again from
    /// `Drop`): after the first call the bar is finished, so later calls are
    /// no-ops.
    pub fn finish_and_clear(&self) {
        if self.inner.is_finished() {
            return;
        }
        self.inner.finish_and_clear();
        GLOBAL_MP.remove(&self.inner);
    }

    fn hidden() -> Self {
        Self {
            inner: IndicatifProgressBar::hidden(),
        }
    }
}

impl Deref for ProgressBar {
    type Target = IndicatifProgressBar;

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

impl Drop for ProgressBar {
    fn drop(&mut self) {
        self.finish_and_clear();
    }
}

/// A `Write` that renders log lines above the active progress bars when the
/// terminal supports them, and to stderr otherwise.
#[derive(Default)]
pub struct ProgressAwareWriter;

impl Write for ProgressAwareWriter {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        if io::stderr().is_terminal() {
            let line = String::from_utf8_lossy(buf)
                .trim_end_matches('\n')
                .to_string();
            GLOBAL_MP.println(line)?;
        } else {
            io::stderr().write_all(buf)?;
        }

        Ok(buf.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        io::stderr().flush()
    }
}

/// A sink that renders download progress events. [`ProgressRenderer`] picks a
/// concrete sink based on `--no-progress`: interactive bars on the global
/// `MultiProgress`, or plain textual progress lines.
pub trait ProgressSink {
    fn new_global_bar(&mut self, total_size: u64);
    fn global_add(&mut self, num: u64);
    fn global_sub(&mut self, num: u64);
    fn new_spinner(&mut self, index: usize, total: usize, msg: String);
    fn new_bar(&mut self, index: usize, total: usize, msg: String, size: u64);
    fn inc(&mut self, index: usize, size: u64);
    fn done(&mut self, index: usize);
    fn refresh_spinner(&mut self, msg: String);
    fn finish_all(&mut self);
    /// Current global (position, length), used for the OSC 94 title update.
    fn position_len(&self) -> Option<(u64, u64)>;
}

/// Progress sink that renders interactive bars on the global `MultiProgress`.
struct BarProgress {
    pb_map: HashMap<usize, ProgressBar>,
}

impl BarProgress {
    fn new() -> Self {
        Self {
            pb_map: HashMap::with_hasher(RandomState::new()),
        }
    }
}

impl ProgressSink for BarProgress {
    fn new_global_bar(&mut self, total_size: u64) {
        let pb = ProgressBar::insert_bar(
            0,
            total_size,
            global_progress_bar_style(WRITER.get_length()),
        );
        self.pb_map.insert(0, pb);
    }

    fn global_add(&mut self, num: u64) {
        if let Some(gpb) = self.pb_map.get(&0) {
            gpb.inc(num);
        }
    }

    fn global_sub(&mut self, num: u64) {
        if let Some(gpb) = self.pb_map.get(&0) {
            gpb.set_position(gpb.position().saturating_sub(num));
        }
    }

    fn new_spinner(&mut self, index: usize, total: usize, msg: String) {
        // A previous attempt may have left a bar at this slot without a
        // ProgressDone (e.g. an early request-phase failure): drop it before
        // inserting so the ordering stays 1:1 with files and retries do not
        // pile up dead bars.
        if let Some(old) = self.pb_map.remove(&(index + 1)) {
            old.finish_and_clear();
        }
        let total_width = total_width(total);
        let pb = ProgressBar::insert_spinner(
            index + 1,
            format!("({:>total_width$}/{total}) {msg}", index + 1),
        );
        self.pb_map.insert(index + 1, pb);
    }

    fn new_bar(&mut self, index: usize, total: usize, msg: String, size: u64) {
        // See new_spinner: clear any leftover bar at this slot before
        // inserting so retries do not accumulate dead bars.
        if let Some(old) = self.pb_map.remove(&(index + 1)) {
            old.finish_and_clear();
        }
        let pb = ProgressBar::insert_bar(index + 1, size, progress_bar_style(WRITER.get_length()));
        let total_width = total_width(total);
        pb.set_message(format!("({:>total_width$}/{total}) {msg}", index + 1));
        self.pb_map.insert(index + 1, pb);
    }

    fn inc(&mut self, index: usize, size: u64) {
        if let Some(pb) = self.pb_map.get(&(index + 1)) {
            pb.inc(size);
        }
    }

    fn done(&mut self, index: usize) {
        if let Some(pb) = self.pb_map.remove(&(index + 1)) {
            pb.finish_and_clear();
        }
    }

    fn refresh_spinner(&mut self, msg: String) {
        let pb = ProgressBar::insert_spinner(1, msg);
        self.pb_map.insert(1, pb);
    }

    fn finish_all(&mut self) {
        // Finish and remove every bar still mounted (e.g. a download that
        // errored before sending `ProgressDone`) so nothing is left on screen.
        for (_, pb) in self.pb_map.drain() {
            pb.finish_and_clear();
        }
    }

    fn position_len(&self) -> Option<(u64, u64)> {
        self.pb_map
            .get(&0)
            .and_then(|gpb| gpb.length().map(|len| (gpb.position(), len)))
    }
}

/// Progress sink for `--no-progress` mode: no bars are drawn, download
/// progress is printed as plain lines and refresh stages as log lines.
struct TextProgress {
    timer: Instant,
    total_size: OnceCell<u64>,
    old_downloaded: u64,
    progress: u64,
}

impl TextProgress {
    fn new() -> Self {
        Self {
            timer: Instant::now(),
            total_size: OnceCell::new(),
            old_downloaded: 0,
            progress: 0,
        }
    }

    fn print(&mut self) {
        let elapsed = self.timer.elapsed();
        if elapsed >= Duration::from_secs(3) {
            if let Some(total_size) = self.total_size.get() {
                msg!(
                    "{} / {} ({}/s)",
                    HumanBytes(self.progress),
                    HumanBytes(*total_size),
                    HumanBytes((self.progress - self.old_downloaded) / elapsed.as_secs())
                );
                self.old_downloaded = self.progress;
            } else {
                msg!("Downloaded {}", HumanBytes(self.progress));
            }
            self.timer = Instant::now();
        }
    }
}

impl ProgressSink for TextProgress {
    fn new_global_bar(&mut self, total_size: u64) {
        self.total_size.get_or_init(|| total_size);
    }

    fn global_add(&mut self, num: u64) {
        self.progress += num;
        self.print();
    }

    fn global_sub(&mut self, num: u64) {
        self.progress = self.progress.saturating_sub(num);
        self.old_downloaded = self.old_downloaded.saturating_sub(num);
        self.print();
    }

    fn new_spinner(&mut self, _index: usize, _total: usize, _msg: String) {}

    fn new_bar(&mut self, _index: usize, _total: usize, _msg: String, _size: u64) {}

    fn inc(&mut self, _index: usize, _size: u64) {}

    fn done(&mut self, _index: usize) {}

    fn refresh_spinner(&mut self, msg: String) {
        info!("{}", msg);
    }

    fn finish_all(&mut self) {}

    fn position_len(&self) -> Option<(u64, u64)> {
        None
    }
}

/// Renders download/refresh progress events by dispatching them to a
/// [`ProgressSink`], chosen at construction based on `--no-progress`.
pub struct ProgressRenderer {
    sink: Box<dyn ProgressSink + Send>,
}

impl ProgressRenderer {
    pub fn new(no_progress: bool) -> Self {
        if no_progress {
            Self {
                sink: Box::new(TextProgress::new()),
            }
        } else {
            Self {
                sink: Box::new(BarProgress::new()),
            }
        }
    }
}

impl ProgressRenderer {
    pub(crate) fn render_refresh_progress(&mut self, rx: &flume::Receiver<RefreshEvent>) {
        while let Ok(event) = rx.recv() {
            match event {
                RefreshEvent::DownloadEvent(event) => {
                    self.download_event(event, true, false);
                }
                RefreshEvent::ScanningTopic => {
                    self.sink.refresh_spinner(fl!("refreshing-topic-metadata"));
                }
                RefreshEvent::ClosingTopic(topic) => {
                    info!("{}", fl!("scan-topic-is-removed", name = topic));
                }
                RefreshEvent::TopicNotInMirror { topic, mirror } => {
                    warn!(
                        "{}",
                        fl!("topic-not-in-mirror", topic = topic, mirror = mirror)
                    );
                    warn!("{}", fl!("skip-write-mirror"));
                }
                RefreshEvent::RunInvokeScript => {
                    self.sink.refresh_spinner(fl!("oma-refresh-success-invoke"));
                }
                RefreshEvent::Done => {
                    self.sink.finish_all();
                    break;
                }
                RefreshEvent::SourceListFileNotSupport { path } => {
                    warn!(
                        "{}",
                        fl!(
                            "unsupported-sources-list",
                            p = color_formatter()
                                .color_str(path.to_string_lossy(), Action::Emphasis)
                                .to_string(),
                            list = color_formatter()
                                .color_str(".list", Action::Secondary)
                                .to_string(),
                            sources = color_formatter()
                                .color_str(".sources", Action::Secondary)
                                .to_string()
                        )
                    );
                }
            }
        }
    }
}

impl ProgressRenderer {
    pub(crate) fn render_progress(&mut self, rx: &flume::Receiver<Event>, download_only: bool) {
        while let Ok(event) = rx.recv() {
            if self.download_event(event, false, download_only) {
                break;
            }
        }
    }
}

impl ProgressRenderer {
    /// Report the download phase as terminal percentage progress (OSC 94).
    fn update_osc94(&self, is_refresh: bool, download_only: bool) {
        if let Some((pos, len)) = self.sink.position_len() {
            osc94(is_refresh, download_only, pos, len);
        }
    }

    fn download_event(&mut self, event: Event, is_refresh: bool, download_only: bool) -> bool {
        match event {
            Event::ChecksumMismatch {
                index: _,
                filename,
                times,
            } => {
                error!(
                    "{}",
                    fl!("checksum-mismatch-retry", c = filename, retry = times)
                );
            }
            Event::GlobalProgressAdd(num) => {
                self.sink.global_add(num);
                self.update_osc94(is_refresh, download_only);
            }
            Event::GlobalProgressSub(num) => {
                self.sink.global_sub(num);
                self.update_osc94(is_refresh, download_only);
            }
            Event::ProgressDone(index) => self.sink.done(index),
            Event::NewProgressSpinner { index, total, msg } => {
                self.sink.new_spinner(index, total, msg);
            }
            Event::NewProgressBar {
                index,
                total,
                msg,
                size,
            } => {
                self.sink.new_bar(index, total, msg, size);
            }
            Event::ProgressInc { index, size } => self.sink.inc(index, size),
            Event::NextUrl {
                index: _,
                file_name,
                err,
            } => {
                handle_download_error(file_name, is_refresh, err);
                info!("{}", fl!("can-not-get-source-next-url"));
            }
            Event::DownloadDone { index, msg } => {
                spdlog::debug!("Downloaded {msg}");
                self.sink.done(index);
            }
            Event::AllDone => {
                self.sink.finish_all();
                if download_only {
                    OSC94.finish();
                } else if !is_refresh {
                    // Hand the progress reporting over to the dpkg phase: the
                    // download phase occupies 0-50%, the dpkg phase 50-100%.
                    OSC94.set(50.0);
                }
                return true;
            }
            Event::NewGlobalProgressBar(total_size) => self.sink.new_global_bar(total_size),
            Event::Failed { file_name, error } => {
                handle_download_error(file_name, is_refresh, error);
            }
            Event::Timeout { filename, times } => {
                error!("{}", fl!("timeout-retry", c = filename, retry = times));
            }
        };

        false
    }
}

#[inline]
fn total_width(total: usize) -> usize {
    total.to_string().len()
}

/// Report download progress as terminal percentage progress (OSC 94), used by
/// `oma install` and `oma download`. The download phase occupies 0-50% of the
/// progress for installs (the dpkg phase fills the remaining 50-100% via
/// `OmaInstallProgressManager`) and 0-100% for `oma download`. Refresh never
/// reports anything.
fn osc94(is_refresh: bool, download_only: bool, pos: u64, len: u64) {
    if is_refresh || len == 0 {
        return;
    }
    let mut percent = (pos as f32 / len as f32) * 100.0;
    if !download_only {
        percent *= 0.5;
    }
    OSC94.set(percent.clamp(0.0, 100.0));
}

fn handle_download_error(file_name: String, is_refresh: bool, error: SingleDownloadError) {
    if let SingleDownloadError::ReqwestMiddlewareError { ref source } = error
        && source
            .status()
            .is_some_and(|x| x == StatusCode::UNAUTHORIZED)
    {
        if !is_root() {
            info!("{}", fl!("auth-need-permission"));
        } else {
            info!("{}", fl!("lack-auth-config-1"));
            info!("{}", fl!("lack-auth-config-2"));
        }
    }

    let err = OutputError::from(error);
    let errs = Chain::new(&err).collect::<Vec<_>>();
    let first_cause = errs.first().unwrap().to_string();
    let last = errs.iter().skip(1).last();

    if let Some(last_cause) = last {
        let reason = format!("{first_cause}: {last_cause}");

        if is_refresh {
            error!(
                "{}",
                fl!(
                    "download-file-failed-with-reason",
                    filename = file_name,
                    reason = reason
                )
            );
        } else {
            error!(
                "{}",
                fl!(
                    "download-package-failed-with-reason",
                    filename = file_name,
                    reason = reason
                )
            );
        }
    } else if is_refresh {
        error!(
            "{}",
            fl!(
                "download-file-failed-with-reason",
                filename = file_name,
                reason = first_cause
            )
        );
    } else {
        error!(
            "{}",
            fl!(
                "download-package-failed-with-reason",
                filename = file_name,
                reason = first_cause
            )
        );
    }
}
