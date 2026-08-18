use std::borrow::Cow;
use std::error::Error;
use std::fmt::Display;
use std::io::{self};
use std::path::Path;

use apt_auth_config::AuthConfigError;
use oma_console::writer::{Writeln, Writer};
use oma_contents::OmaContentsError;
use oma_fetch::SingleDownloadError;
use oma_fetch::checksum::ChecksumError;
use oma_fetch::download::BuilderError;
use oma_history::HistoryError;

#[cfg(feature = "aosc")]
use oma_mirror::MirrorError;

use oma_apt_pkg::search::OmaSearchError;
use oma_pm::oma_apt::error::AptErrors;
use oma_pm::pkginfo::PtrIsNone;
use oma_pm::{apt::OmaAptError, matches::MatcherError};
use oma_refresh::db::RefreshError;
use oma_refresh::inrelease::InReleaseError;
use oma_repo_verify::VerifyError;

#[cfg(feature = "aosc")]
use oma_tum::TumError;

use oma_utils::GetLockError;
use oma_utils::dbus::OmaDbusError;
use oma_utils::dpkg::DpkgError;

#[cfg(feature = "aosc")]
use oma_topics::OmaTopicsError;
use spdlog::{debug, error, info};

use crate::{due_to, fl, msg};

/// Top-level error type for all oma operations.
///
/// A thin wrapper around [`anyhow::Error`]: the displayed message is the
/// localized user-facing text, and [`Error::source`] walks the underlying
/// cause chain ("due to") shown on failure.
#[derive(Debug)]
pub struct OutputError(anyhow::Error);

impl From<anyhow::Error> for OutputError {
    fn from(error: anyhow::Error) -> Self {
        OutputError(error)
    }
}

impl Display for OutputError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        Display::fmt(&self.0, f)
    }
}

impl Error for OutputError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        self.0.source()
    }
}

/// Marker error for [`OutputError::already_reported`]: the full details were
/// already printed (e.g. dependency issues that logged each message
/// individually), so nothing more should be displayed.
#[derive(Debug)]
struct AlreadyReported;

impl Display for AlreadyReported {
    fn fmt(&self, _f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        Ok(())
    }
}

impl Error for AlreadyReported {}

impl OutputError {
    /// Wrap a localized message with no underlying cause.
    pub fn msg(message: impl Into<Cow<'static, str>>) -> Self {
        OutputError(anyhow::Error::msg(message.into()))
    }

    /// An error whose details were already printed (e.g. dependency issues
    /// that logged each message individually); the display logic skips it.
    pub fn already_reported() -> Self {
        OutputError(anyhow::Error::new(AlreadyReported))
    }

    /// Whether the full details of this error were already printed elsewhere.
    pub fn is_already_reported(&self) -> bool {
        self.0.is::<AlreadyReported>()
    }

    /// Wrap a localized message over an underlying cause.
    pub fn with_source(
        message: impl Into<Cow<'static, str>>,
        source: impl Into<anyhow::Error>,
    ) -> Self {
        OutputError(source.into().context(message.into()))
    }
}

impl From<String> for OutputError {
    fn from(message: String) -> Self {
        OutputError::msg(message)
    }
}

impl From<&'static str> for OutputError {
    fn from(message: &'static str) -> Self {
        OutputError::msg(message)
    }
}

impl From<OmaAptError> for OutputError {
    fn from(value: OmaAptError) -> Self {
        oma_apt_error_to_output(value)
    }
}

impl From<PtrIsNone> for OutputError {
    fn from(value: PtrIsNone) -> Self {
        Self::msg(value.to_string())
    }
}

impl From<serde_json::Error> for OutputError {
    fn from(value: serde_json::Error) -> Self {
        Self::with_source("Failed to serialize to JSON", value)
    }
}

impl From<oma_apt_pkg::Error> for OutputError {
    fn from(value: oma_apt_pkg::Error) -> Self {
        Self::with_source("Failed to load apt data", value)
    }
}

#[cfg(feature = "aosc")]
impl From<MirrorError> for OutputError {
    fn from(value: MirrorError) -> Self {
        match value {
            MirrorError::ReadFile { path, source } => Self::with_source(
                fl!("failed-to-operate-path", p = path.display()),
                source,
            ),
            MirrorError::ParseJson { path, source } => Self::with_source(
                fl!("failed-to-parse-file", p = path.display()),
                source,
            ),
            MirrorError::MirrorNotExist { mirror_name } => {
                fl!("mirror-not-found", mirror = mirror_name.as_ref()).into()
            }
            MirrorError::SerializeJson { source } => {
                Self::with_source(fl!("failed-to-serialize-struct"), source)
            }
            MirrorError::WriteFile { path, source } => Self::with_source(
                fl!("failed-to-write-file", p = path.display()),
                source,
            ),
            MirrorError::CreateFile { path, source } => Self::with_source(
                fl!("failed-to-create-file", p = path.display()),
                source,
            ),
            MirrorError::ApplyEmptySettings => fl!("mirrors-setting-empty").into(),
            MirrorError::ParseConfig { source } => {
                Self::with_source("Failed to parse file", source)
            }
        }
    }
}

impl From<OmaDbusError> for OutputError {
    fn from(value: OmaDbusError) -> Self {
        debug!("{:?}", value);
        match value {
            OmaDbusError::FailedConnectDbus(e) => e.to_string().into(),
            OmaDbusError::FailedTakeWakeLock(e) => {
                Self::with_source(fl!("failed-to-set-lockscreen"), e)
            }
            OmaDbusError::FailedCreateProxy(proxy, e) => {
                let proxy = proxy.to_string();
                Self::with_source(fl!("failed-to-create-proxy", proxy = proxy), e)
            }
            OmaDbusError::FailedGetBatteryStatus(e) => {
                Self::with_source(fl!("failed-to-set-lockscreen"), e)
            }
            OmaDbusError::FailedGetOmaStatus(e) => Self::with_source("Failed to get oma status", e),
            OmaDbusError::SessionState(_) => value.to_string().into(),
        }
    }
}

#[cfg(feature = "aosc")]
impl From<TumError> for OutputError {
    fn from(value: TumError) -> Self {
        use crate::utils::get_lists_dir;

        let p1 = get_lists_dir().to_string_lossy().to_string();

        match value {
            TumError::ReadAptListDir { source } => {
                Self::with_source(fl!("failed-to-operate-path", p = p1), source)
            }
            TumError::ReadDirEntry { source } => {
                Self::with_source("Failed to read dir entry", source)
            }
            TumError::ReadFile { path, source } => {
                let path = path.to_string_lossy().to_string();
                Self::with_source(fl!("failed-to-operate-path", p = path), source)
            }
        }
    }
}

impl From<OmaSearchError> for OutputError {
    fn from(value: OmaSearchError) -> Self {
        match value {
            OmaSearchError::NoResult(e) => fl!("could-not-find-pkg-from-keyword", c = e).into(),
            OmaSearchError::FailedGetCandidate(s) => fl!("no-candidate-ver", pkg = s).into(),
            OmaSearchError::PtrIsNone => value.to_string().into(),
        }
    }
}

impl From<AptErrors> for OutputError {
    fn from(e: AptErrors) -> Self {
        for c in e.iter() {
            if c.is_error {
                error!("{}", c.msg);
                continue;
            }

            info!("{}", c.msg);
        }

        fl!("apt-error").into()
    }
}

impl From<MatcherError> for OutputError {
    fn from(value: MatcherError) -> Self {
        oma_database_error(value)
    }
}

impl From<RefreshError> for OutputError {
    fn from(value: RefreshError) -> Self {
        debug!("{:?}", value);
        match value {
            RefreshError::InvalidUrl(url) => fl!("invalid-url", url = url).into(),
            RefreshError::ScanSourceError(e) => e.to_string().into(),
            RefreshError::UnsupportedProtocol(s) => fl!("unsupported-protocol", url = s).into(),
            #[cfg(feature = "aosc")]
            RefreshError::TopicsError(e) => oma_topics_error(e),
            RefreshError::NoInReleaseFile(s) => fl!("not-found", url = s).into(),
            RefreshError::InReleaseParseError(path, e) => match e {
                InReleaseError::VerifyError(e) => match e {
                    VerifyError::CertParseFileError(p, e) => Self::with_source(
                        fl!("fail-load-certs-from-file", path = p),
                        io::Error::other(e),
                    ),
                    VerifyError::BadCertFile(p, e) => {
                        Self::with_source(fl!("cert-file-is-bad", path = p), io::Error::other(e))
                    }
                    VerifyError::TrustedDirNotExist => e.to_string().into(),
                    VerifyError::Anyhow(e) => Self::with_source(
                        fl!("verify-error", p = file_name(&path)),
                        io::Error::other(e),
                    ),
                    VerifyError::FailedToReadInRelease(e) => {
                        Self::with_source(fl!("failed-to-read-decode-inrelease"), e)
                    }
                },
                InReleaseError::BadInReleaseData => fl!("can-not-parse-date").into(),
                InReleaseError::BadInReleaseValidUntil => fl!("can-not-parse-valid-until").into(),
                InReleaseError::EarlierSignature => {
                    fl!("earlier-signature", filename = file_name(&path)).into()
                }
                InReleaseError::ExpiredSignature => {
                    fl!("expired-signature", filename = file_name(&path)).into()
                }
                InReleaseError::InReleaseSyntaxError => {
                    fl!("inrelease-syntax-error", path = file_name(&path)).into()
                }
                InReleaseError::UnsupportedFileType => {
                    fl!("inrelease-parse-unsupported-file-type").into()
                }
                InReleaseError::ParseIntError(e) => e.to_string().into(),
                InReleaseError::NotTrusted => {
                    fl!("mirror-is-not-trusted", mirror = file_name(&path)).into()
                }
                InReleaseError::BrokenInRelease => {
                    fl!("inrelease-checksum-can-not-parse", p = file_name(&path)).into()
                }
                InReleaseError::ReadGPGFileName(error, file_name) => {
                    Self::with_source(fl!("failed-to-parse-file", p = file_name), error)
                }
            },
            RefreshError::JoinError(e) => e.to_string().into(),
            RefreshError::ChecksumError(e) => e.into(),
            RefreshError::FailedToOperateDirOrFile(path, e) => {
                Self::with_source(fl!("failed-to-operate-path", p = path), e)
            }
            RefreshError::ReadDownloadDir(path, e) => {
                Self::with_source(fl!("failed-to-operate-path", p = path), e)
            }
            RefreshError::AhoCorasickBuilder(e) => e.to_string().into(),
            RefreshError::ReplaceAll(e) => Self::with_source("stream_replace_all failed", e),
            RefreshError::SetLock(e) => match e {
                GetLockError::SetLock(errno) => Self::with_source(fl!("oma-refresh-lock"), errno),
                GetLockError::SetLockWithProcess(cmd, pid) => Self::with_source(
                    fl!("oma-refresh-lock"),
                    io::Error::other(fl!("oma-refresh-lock-dueto", exec = cmd, pid = pid)),
                ),
            },
            RefreshError::DuplicateComponents(url, component) => {
                fl!("doplicate-component", url = url.as_ref(), c = component).into()
            }
            RefreshError::SourceListsEmpty => fl!("sources-list-empty").into(),
            RefreshError::DownloadFailed(err) => {
                if let Some(err) = err {
                    Self::with_source(fl!("failed-refresh"), err)
                } else {
                    fl!("failed-refresh").into()
                }
            }
            RefreshError::OperateFile(path, error) => Self::with_source(
                fl!("failed-to-operate-path", p = path.display()),
                error,
            ),
            RefreshError::WrongThreadCount(count) => {
                fl!("wrong-thread-count", count = count).into()
            }
            RefreshError::DownloadManagerBuilderError(builder_error) => match builder_error {
                BuilderError::EmptySource { file_name } => {
                    format!("BUG: task {file_name} should is not empty").into()
                }
                BuilderError::IllegalDownloadThread { count } => {
                    fl!("wrong-thread-count", count = count).into()
                }
            },
            RefreshError::NoMetadataToDownload => fl!("oma-refresh-no-metadata-to-download").into(),
            RefreshError::CreateTokioRuntime(error) => error.to_string().into(),
        }
    }
}

impl From<AuthConfigError> for OutputError {
    fn from(value: AuthConfigError) -> Self {
        match value {
            AuthConfigError::ReadDir { path, err } => {
                Self::with_source(format!("Failed to read dir {}", path.display()), err)
            }
            AuthConfigError::DirEntry(error) => {
                Self::with_source("Failed to read dir entry", error)
            }
            AuthConfigError::OpenFile { path, err } => {
                Self::with_source(format!("Failed to open file: {}", path.display()), err)
            }
            AuthConfigError::ParseError(error) => {
                Self::with_source("Parse auth file got error", error)
            }
        }
    }
}

#[cfg(feature = "aosc")]
impl From<OmaTopicsError> for OutputError {
    fn from(value: OmaTopicsError) -> Self {
        oma_topics_error(value)
    }
}

#[cfg(feature = "aosc")]
fn oma_topics_error(e: OmaTopicsError) -> OutputError {
    debug!("{:?}", e);
    match e {
        OmaTopicsError::FailedToOperateDirOrFile(path, e) => {
            OutputError::with_source(fl!("failed-to-operate-path", p = path), e)
        }
        OmaTopicsError::CanNotFindTopic(topic) => {
            fl!("can-not-find-specified-topic", topic = topic).into()
        }
        OmaTopicsError::FailedToDisableTopic(topic) => {
            fl!("can-not-find-specified-topic", topic = topic).into()
        }
        OmaTopicsError::ReqwestError(e) => OutputError::from(e),
        OmaTopicsError::FailedSer => e.to_string().into(),
        OmaTopicsError::FailedGetParentPath(p) => {
            fl!("failed-to-get-parent-path", p = p.display()).into()
        }
        OmaTopicsError::BrokenFile(p) => fl!("failed-to-read", p = p).into(),
        OmaTopicsError::ParseUrl(e, url) => {
            OutputError::with_source(fl!("invalid-url", url = url), e)
        }
        OmaTopicsError::UnsupportedProtocol(s) => fl!("unsupported-protocol", url = s).into(),
        OmaTopicsError::OpenFile(s, e) => {
            OutputError::with_source(fl!("failed-to-operate-path", p = s), e)
        }
        OmaTopicsError::ReadFile(s, e) => {
            OutputError::with_source(fl!("failed-to-read-file-metadata", p = s), e)
        }
        OmaTopicsError::MirrorError(mirror_error) => OutputError::from(mirror_error),

        OmaTopicsError::IllegalTopicEntry(name) => fl!(
            "illegal-topic-entry",
            name = name.escape_default().to_string()
        )
        .into(),
        OmaTopicsError::FailedToCreateTokioRuntime(error) => error.to_string().into(),
        OmaTopicsError::NotSupportCurrentThread => unreachable!(),
        OmaTopicsError::CreateTokioRuntime(error) => {
            OutputError::with_source("Failed to create tokio runtime", error)
        }
        OmaTopicsError::RecvError => e.to_string().into(),
    }
}

impl From<DpkgError> for OutputError {
    fn from(value: DpkgError) -> Self {
        debug!("{:?}", value);
        Self::with_source(fl!("can-not-run-dpkg-print-arch"), value)
    }
}

impl From<OmaContentsError> for OutputError {
    fn from(value: OmaContentsError) -> Self {
        match value {
            OmaContentsError::ContentsNotExist => fl!("contents-does-not-exist").into(),
            OmaContentsError::ExecuteRgFailed(e) => {
                Self::with_source(fl!("execute-ripgrep-failed"), e)
            }
            OmaContentsError::ContentsEntryMissingPathList(s) => {
                fl!("contents-entry-missing-path-list", entry = s).into()
            }
            OmaContentsError::CnfWrongArgument => value.to_string().into(),
            OmaContentsError::RgWithError => fl!("rg-non-zero").into(),
            OmaContentsError::FailedToOperateDirOrFile(path, e) => {
                Self::with_source(fl!("failed-to-operate-path", p = path), e)
            }
            OmaContentsError::FailedToGetFileMetadata(path, e) => {
                Self::with_source(fl!("failed-to-read-file-metadata", p = path), e)
            }
            OmaContentsError::FailedToWaitExit(e) => {
                Self::with_source(fl!("failed-to-get-rg-process-info"), e)
            }
            OmaContentsError::LzzzErr(e) => {
                Self::with_source(fl!("failed-to-decompress-contents"), e)
            }
            OmaContentsError::NoResult => OutputError::already_reported(),
            OmaContentsError::IllegalFile(path) => format!("Illegal file: {path}").into(),
            OmaContentsError::InvalidContents(_) => value.to_string().into(),
            OmaContentsError::InvalidContentsWithLine(_, _) => unreachable!(),
        }
    }
}

pub fn oma_apt_error_to_output(err: OmaAptError) -> OutputError {
    debug!("{:?}", err);
    match err {
        OmaAptError::OmaDatabaseError(e) => oma_database_error(e),
        OmaAptError::MarkReinstallError(pkg, version) => {
            fl!("can-not-mark-reinstall", name = pkg, version = version).into()
        }
        OmaAptError::DependencyIssue {
            broken_dependencies: broken_deps,
            is_solver3,
            apt_errors,
        } => {
            error!("{}", fl!("dep-issue-1"));
            debug!("{:#?}", broken_deps);

            let mut solver_3_errs = vec![];

            if is_solver3 {
                for err in apt_errors.iter() {
                    if !solver_3_errs.contains(&&err.msg) {
                        solver_3_errs.push(&err.msg);
                    }
                }

                #[cfg(feature = "aosc")]
                if !solver_3_errs.is_empty() {
                    info!("{}", fl!("dep-issue-2"));
                }

                eprintln!();

                for err in &solver_3_errs {
                    msg!("{err}");
                }
            }

            if !broken_deps.is_empty() && solver_3_errs.is_empty() {
                let name_len_max = broken_deps
                    .iter()
                    .filter(|dep| !dep.is_empty())
                    .map(|dep| dep[0].name.len())
                    .max();

                if let Some(name_len_max) = name_len_max {
                    #[cfg(feature = "aosc")]
                    info!("{}", fl!("dep-issue-2"));

                    println!();

                    let first_writer = Writer::new_no_limit_length(name_len_max as u16 + 2 + 4);
                    let second_writer =
                        Writer::new_no_limit_length(name_len_max as u16 + 2 + 4 + 4);

                    let mut last_name = "";

                    for dep in &broken_deps {
                        let mut prefix = String::new();
                        if last_name != dep[0].name {
                            prefix = format!("{}:", dep[0].name);
                            last_name = &dep[0].name;
                        }

                        let why = &dep[0].why;
                        let mut output = format!("{}: {}", why.0, why.1);

                        let readson = &dep[0].reason;

                        if let Some(reason) = readson {
                            output += &format!(" {reason}");
                        }

                        if dep.len() > 1 {
                            output += " or";
                        }

                        first_writer.writeln(&prefix, &output).ok();

                        if dep.len() > 1 {
                            for or in dep.iter().skip(1) {
                                let reason = &or.reason;

                                if let Some(reason) = reason {
                                    second_writer
                                        .writeln("", &format!("{} {}", or.why.1, reason))
                                        .ok();
                                } else {
                                    second_writer.writeln("", &or.why.1).ok();
                                }
                            }
                        }
                    }

                    println!();
                }
            }

            OutputError::already_reported()
        }
        OmaAptError::PkgIsEssential(pkg) => fl!("pkg-is-essential", name = pkg).into(),
        OmaAptError::PkgNoCandidate(s) => fl!("no-candidate-ver", pkg = s).into(),
        OmaAptError::PkgNoChecksum(s) => fl!("pkg-no-checksum", name = s).into(),
        OmaAptError::InvalidFileName(s) => fl!("invalid-filename", name = s).into(),
        OmaAptError::DpkgFailedConfigure(_) => OutputError::with_source(
            fl!("dpkg-configure-a-non-zero"),
            io::Error::other(fl!("dpkg-configure-failed-due-to-tips")),
        ),
        OmaAptError::DiskSpaceInsufficient(need, avail) => fl!(
            "need-more-size",
            a = avail.to_string(),
            n = need.to_string()
        )
        .into(),
        OmaAptError::MarkStatus(e) => OutputError::with_source("Failed to mark package status", e),
        OmaAptError::MarkPkgNotInstalled(pkg) => fl!("pkg-is-not-installed", pkg = pkg).into(),
        OmaAptError::DpkgError(e) => OutputError::from(e),
        OmaAptError::PkgUnavailable(pkg, ver) => {
            fl!("pkg-unavailable", pkg = pkg, ver = ver).into()
        }
        OmaAptError::FailedCreateAsyncRuntime(e) => {
            OutputError::with_source("Failed to create async runtime", e)
        }
        OmaAptError::FailedOperateDirOrFile(path, e) => {
            OutputError::with_source(fl!("failed-to-operate-path", p = path), e)
        }
        OmaAptError::FailedGetAvailableSpace(e) => {
            OutputError::with_source(fl!("failed-to-calculate-available-space"), e)
        }
        OmaAptError::FailedGetParentPath(p) => {
            fl!("failed-to-get-parent-path", p = p.display()).into()
        }
        OmaAptError::FailedGetCanonicalize(p, e) => {
            OutputError::with_source(format!("Failed canonicalize path: {p}"), e)
        }
        OmaAptError::PtrIsNone(_) => err.to_string().into(),
        OmaAptError::ChecksumError(e) => e.into(),
        OmaAptError::Features => fl!("features-abort").into(),
        OmaAptError::DpkgTriggers(e) => {
            OutputError::with_source(fl!("dpkg-triggers-only-a-non-zero"), e)
        }
        OmaAptError::FailedToDownload(len) => fl!("download-failed-with-len", len = len).into(),
        OmaAptError::CreateCache(apt_errors) => {
            error!("{}", fl!("failed-create-pkg-index-cache"));

            for_each_display_apt_err_messages(apt_errors);

            due_to!("{}", fl!("failed-create-cache-tips"));

            #[cfg(feature = "aosc")]
            info!("{}", fl!("aosc-upload-issue-tips"));

            OutputError::already_reported()
        }
        OmaAptError::SetUpgradeMode(apt_errors) => {
            error!("{}", fl!("failed-set-upgrade-mode"));

            for_each_display_apt_err_messages(apt_errors);

            due_to!("{}", fl!("failed-set-upgrade-mode-tips"));

            #[cfg(feature = "aosc")]
            info!("{}", fl!("aosc-upload-issue-tips"));

            OutputError::already_reported()
        }
        OmaAptError::LockApt(apt_errors) => {
            error!("{}", fl!("failed-lock-apt"));

            for_each_display_apt_err_messages(apt_errors);

            due_to!("{}", fl!("failed-set-upgrade-mode-tips"));

            #[cfg(feature = "aosc")]
            info!("{}", fl!("aosc-upload-issue-tips"));

            OutputError::already_reported()
        }
        OmaAptError::InstallPackages(apt_errors) => {
            error!("{}", fl!("failed-install-pkgs"));

            for_each_display_apt_err_messages(apt_errors);

            due_to!("{}", fl!("failed-install-pkgs-dueto"));

            #[cfg(feature = "aosc")]
            info!("{}", fl!("aosc-upload-issue-tips"));

            OutputError::already_reported()
        }
        OmaAptError::PathNotExist(path) => fl!("path-not-exist", path = path).into(),
        OmaAptError::DpkgStatusGetPkg(_) => anyhow::anyhow!("{err}").into(),
        OmaAptError::WrongDpkgStatus(_) => anyhow::anyhow!("{err}").into(),
        OmaAptError::DpkgStatusBroken(_) => anyhow::anyhow!("{err}").into(),
        OmaAptError::FailedGetArchiveDirLock(get_lock_error) => match get_lock_error {
            GetLockError::SetLock(errno) => {
                OutputError::with_source(fl!("oma-archive-lock"), errno)
            }
            GetLockError::SetLockWithProcess(cmd, pid) => OutputError::with_source(
                fl!("oma-archive-lock"),
                io::Error::other(fl!("oma-archive-lock-dueto", exec = cmd, pid = pid)),
            ),
        },
        OmaAptError::RecvError => anyhow::anyhow!("{err}").into(),
        OmaAptError::Anyhow(error) => error.into(),
    }
}

fn for_each_display_apt_err_messages(apt_errors: AptErrors) {
    for (i, e) in apt_errors.iter().enumerate() {
        msg!("{}: {}", i + 1, e);
    }
}

impl From<reqwest_middleware::Error> for OutputError {
    fn from(value: reqwest_middleware::Error) -> Self {
        match value {
            reqwest_middleware::Error::Middleware(error) => OutputError::from(error),
            reqwest_middleware::Error::Reqwest(error) => OutputError::from(error),
        }
    }
}

impl From<reqwest::Error> for OutputError {
    fn from(e: reqwest::Error) -> Self {
        debug!("{:?}", e);
        let filename = &e
            .url()
            .and_then(|x| x.path_segments())
            .and_then(|mut x| x.next_back());

        if e.is_builder() {
            return Self::with_source(fl!("failed-to-create-http-client"), e);
        }

        if let Some(filename) = filename
            && filename.len() <= 256
        {
            return Self::with_source(fl!("download-failed", filename = *filename), e);
        }

        fl!("download-failed-no-name").into()
    }
}

fn oma_checksum_error(e: ChecksumError) -> OutputError {
    debug!("{:?}", e);
    match e {
        ChecksumError::OpenFile { source, path } => OutputError::with_source(
            fl!(
                "failed-to-open-to-checksum",
                path = path.display().to_string()
            ),
            source,
        ),
        ChecksumError::Copy { source } => OutputError::with_source(fl!("can-not-checksum"), source),
        ChecksumError::BadLength => fl!("sha256-bad-length").into(),
        ChecksumError::Decode { source } => {
            OutputError::with_source(fl!("can-not-checksum"), source)
        }
    }
}

impl From<ChecksumError> for OutputError {
    fn from(value: ChecksumError) -> Self {
        oma_checksum_error(value)
    }
}

fn oma_database_error(e: MatcherError) -> OutputError {
    debug!("{:?}", e);
    match e {
        MatcherError::InvalidPattern(s) => fl!("invalid-pattern", p = s).into(),
        MatcherError::NoPackage(s) => fl!("can-not-get-pkg-from-database", name = s).into(),
        MatcherError::NoVersion(pkg, ver) => fl!("pkg-unavailable", pkg = pkg, ver = ver).into(),
        MatcherError::NoPath(s) => fl!("invalid-path", p = s).into(),
        MatcherError::NoCandidate(s) => fl!("no-candidate-ver", pkg = s).into(),
        MatcherError::PtrIsNone(_) => e.to_string().into(),
        MatcherError::DpkgError(dpkg_error) => OutputError::from(dpkg_error),
    }
}

impl From<HistoryError> for OutputError {
    fn from(value: HistoryError) -> Self {
        debug!("{:?}", value);
        match value {
            HistoryError::FailedOperateDirOrFile(s, e) => {
                Self::with_source(fl!("failed-to-operate-path", p = s), e)
            }
            HistoryError::ConnectError(e) => {
                Self::with_source(fl!("failed-to-connect-history-database"), e)
            }
            HistoryError::ExecuteError(e) => {
                Self::with_source(fl!("failed-to-execute-query-stmt"), e)
            }
            HistoryError::ParseDbError(e) => {
                Self::with_source(fl!("failed-to-parse-history-object"), e)
            }
            HistoryError::NoResult(id) => format!("No result by id: {id}").into(),
            HistoryError::HistoryEmpty => fl!("oma-history-is-empty").into(),
            HistoryError::FailedParentPath(p) => fl!("failed-to-get-parent-path", p = p).into(),
            HistoryError::CreateTransaction(error) => {
                Self::with_source(fl!("failed-to-execute-query-stmt"), error)
            }
            HistoryError::NoUpgradeSystemLog => unreachable!(),
        }
    }
}

impl From<SingleDownloadError> for OutputError {
    fn from(value: SingleDownloadError) -> Self {
        match value {
            SingleDownloadError::SetPermission { source } => {
                Self::with_source(fl!("set-permission"), source)
            }
            SingleDownloadError::OpenAsWriteMode { source } => {
                Self::with_source(fl!("open-file-as-write-mode"), source)
            }
            SingleDownloadError::Open { source } => Self::with_source(fl!("open-err"), source),
            SingleDownloadError::Create { source } => Self::with_source(fl!("create-err"), source),
            SingleDownloadError::Seek { source } => Self::with_source(fl!("seek-err"), source),
            SingleDownloadError::Write { source } => Self::with_source(fl!("write-err"), source),
            SingleDownloadError::Flush { source } => Self::with_source(fl!("flush-err"), source),
            SingleDownloadError::Remove { source } => Self::with_source(fl!("remove-err"), source),
            SingleDownloadError::CreateSymlink { source } => {
                Self::with_source(fl!("create-symlink-err"), source)
            }
            SingleDownloadError::ReqwestMiddlewareError { source } => {
                Self::with_source(fl!("reqwest-err"), source)
            }
            SingleDownloadError::BrokenPipe { source } => {
                Self::with_source(fl!("broken-pipe-err"), source)
            }
            SingleDownloadError::SendRequestTimeout => fl!("send-request-timeout").into(),
            SingleDownloadError::DownloadTimeout => fl!("download-timeout").into(),
            SingleDownloadError::ChecksumMismatch => fl!("checksum-mismatch-download-err").into(),
            SingleDownloadError::AcquireError => value.to_string().into(),
        }
    }
}

fn file_name(p: &Path) -> String {
    p.file_name()
        .map(|x| x.to_string_lossy().to_string())
        .unwrap_or_else(|| "..".into())
}
