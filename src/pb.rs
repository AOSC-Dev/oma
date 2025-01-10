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
    writer::{gen_prefix, writeln_inner, MessageType, Writeln},
};
use oma_fetch::{Event, SingleDownloadError};
use reqwest::StatusCode;

use crate::{color_formatter, error::OutputError};
use crate::{error::Chain, fl, msg, utils::is_root, WRITER};
use oma_refresh::db::Event as RefreshEvent;
use oma_utils::human_bytes::HumanBytes;
use tracing::{debug, error, info, warn};

pub trait RenderDownloadProgress {
    fn render_progress(&mut self, rx: &flume::Receiver<Event>);
}

pub trait RenderRefreshProgress {
    fn render_refresh_progress(&mut self, rx: &flume::Receiver<RefreshEvent>);
}

pub struct OmaProgressBar {
    pub inner: ProgressBar,
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

impl Writeln for OmaProgressBar {
    fn writeln(&self, prefix: &str, msg: &str) -> std::io::Result<()> {
        let max_len = WRITER.get_max_len();
        let mut output = (None, None);

        writeln_inner(msg, prefix, max_len as usize, WRITER.prefix_len, |t, s| {
            match t {
                MessageType::Msg => {
                    let s: Box<str> = Box::from(s);
                    output.1 = Some(s)
                }
                MessageType::Prefix => output.0 = Some(gen_prefix(s, 10)),
            }

            if let (Some(prefix), Some(msg)) = &output {
                self.inner.println(format!("{prefix}{msg}"));
                output = (None, None);
            }
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

impl Writeln for OmaMultiProgressBar {
    fn writeln(&self, prefix: &str, msg: &str) -> std::io::Result<()> {
        let max_len = WRITER.get_max_len();
        let mut output = (None, None);

        let mut result = Ok(());

        writeln_inner(msg, prefix, max_len as usize, WRITER.prefix_len, |t, s| {
            match t {
                MessageType::Msg => {
                    let s: Box<str> = Box::from(s);
                    output.1 = Some(s)
                }
                MessageType::Prefix => output.0 = Some(gen_prefix(s, 10)),
            }

            if let (Some(prefix), Some(msg)) = &output {
                result = self.mb.println(format!("{prefix}{msg}"));
                output = (None, None);
            }
        });

        result
    }
}

impl RenderRefreshProgress for OmaMultiProgressBar {
    fn render_refresh_progress(&mut self, rx: &flume::Receiver<RefreshEvent>) {
        while let Ok(event) = rx.recv() {
            match event {
                RefreshEvent::DownloadEvent(event) => {
                    self.download_event(event);
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
                    self.writeln(
                        &style("INFO").blue().bold().to_string(),
                        &fl!("scan-topic-is-removed", name = topic),
                    )
                    .ok();
                }
                RefreshEvent::TopicNotInMirror { topic, mirror } => {
                    self.writeln(
                        &style("WARNING").yellow().bold().to_string(),
                        &fl!("topic-not-in-mirror", topic = topic, mirror = mirror),
                    )
                    .ok();
                    self.writeln(
                        &style("WARNING").yellow().bold().to_string(),
                        &fl!("skip-write-mirror"),
                    )
                    .ok();
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
                    self.writeln(
                        &style("WARNING").yellow().bold().to_string(),
                        &fl!(
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
                        ),
                    )
                    .ok();
                }
            }
        }
    }
}

impl RenderDownloadProgress for OmaMultiProgressBar {
    fn render_progress(&mut self, rx: &flume::Receiver<Event>) {
        while let Ok(event) = rx.recv() {
            if self.download_event(event) {
                break;
            }
        }
    }
}

impl OmaMultiProgressBar {
    fn download_event(&mut self, event: Event) -> bool {
        match event {
            Event::ChecksumMismatch {
                index: _,
                filename,
                times,
            } => {
                self.writeln(
                    &style("ERROR").red().bold().to_string(),
                    &fl!("checksum-mismatch-retry", c = filename, retry = times),
                )
                .ok();
            }
            Event::GlobalProgressSet(num) => {
                if let Some(gpb) = self.pb_map.get(&0) {
                    gpb.set_position(num);
                }
            }
            Event::ProgressDone(index) => {
                if let Some(pb) = self.pb_map.get(&(index + 1)) {
                    pb.finish_and_clear();
                }
            }
            Event::NewProgressSpinner { index, msg } => {
                let (sty, inv) = spinner_style();
                let pb = self
                    .mb
                    .insert(index + 1, ProgressBar::new_spinner().with_style(sty));
                pb.set_message(msg);
                pb.enable_steady_tick(inv);
                self.pb_map.insert(index + 1, pb);
            }
            Event::NewProgressBar { index, msg, size } => {
                let sty = progress_bar_style(&WRITER);
                let pb = self
                    .mb
                    .insert(index + 1, ProgressBar::new(size).with_style(sty));
                pb.set_message(msg);
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
                self.handle_download_err(file_name, err);
                self.writeln(
                    &style("INFO").blue().bold().to_string(),
                    &fl!("can-not-get-source-next-url"),
                )
                .ok();
            }
            Event::DownloadDone { index: _, msg } => {
                tracing::debug!("Downloaded {msg}");
            }
            Event::AllDone => {
                if let Some(gpb) = &self.pb_map.get(&0) {
                    gpb.finish_and_clear();
                }
                return true;
            }
            Event::NewGlobalProgressBar(total_size) => {
                let sty = global_progress_bar_style(&WRITER);
                let pb = self
                    .mb
                    .insert(0, ProgressBar::new(total_size).with_style(sty));
                self.pb_map.insert(0, pb);
            }
            Event::Failed { file_name, error } => {
                self.handle_download_err(file_name, error);
            }
        };

        false
    }

    fn handle_download_err(&mut self, file_name: String, error: SingleDownloadError) {
        if let SingleDownloadError::ReqwestError { ref source } = error {
            if source
                .status()
                .is_some_and(|x| x == StatusCode::UNAUTHORIZED)
            {
                if !is_root() {
                    self.writeln(
                        &style("INFO").blue().bold().to_string(),
                        &fl!("auth-need-permission"),
                    )
                    .ok();
                } else {
                    self.writeln(
                        &style("INFO").blue().bold().to_string(),
                        &fl!("lack-auth-config-1"),
                    )
                    .ok();
                    self.writeln(
                        &style("INFO").blue().bold().to_string(),
                        &fl!("lack-auth-config-2"),
                    )
                    .ok();
                }
            }

            let err = OutputError::from(error);
            let errs = Chain::new(&err).collect::<Vec<_>>();
            let first_cause = errs.first().unwrap().to_string();
            let last = errs.iter().skip(1).last();

            if let Some(last_cause) = last {
                let reason = format!("{}: {}", first_cause, last_cause);
                self.writeln(
                    &style("ERROR").red().bold().to_string(),
                    &fl!(
                        "download-package-failed-with-reason",
                        filename = file_name,
                        reason = reason
                    ),
                )
                .ok();
            } else {
                self.writeln(
                    &style("ERROR").red().bold().to_string(),
                    &fl!("download-failed", filename = file_name),
                )
                .ok();
            }

            debug!("{:#?}", errs);
        }
    }
}

impl Default for NoProgressBar {
    fn default() -> Self {
        Self {
            timer: Instant::now(),
            total_size: OnceCell::new(),
            old_downloaded: 0,
        }
    }
}

pub struct NoProgressBar {
    timer: Instant,
    total_size: OnceCell<u64>,
    old_downloaded: u64,
}

impl RenderDownloadProgress for NoProgressBar {
    fn render_progress(&mut self, rx: &flume::Receiver<Event>) {
        while let Ok(event) = rx.recv() {
            if self.download_event(event) {
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
                    self.download_event(event);
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
    fn download_event(&mut self, event: Event) -> bool {
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
            Event::GlobalProgressSet(downloaded) => {
                let elapsed = self.timer.elapsed();
                if elapsed >= Duration::from_secs(3) {
                    if let Some(total_size) = self.total_size.get() {
                        msg!(
                            "{} / {} ({}/s)",
                            HumanBytes(downloaded),
                            HumanBytes(*total_size),
                            HumanBytes(downloaded - self.old_downloaded / elapsed.as_secs())
                        );
                        self.old_downloaded = downloaded;
                    } else {
                        msg!("Downloaded {}", HumanBytes(downloaded));
                    }
                    self.timer = Instant::now();
                }
            }
            Event::NextUrl {
                index: _,
                file_name,
                err,
            } => {
                handle_no_pb_download_error(file_name, err);
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
                handle_no_pb_download_error(file_name, error);
            }
            _ => {}
        };

        false
    }
}

fn handle_no_pb_download_error(file_name: String, error: SingleDownloadError) {
    if let SingleDownloadError::ReqwestError { ref source } = error {
        if source
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
    }

    let err = OutputError::from(error);
    let errs = Chain::new(&err).collect::<Vec<_>>();
    let first_cause = errs.first().unwrap().to_string();
    let last = errs.iter().skip(1).last();

    if let Some(last_cause) = last {
        let reason = format!("{}: {}", first_cause, last_cause);
        error!(
            "{}",
            fl!(
                "download-package-failed-with-reason",
                filename = file_name,
                reason = reason
            )
        );
    } else {
        error!("{}", fl!("download-failed", filename = file_name));
    }
}
