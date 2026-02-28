use std::{
    borrow::Cow,
    cell::OnceCell,
    time::{Duration, Instant},
};

use ahash::{HashMap, RandomState};
use oma_console::{
    console::style,
    indicatif::{MultiProgress, ProgressBar},
    pb::{global_progress_bar_style, progress_bar_style, spinner_style},
    print::Action,
    terminal::gen_prefix,
    writer::Writeln,
};
use oma_fetch::{Event, SingleDownloadError};
use reqwest::StatusCode;

use crate::{
    LATENCY, WRITER, error::Chain, fl, install_progress::osc94_progress, msg, utils::is_root,
};
use crate::{color_formatter, error::OutputError};
use oma_refresh::db::Event as RefreshEvent;
use oma_utils::human_bytes::HumanBytes;
use spdlog::{debug, error, info, warn};

pub trait RenderPackagesDownloadProgress {
    fn render_progress(&mut self, rx: &flume::Receiver<Event>, download_only: bool);
}

pub trait RenderRefreshProgress {
    fn render_refresh_progress(&mut self, rx: &flume::Receiver<RefreshEvent>);
}

pub trait Print {
    fn info(&self, msg: &str);
    fn warn(&self, msg: &str);
    fn error(&self, msg: &str);
}

pub struct OmaProgressBar {
    pub inner: ProgressBar,
}

impl Print for OmaProgressBar {
    #[inline]
    fn info(&self, msg: &str) {
        self.writeln(&style("INFO").blue().bold().to_string(), msg)
            .ok();
    }

    #[inline]
    fn warn(&self, msg: &str) {
        self.writeln(&style("WARNING").yellow().bold().to_string(), msg)
            .ok();
    }

    #[inline]
    fn error(&self, msg: &str) {
        self.writeln(&style("ERROR").red().bold().to_string(), msg)
            .ok();
    }
}

impl OmaProgressBar {
    pub fn new_spinner(spinner_message: Option<impl Into<Cow<'static, str>>>) -> Self {
        let pb = ProgressBar::new_spinner();
        let (sty, inv) = spinner_style();
        pb.set_style(sty);
        pb.enable_steady_tick(inv);

        if let Some(msg) = spinner_message {
            pb.set_message(msg);
        }

        Self { inner: pb }
    }

    #[allow(dead_code)]
    pub fn new(pb: ProgressBar) -> Self {
        Self { inner: pb }
    }
}

impl Drop for OmaProgressBar {
    fn drop(&mut self) {
        self.inner.finish_and_clear();
    }
}

impl Writeln for OmaProgressBar {
    fn writeln(&self, prefix: &str, msg: &str) -> std::io::Result<()> {
        WRITER
            .get_terminal()
            .wrap_content(prefix, msg)
            .into_iter()
            .for_each(|(prefix, body)| {
                self.inner
                    .println(format!("{}{}", &gen_prefix(prefix, 10), &body));
            });

        Ok(())
    }
}

pub struct OmaMultiProgressBar {
    mb: MultiProgress,
    pb_map: HashMap<usize, ProgressBar>,
}

impl Default for OmaMultiProgressBar {
    fn default() -> Self {
        Self {
            mb: MultiProgress::new(),
            pb_map: HashMap::with_hasher(RandomState::new()),
        }
    }
}

impl Print for OmaMultiProgressBar {
    #[inline]
    fn info(&self, msg: &str) {
        self.writeln(&style("INFO").blue().bold().to_string(), msg)
            .ok();
    }

    #[inline]
    fn warn(&self, msg: &str) {
        self.writeln(&style("WARNING").yellow().bold().to_string(), msg)
            .ok();
    }

    #[inline]
    fn error(&self, msg: &str) {
        self.writeln(&style("ERROR").red().bold().to_string(), msg)
            .ok();
    }
}

impl Writeln for OmaMultiProgressBar {
    fn writeln(&self, prefix: &str, msg: &str) -> std::io::Result<()> {
        for (prefix, body) in WRITER.get_terminal().wrap_content(prefix, msg).into_iter() {
            self.mb
                .println(format!("{}{}", &gen_prefix(prefix, 10), &body))?;
        }

        Ok(())
    }
}

impl RenderRefreshProgress for OmaMultiProgressBar {
    fn render_refresh_progress(&mut self, rx: &flume::Receiver<RefreshEvent>) {
        while let Ok(event) = rx.recv() {
            match event {
                RefreshEvent::DownloadEvent(event) => {
                    self.download_event(event, true, false);
                }
                RefreshEvent::ScanningTopic => {
                    let (sty, inv) = spinner_style();
                    let pb = self
                        .mb
                        .insert(1, ProgressBar::new_spinner().with_style(sty));
                    pb.set_message(fl!("refreshing-topic-metadata"));
                    pb.enable_steady_tick(inv);
                    self.pb_map.insert(1, pb);
                }
                RefreshEvent::ClosingTopic(topic) => {
                    self.info(&fl!("scan-topic-is-removed", name = topic));
                }
                RefreshEvent::TopicNotInMirror { topic, mirror } => {
                    self.warn(&fl!("topic-not-in-mirror", topic = topic, mirror = mirror));
                    self.warn(&fl!("skip-write-mirror"));
                }
                RefreshEvent::RunInvokeScript => {
                    let (sty, inv) = spinner_style();
                    let pb = self
                        .mb
                        .insert(1, ProgressBar::new_spinner().with_style(sty));
                    pb.set_message(fl!("oma-refresh-success-invoke"));
                    pb.enable_steady_tick(inv);
                    self.pb_map.insert(1, pb);
                }
                RefreshEvent::Done => break,
                RefreshEvent::SourceListFileNotSupport { path } => {
                    self.warn(&fl!(
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
                    ));
                }
            }
        }
    }
}

impl RenderPackagesDownloadProgress for OmaMultiProgressBar {
    fn render_progress(&mut self, rx: &flume::Receiver<Event>, download_only: bool) {
        while let Ok(event) = rx.recv() {
            if self.download_event(event, false, download_only) {
                break;
            }
        }
    }
}

impl OmaMultiProgressBar {
    fn download_event(&mut self, event: Event, is_refresh: bool, download_only: bool) -> bool {
        match event {
            Event::ChecksumMismatch {
                index: _,
                filename,
                times,
            } => {
                self.error(&fl!("checksum-mismatch-retry", c = filename, retry = times));
            }
            Event::GlobalProgressAdd(num) => {
                if let Some(gpb) = self.pb_map.get(&0) {
                    gpb.inc(num);
                    let pos = gpb.position();
                    osc94(is_refresh, download_only, pos, gpb);
                }
            }
            Event::GlobalProgressSub(num) => {
                if let Some(gpb) = self.pb_map.get(&0) {
                    gpb.set_position(gpb.position().saturating_sub(num));
                    osc94(is_refresh, download_only, gpb.position(), gpb);
                }
            }
            Event::ProgressDone(index) => {
                if let Some(pb) = self.pb_map.get(&(index + 1)) {
                    pb.finish_and_clear();
                }
            }
            Event::NewProgressSpinner { index, total, msg } => {
                let (sty, inv) = spinner_style();
                let pb = self
                    .mb
                    .insert(index + 1, ProgressBar::new_spinner().with_style(sty));
                let total_width = total_width(total);
                pb.set_message(format!("({:>total_width$}/{total}) {msg}", index + 1));
                pb.enable_steady_tick(inv);
                self.pb_map.insert(index + 1, pb);
            }
            Event::NewProgressBar {
                index,
                total,
                msg,
                size,
            } => {
                let sty = progress_bar_style(WRITER.get_length());
                let pb = self
                    .mb
                    .insert(index + 1, ProgressBar::new(size).with_style(sty));
                let total_width = total_width(total);
                pb.set_message(format!("({:>total_width$}/{total}) {msg}", index + 1));
                pb.enable_steady_tick(
                    LATENCY
                        .get()
                        .copied()
                        .unwrap_or_else(|| Duration::from_millis(100)),
                );
                self.pb_map.insert(index + 1, pb);
            }
            Event::ProgressInc { index, size } => {
                if let Some(pb) = self.pb_map.get(&(index + 1)) {
                    pb.inc(size);
                }
            }
            Event::NextUrl {
                index: _,
                file_name,
                err,
            } => {
                self.handle_download_err(file_name, is_refresh, err);
                self.info(&fl!("can-not-get-source-next-url"));
            }
            Event::DownloadDone { index: _, msg } => {
                spdlog::debug!("Downloaded {msg}");
            }
            Event::AllDone => {
                if let Some(gpb) = &self.pb_map.get(&0) {
                    gpb.finish_and_clear();
                }
                if download_only {
                    osc94_progress(100.0, true);
                }
                return true;
            }
            Event::NewGlobalProgressBar(total_size) => {
                let sty = global_progress_bar_style(WRITER.get_length());
                let pb = self
                    .mb
                    .insert(0, ProgressBar::new(total_size).with_style(sty));
                pb.enable_steady_tick(
                    LATENCY
                        .get()
                        .copied()
                        .unwrap_or_else(|| Duration::from_millis(100)),
                );
                self.pb_map.insert(0, pb);
            }
            Event::Failed { file_name, error } => {
                self.handle_download_err(file_name, is_refresh, error);
            }
            Event::Timeout { filename, times } => {
                self.error(&fl!("timeout-retry", c = filename, retry = times));
            }
        };

        false
    }

    fn handle_download_err(
        &mut self,
        file_name: String,
        is_refresh: bool,
        error: SingleDownloadError,
    ) {
        if let SingleDownloadError::ReqwestError { ref source } = error
            && source
                .status()
                .is_some_and(|x| x == StatusCode::UNAUTHORIZED)
        {
            if !is_root() {
                self.info(&fl!("auth-need-permission"));
            } else {
                self.info(&fl!("lack-auth-config-1"));
                self.info(&fl!("lack-auth-config-2"));
            }
        }

        let err = OutputError::from(error);
        let errs = Chain::new(&err).collect::<Vec<_>>();
        let first_cause = errs.first().unwrap().to_string();
        let last = errs.iter().skip(1).last();

        if let Some(last_cause) = last {
            let reason = format!("{first_cause}: {last_cause}");
            self.error_display(&file_name, is_refresh, reason);
        } else if is_refresh {
            self.error_display(&file_name, is_refresh, first_cause);
        }

        debug!("{:#?}", errs);
    }

    fn error_display(&mut self, file_name: &String, is_refresh: bool, reason: String) {
        if is_refresh {
            self.error(&fl!(
                "download-file-failed-with-reason",
                filename = file_name,
                reason = reason
            ));
        } else {
            self.error(&fl!(
                "download-package-failed-with-reason",
                filename = file_name,
                reason = reason
            ));
        }
    }
}

#[inline]
fn total_width(total: usize) -> usize {
    total.to_string().len()
}

fn osc94(is_refresh: bool, download_only: bool, pos: u64, gpb: &ProgressBar) {
    if let Some(len) = gpb.length()
        && !is_refresh
    {
        let mut pos = (pos as f32 / len as f32) * 100.0;
        if !download_only {
            pos *= 0.5;
        }
        osc94_progress(pos, false);
    }
}

impl Default for NoProgressBar {
    fn default() -> Self {
        Self {
            timer: Instant::now(),
            total_size: OnceCell::new(),
            old_downloaded: 0,
            progress: 0,
        }
    }
}

pub struct NoProgressBar {
    timer: Instant,
    total_size: OnceCell<u64>,
    old_downloaded: u64,
    progress: u64,
}

impl RenderPackagesDownloadProgress for NoProgressBar {
    fn render_progress(&mut self, rx: &flume::Receiver<Event>, _download_only: bool) {
        while let Ok(event) = rx.recv() {
            if self.download_event(event, false) {
                break;
            }
        }
    }
}

impl RenderRefreshProgress for NoProgressBar {
    fn render_refresh_progress(&mut self, rx: &flume::Receiver<RefreshEvent>) {
        while let Ok(event) = rx.recv() {
            match event {
                RefreshEvent::DownloadEvent(event) => {
                    self.download_event(event, true);
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
                    info!("{}", fl!("oma-refresh-success-invoke"));
                }
                RefreshEvent::Done => break,
                _ => {}
            }
        }
    }
}

impl NoProgressBar {
    fn download_event(&mut self, event: Event, is_refresh: bool) -> bool {
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
            Event::GlobalProgressAdd(inc) => {
                self.progress += inc;
                self.print_progress();
            }
            Event::GlobalProgressSub(num) => {
                self.progress = self.progress.saturating_sub(num);
                self.old_downloaded = self.old_downloaded.saturating_sub(num);
                self.print_progress();
            }
            Event::NextUrl {
                index: _,
                file_name,
                err,
            } => {
                handle_no_pb_download_error(file_name, err, is_refresh);
                info!("{}", fl!("can-not-get-source-next-url"));
            }
            Event::DownloadDone { index: _, msg } => {
                WRITER.writeln("DONE", &msg).ok();
            }
            Event::AllDone => return true,
            Event::NewGlobalProgressBar(total_size) => {
                self.total_size.get_or_init(|| total_size);
            }
            Event::Failed { file_name, error } => {
                handle_no_pb_download_error(file_name, error, is_refresh);
            }
            _ => {}
        };

        false
    }

    fn print_progress(&mut self) {
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

fn handle_no_pb_download_error(file_name: String, error: SingleDownloadError, is_refresh: bool) {
    if let SingleDownloadError::ReqwestError { ref source } = error
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
