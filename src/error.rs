use std::error::Error;
use std::fmt::Display;
use std::io::{self, ErrorKind};

use apt_auth_config::AuthConfigError;
use oma_console::due_to;
use oma_console::writer::{Writeln, Writer};
use oma_contents::OmaContentsError;
use oma_fetch::checksum::ChecksumError;
use oma_fetch::DownloadError;
use oma_history::HistoryError;

#[cfg(feature = "aosc")]
use oma_mirror::MirrorError;

use oma_pm::search::OmaSearchError;
use oma_pm::AptErrors;
use oma_pm::{apt::OmaAptError, matches::MatcherError};
use oma_refresh::db::RefreshError;
use oma_refresh::inrelease::InReleaseError;
use oma_repo_verify::VerifyError;
use oma_utils::dbus::OmaDbusError;
use oma_utils::dpkg::DpkgError;

#[cfg(feature = "aosc")]
use oma_topics::OmaTopicsError;
use reqwest::StatusCode;
use tracing::{debug, error, info};

use crate::fl;
use crate::subcommand::utils::LockError;

use self::ChainState::*;

use std::vec;

#[derive(Clone)]
pub(crate) enum ChainState<'a> {
    Linked {
        next: Option<&'a (dyn Error + 'static)>,
    },
    Buffered {
        rest: vec::IntoIter<&'a (dyn Error + 'static)>,
    },
}

pub struct Chain<'a> {
    state: ChainState<'a>,
}

impl<'a> Chain<'a> {
    /// Construct an iterator over a chain of errors via the `source` method
    ///
    /// # Example
    ///
    /// ```rust
    /// use std::error::Error;
    /// use std::fmt::{self, Write};
    /// use eyre::Chain;
    /// use indenter::indented;
    ///
    /// fn report(error: &(dyn Error + 'static), f: &mut fmt::Formatter<'_>) -> fmt::Result {
    ///     let mut errors = Chain::new(error).enumerate();
    ///     for (i, error) in errors {
    ///         writeln!(f)?;
    ///         write!(indented(f).ind(i), "{}", error)?;
    ///     }
    ///
    ///     Ok(())
    /// }
    /// ```
    pub fn new(head: &'a (dyn Error + 'static)) -> Self {
        Chain {
            state: ChainState::Linked { next: Some(head) },
        }
    }
}

impl<'a> Iterator for Chain<'a> {
    type Item = &'a (dyn Error + 'static);

    fn next(&mut self) -> Option<Self::Item> {
        match &mut self.state {
            Linked { next } => {
                let error = (*next)?;
                *next = error.source();
                Some(error)
            }
            Buffered { rest } => rest.next(),
        }
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        let len = self.len();
        (len, Some(len))
    }
}

impl DoubleEndedIterator for Chain<'_> {
    fn next_back(&mut self) -> Option<Self::Item> {
        match &mut self.state {
            Linked { mut next } => {
                let mut rest = Vec::new();
                while let Some(cause) = next {
                    next = cause.source();
                    rest.push(cause);
                }
                let mut rest = rest.into_iter();
                let last = rest.next_back();
                self.state = Buffered { rest };
                last
            }
            Buffered { rest } => rest.next_back(),
        }
    }
}

impl ExactSizeIterator for Chain<'_> {
    fn len(&self) -> usize {
        match &self.state {
            Linked { mut next } => {
                let mut len = 0;
                while let Some(cause) = next {
                    next = cause.source();
                    len += 1;
                }
                len
            }
            Buffered { rest } => rest.len(),
        }
    }
}

impl Default for Chain<'_> {
    fn default() -> Self {
        Chain {
            state: ChainState::Buffered {
                rest: Vec::new().into_iter(),
            },
        }
    }
}

#[derive(Debug)]
pub struct OutputError {
    pub description: String,
    pub source: Option<Box<dyn Error>>,
}

impl Display for OutputError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.description)
    }
}

impl Error for OutputError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        self.source.as_deref()
    }

    fn cause(&self) -> Option<&dyn Error> {
        self.source()
    }
}

impl From<OmaAptError> for OutputError {
    fn from(value: OmaAptError) -> Self {
        oma_apt_error_to_output(value)
    }
}

impl From<LockError> for OutputError {
    fn from(value: LockError) -> Self {
        Self {
            description: "".to_string(),
            source: Some(Box::new(value)),
        }
    }
}

#[cfg(feature = "aosc")]
impl From<MirrorError> for OutputError {
    fn from(value: MirrorError) -> Self {
        match value {
            MirrorError::ReadFile { path, source } => Self {
                description: fl!("failed-to-operate-path", p = path.display().to_string()),
                source: Some(Box::new(source)),
            },
            MirrorError::ParseJson { path, source } => Self {
                description: fl!("failed-to-parse-file", p = path.display().to_string()),
                source: Some(Box::new(source)),
            },
            MirrorError::ParseYaml { path, source } => Self {
                description: fl!("failed-to-parse-file", p = path.display().to_string()),
                source: Some(Box::new(source)),
            },
            MirrorError::MirrorNotExist { mirror_name } => Self {
                description: fl!("mirror-not-found", mirror = mirror_name.as_ref()),
                source: None,
            },
            MirrorError::SerializeJson { source } => Self {
                description: fl!("failed-to-serialize-struct"),
                source: Some(Box::new(source)),
            },
            MirrorError::WriteFile { path, source } => Self {
                description: fl!("failed-to-write-file", p = path.display().to_string()),
                source: Some(Box::new(source)),
            },
            MirrorError::CreateFile { path, source } => Self {
                description: fl!("failed-to-create-file", p = path.display().to_string()),
                source: Some(Box::new(source)),
            },
        }
    }
}

impl From<OmaDbusError> for OutputError {
    fn from(value: OmaDbusError) -> Self {
        debug!("{:?}", value);
        match value {
            OmaDbusError::FailedConnectDbus(_) => Self {
                description: "".to_string(),
                source: Some(Box::new(value)),
            },
            OmaDbusError::FailedTakeWakeLock(e) => Self {
                description: fl!("failed-to-set-lockscreen"),
                source: Some(Box::new(e)),
            },
            OmaDbusError::FailedCreateProxy(proxy, e) => {
                let proxy = proxy.to_string();
                Self {
                    description: fl!("failed-to-create-proxy", proxy = proxy),
                    source: Some(Box::new(e)),
                }
            }
            OmaDbusError::FailedGetBatteryStatus(e) => Self {
                description: fl!("failed-to-set-lockscreen"),
                source: Some(Box::new(e)),
            },
            OmaDbusError::FailedGetOmaStatus(e) => Self {
                description: "Failed to get oma status".to_string(),
                source: Some(Box::new(e)),
            },
            OmaDbusError::SessionState(_) => Self {
                description: value.to_string(),
                source: None,
            },
        }
    }
}

impl From<OmaSearchError> for OutputError {
    fn from(value: OmaSearchError) -> Self {
        match value {
            OmaSearchError::AptError(e) => OutputError {
                description: fl!("apt-error"),
                source: Some(Box::new(e)),
            },
            OmaSearchError::NoResult(e) => OutputError {
                description: fl!("could-not-find-pkg-from-keyword", c = e),
                source: None,
            },
            OmaSearchError::FailedGetCandidate(s) => OutputError {
                description: fl!("no-candidate-ver", pkg = s),
                source: None,
            },
            OmaSearchError::AptErrors(e) => OutputError::from(e),
            OmaSearchError::AptCxxException(e) => OutputError {
                description: fl!("apt-error"),
                source: Some(Box::new(AptErrors::from(e))),
            },
            OmaSearchError::PtrIsNone(_) => OutputError {
                description: value.to_string(),
                source: None,
            },
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

        OutputError {
            description: fl!("apt-error"),
            source: None,
        }
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
        #[cfg(feature = "aosc")]
        match value {
            RefreshError::InvalidUrl(_) => Self {
                description: fl!("invalid-url"),
                source: None,
            },
            RefreshError::ScanSourceError(e) => Self {
                description: e.to_string(),
                source: None,
            },
            RefreshError::UnsupportedProtocol(s) => Self {
                description: fl!("unsupported-protocol", url = s),
                source: None,
            },
            RefreshError::FetcherError(e) => oma_download_error(e),
            RefreshError::ReqwestError(e) => OutputError::from(e),
            RefreshError::TopicsError(e) => oma_topics_error(e),
            RefreshError::NoInReleaseFile(s) => Self {
                description: fl!("not-found", url = s),
                source: None,
            },
            RefreshError::InReleaseParseError(path, e) => match e {
                InReleaseError::VerifyError(e) => match e {
                    VerifyError::CertParseFileError(p, e) => Self {
                        description: fl!("fail-load-certs-from-file", path = p),
                        source: Some(Box::new(io::Error::new(ErrorKind::Other, e))),
                    },
                    VerifyError::BadCertFile(p, e) => Self {
                        description: fl!("cert-file-is-bad", path = p),
                        source: Some(Box::new(io::Error::new(ErrorKind::Other, e))),
                    },
                    VerifyError::TrustedDirNotExist => Self {
                        description: e.to_string(),
                        source: None,
                    },
                    VerifyError::Anyhow(e) => Self {
                        description: e.to_string(),
                        source: None,
                    },
                    VerifyError::FailedToReadInRelease(e) => Self {
                        description: fl!("failed-to-read-decode-inrelease"),
                        source: Some(Box::new(e)),
                    },
                },
                InReleaseError::BadInReleaseData => Self {
                    description: fl!("can-not-parse-date"),
                    source: None,
                },
                InReleaseError::BadInReleaseValidUntil => Self {
                    description: fl!("can-not-parse-valid-until"),
                    source: None,
                },
                InReleaseError::EarlierSignature => Self {
                    description: fl!("earlier-signature", filename = path),
                    source: None,
                },
                InReleaseError::ExpiredSignature => Self {
                    description: fl!("expired-signature", filename = path),
                    source: None,
                },
                InReleaseError::InReleaseSyntaxError => Self {
                    description: fl!("inrelease-syntax-error", path = path),
                    source: None,
                },
                InReleaseError::UnsupportedFileType => Self {
                    description: fl!("inrelease-parse-unsupported-file-type"),
                    source: None,
                },
                InReleaseError::ParseIntError(e) => Self {
                    description: e.to_string(),
                    source: None,
                },
                InReleaseError::NotTrusted => Self {
                    description: fl!("mirror-is-not-trusted", mirror = path),
                    source: None,
                },
                InReleaseError::BrokenInRelease => Self {
                    description: fl!("inrelease-checksum-can-not-parse", p = path),
                    source: None,
                },
            },
            RefreshError::DpkgArchError(e) => OutputError::from(e),
            RefreshError::JoinError(e) => Self {
                description: e.to_string(),
                source: None,
            },
            RefreshError::ChecksumError(e) => oma_checksum_error(e),
            RefreshError::FailedToOperateDirOrFile(path, e) => Self {
                description: fl!("failed-to-operate-path", p = path),
                source: Some(Box::new(e)),
            },
            RefreshError::ReadDownloadDir(_, e) => Self {
                description: e.to_string(),
                source: Some(Box::new(e)),
            },
            RefreshError::AhoCorasickBuilder(e) => Self {
                description: e.to_string(),
                source: None,
            },
            RefreshError::ReplaceAll(e) => Self {
                description: e.to_string(),
                source: Some(Box::new(e)),
            },
            RefreshError::SetLock(errno) => Self {
                description: fl!("oma-refresh-lock"),
                source: Some(Box::new(errno)),
            },
            RefreshError::SetLockWithProcess(cmd, pid) => Self {
                description: fl!("oma-refresh-lock"),
                source: Some(Box::new(io::Error::new(
                    ErrorKind::Other,
                    fl!("oma-refresh-lock-dueto", exec = cmd, pid = pid),
                ))),
            },
            RefreshError::DuplicateComponents(url, component) => Self {
                description: fl!("doplicate-component", url = url.to_string(), c = component),
                source: None,
            },
        }
        #[cfg(not(feature = "aosc"))]
        match value {
            RefreshError::InvalidUrl(_) => Self {
                description: fl!("invalid-url"),
                source: None,
            },
            RefreshError::ScanSourceError(e) => Self {
                description: e.to_string(),
                source: None,
            },
            RefreshError::UnsupportedProtocol(s) => Self {
                description: fl!("unsupported-protocol", url = s),
                source: None,
            },
            RefreshError::FetcherError(e) => oma_download_error(e),
            RefreshError::ReqwestError(e) => OutputError::from(e),
            RefreshError::NoInReleaseFile(s) => Self {
                description: fl!("not-found", url = s),
                source: None,
            },
            RefreshError::InReleaseParseError(p, e) => match e {
                InReleaseError::VerifyError(e) => match e {
                    VerifyError::CertParseFileError(p, e) => Self {
                        description: fl!("fail-load-certs-from-file", path = p),
                        source: Some(Box::new(io::Error::new(ErrorKind::Other, e))),
                    },
                    VerifyError::BadCertFile(p, e) => Self {
                        description: fl!("cert-file-is-bad", path = p),
                        source: Some(Box::new(io::Error::new(ErrorKind::Other, e))),
                    },
                    VerifyError::TrustedDirNotExist => Self {
                        description: e.to_string(),
                        source: None,
                    },
                    VerifyError::Anyhow(e) => Self {
                        description: e.to_string(),
                        source: None,
                    },
                    VerifyError::FailedToReadInRelease(e) => Self {
                        description: fl!("failed-to-read-decode-inrelease"),
                        source: Some(Box::new(e)),
                    },
                },
                InReleaseError::BadInReleaseData => Self {
                    description: fl!("can-not-parse-date"),
                    source: None,
                },
                InReleaseError::BadInReleaseValidUntil => Self {
                    description: fl!("can-not-parse-valid-until"),
                    source: None,
                },
                InReleaseError::EarlierSignature => Self {
                    description: fl!("earlier-signature", filename = p),
                    source: None,
                },
                InReleaseError::ExpiredSignature => Self {
                    description: fl!("expired-signature", filename = p),
                    source: None,
                },
                InReleaseError::InReleaseSyntaxError => Self {
                    description: fl!("inrelease-syntax-error", path = p),
                    source: None,
                },
                InReleaseError::UnsupportedFileType => Self {
                    description: fl!("inrelease-parse-unsupported-file-type"),
                    source: None,
                },
                InReleaseError::ParseIntError(e) => Self {
                    description: e.to_string(),
                    source: None,
                },
                InReleaseError::NotTrusted => Self {
                    description: fl!("mirror-is-not-trusted", mirror = p),
                    source: None,
                },
                InReleaseError::BrokenInRelease => Self {
                    description: fl!("inrelease-checksum-can-not-parse", p = p),
                    source: None,
                },
            },
            RefreshError::DpkgArchError(e) => OutputError::from(e),
            RefreshError::JoinError(e) => Self {
                description: e.to_string(),
                source: None,
            },
            RefreshError::ChecksumError(e) => oma_checksum_error(e),
            RefreshError::FailedToOperateDirOrFile(path, e) => Self {
                description: fl!("failed-to-operate-path", p = path),
                source: Some(Box::new(e)),
            },
            RefreshError::ReadDownloadDir(_, e) => Self {
                description: e.to_string(),
                source: Some(Box::new(e)),
            },
            RefreshError::AhoCorasickBuilder(e) => Self {
                description: e.to_string(),
                source: None,
            },
            RefreshError::ReplaceAll(e) => Self {
                description: e.to_string(),
                source: Some(Box::new(e)),
            },
            RefreshError::SetLock(errno) => Self {
                description: fl!("oma-refresh-lock"),
                source: Some(Box::new(errno)),
            },
            RefreshError::SetLockWithProcess(cmd, pid) => Self {
                description: fl!("oma-refresh-lock"),
                source: Some(Box::new(io::Error::new(
                    ErrorKind::Other,
                    fl!("oma-refresh-lock-dueto", exec = cmd, pid = pid),
                ))),
            },
            RefreshError::DuplicateComponents(url, component) => Self {
                description: fl!("doplicate-component", url = url.to_string(), c = component),
                source: None,
            },
        }
    }
}

impl From<AuthConfigError> for OutputError {
    fn from(value: AuthConfigError) -> Self {
        match value {
            AuthConfigError::ReadDir { path, err } => Self {
                description: format!("Failed to read dir {}", path.display()),
                source: Some(Box::new(err)),
            },
            AuthConfigError::DirEntry(error) => Self {
                description: "Failed to read dir entry".to_string(),
                source: Some(Box::new(error)),
            },
            AuthConfigError::OpenFile { path, err } => Self {
                description: format!("Failed to open file: {}", path.display()),
                source: Some(Box::new(err)),
            },
            AuthConfigError::MissingEntry(field) => Self {
                description: format!("Missing field: {field}"),
                source: None,
            },
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
        OmaTopicsError::FailedToOperateDirOrFile(path, e) => OutputError {
            description: fl!("failed-to-operate-path", p = path),
            source: Some(Box::new(e)),
        },
        OmaTopicsError::CanNotFindTopic(topic) => OutputError {
            description: fl!("can-not-find-specified-topic", topic = topic),
            source: None,
        },
        OmaTopicsError::FailedToDisableTopic(topic) => OutputError {
            description: fl!("can-not-find-specified-topic", topic = topic),
            source: None,
        },
        OmaTopicsError::ReqwestError(e) => OutputError::from(e),
        OmaTopicsError::FailedSer => OutputError {
            description: e.to_string(),
            source: None,
        },
        OmaTopicsError::FailedGetParentPath(p) => OutputError {
            description: fl!("failed-to-get-parent-path", p = p.display().to_string()),
            source: None,
        },
        OmaTopicsError::BrokenFile(p) => OutputError {
            description: fl!("failed-to-read", p = p),
            source: None,
        },
        OmaTopicsError::ParseUrl(e) => OutputError {
            description: fl!("invalid-url"),
            source: Some(Box::new(e)),
        },
        OmaTopicsError::UnsupportedProtocol(s) => OutputError {
            description: fl!("unsupported-protocol", url = s),
            source: None,
        },
        OmaTopicsError::OpenFile(s, e) => OutputError {
            description: fl!("failed-to-operate-path", p = s),
            source: Some(Box::new(e)),
        },
        OmaTopicsError::ReadFile(s, e) => OutputError {
            description: fl!("failed-to-read-file-metadata", p = s),
            source: Some(Box::new(e)),
        },
        OmaTopicsError::MirrorError(mirror_error) => OutputError::from(mirror_error),
    }
}

impl From<DpkgError> for OutputError {
    fn from(value: DpkgError) -> Self {
        debug!("{:?}", value);
        Self {
            description: fl!("can-not-run-dpkg-print-arch"),
            source: Some(Box::new(value)),
        }
    }
}

impl From<DownloadError> for OutputError {
    fn from(value: DownloadError) -> Self {
        oma_download_error(value)
    }
}

impl From<OmaContentsError> for OutputError {
    fn from(value: OmaContentsError) -> Self {
        match value {
            OmaContentsError::ContentsNotExist => Self {
                description: fl!("contents-does-not-exist"),
                source: None,
            },
            OmaContentsError::ExecuteRgFailed(e) => Self {
                description: fl!("execute-ripgrep-failed"),
                source: Some(Box::new(e)),
            },
            OmaContentsError::ContentsEntryMissingPathList(s) => Self {
                description: fl!("contents-entry-missing-path-list", entry = s),
                source: None,
            },
            OmaContentsError::CnfWrongArgument => Self {
                description: value.to_string(),
                source: None,
            },
            OmaContentsError::RgWithError => Self {
                description: fl!("rg-non-zero"),
                source: None,
            },
            OmaContentsError::FailedToOperateDirOrFile(path, e) => Self {
                description: fl!("failed-to-operate-path", p = path),
                source: Some(Box::new(e)),
            },
            OmaContentsError::FailedToGetFileMetadata(path, e) => Self {
                description: fl!("failed-to-read-file-metadata", p = path),
                source: Some(Box::new(e)),
            },
            OmaContentsError::FailedToWaitExit(e) => Self {
                description: fl!("failed-to-get-rg-process-info"),
                source: Some(Box::new(e)),
            },
            OmaContentsError::LzzzErr(e) => Self {
                description: fl!("failed-to-decompress-contents"),
                source: Some(Box::new(e)),
            },
            OmaContentsError::NoResult => Self {
                description: "".to_string(),
                source: None,
            },
            OmaContentsError::IllegalFile(path) => Self {
                description: format!("Illegal file: {path}"),
                source: None,
            },
        }
    }
}

impl From<anyhow::Error> for OutputError {
    fn from(value: anyhow::Error) -> Self {
        Self {
            description: value.to_string(),
            source: None,
        }
    }
}

pub fn oma_apt_error_to_output(err: OmaAptError) -> OutputError {
    debug!("{:?}", err);
    match err {
        OmaAptError::AptErrors(e) => OutputError::from(e),
        OmaAptError::OmaDatabaseError(e) => oma_database_error(e),
        OmaAptError::MarkReinstallError(pkg, version) => OutputError {
            description: fl!("can-not-mark-reinstall", name = pkg, version = version),
            source: None,
        },
        OmaAptError::DependencyIssue(ref broken_deps) => {
            error!("{}", fl!("dep-issue-1"));

            if !broken_deps.is_empty() {
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

                    for dep in broken_deps {
                        let mut prefix = String::new();
                        if last_name != dep[0].name {
                            prefix = format!("{}:", dep[0].name);
                            last_name = &dep[0].name;
                        }

                        let why = &dep[0].why;
                        let mut output = format!("{}: {}", why.0, why.1);

                        let readson = &dep[0].reason;

                        if let Some(reason) = readson {
                            output += &format!(" {}", reason);
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

            OutputError {
                description: "".to_string(),
                source: None,
            }
        }
        OmaAptError::PkgIsEssential(pkg) => OutputError {
            description: fl!("pkg-is-essential", name = pkg),
            source: None,
        },
        OmaAptError::PkgNoCandidate(s) => OutputError {
            description: fl!("no-candidate-ver", pkg = s),
            source: None,
        },
        OmaAptError::PkgNoChecksum(s) => OutputError {
            description: fl!("pkg-no-checksum", name = s),
            source: None,
        },
        OmaAptError::InvalidFileName(s) => OutputError {
            description: fl!("invalid-filename", name = s),
            source: None,
        },
        OmaAptError::DownloadError(e) => oma_download_error(e),
        OmaAptError::DpkgFailedConfigure(e) => OutputError {
            description: fl!("dpkg-configure-a-non-zero"),
            source: Some(Box::new(e)),
        },
        OmaAptError::DiskSpaceInsufficient(need, avail) => OutputError {
            description: fl!(
                "need-more-size",
                a = avail.to_string(),
                n = need.to_string()
            ),
            source: None,
        },
        OmaAptError::CommitErr(e) => OutputError {
            description: e,
            source: None,
        },
        OmaAptError::MarkPkgNotInstalled(pkg) => OutputError {
            description: fl!("pkg-is-not-installed", pkg = pkg),
            source: None,
        },
        OmaAptError::DpkgError(e) => OutputError::from(e),
        OmaAptError::PkgUnavailable(pkg, ver) => OutputError {
            description: fl!("pkg-unavailable", pkg = pkg, ver = ver),
            source: None,
        },
        OmaAptError::FailedToDownload(size, errs) => {
            for i in errs {
                let err = oma_download_error(i);
                error!("{}", err.description);
                if let Some(s) = err.source {
                    due_to!("{s}");
                    if let Some(e) = s.downcast_ref::<reqwest::Error>() {
                        if e.status().is_some_and(|x| x == StatusCode::UNAUTHORIZED) {
                            info!("{}", fl!("lack-auth-config-1"));
                            info!("{}", fl!("lack-auth-config-2"));
                        }
                    }
                }
            }
            OutputError {
                description: fl!("download-failed-with-len", len = size),
                source: None,
            }
        }
        OmaAptError::FailedCreateAsyncRuntime(e) => OutputError {
            description: "Failed to create async runtime".to_string(),
            source: Some(Box::new(e)),
        },
        OmaAptError::FailedOperateDirOrFile(path, e) => OutputError {
            description: fl!("failed-to-operate-path", p = path),
            source: Some(Box::new(e)),
        },
        OmaAptError::FailedGetAvailableSpace(e) => OutputError {
            description: fl!("failed-to-calculate-available-space"),
            source: Some(Box::new(e)),
        },
        OmaAptError::FailedGetParentPath(p) => OutputError {
            description: fl!("failed-to-get-parent-path", p = p.display().to_string()),
            source: None,
        },
        OmaAptError::FailedGetCanonicalize(p, e) => OutputError {
            description: format!("Failed canonicalize path: {p}"),
            source: Some(Box::new(e)),
        },
        OmaAptError::AptError(e) => OutputError {
            description: fl!("apt-error"),
            source: Some(Box::new(e)),
        },
        OmaAptError::AptCxxException(e) => OutputError {
            description: fl!("apt-error"),
            source: Some(Box::new(AptErrors::from(e))),
        },
        OmaAptError::PtrIsNone(_) => OutputError {
            description: err.to_string(),
            source: None,
        },
        OmaAptError::ChecksumError(e) => oma_checksum_error(e),
        OmaAptError::Features => OutputError {
            description: fl!("features-abort"),
            source: None,
        },
        OmaAptError::DpkgTriggers(e) => OutputError {
            description: fl!("dpkg-triggers-only-a-non-zero"),
            source: Some(Box::new(e)),
        },
    }
}

impl From<reqwest::Error> for OutputError {
    fn from(e: reqwest::Error) -> Self {
        debug!("{:?}", e);
        let filename = &e
            .url()
            .and_then(|x| x.path_segments())
            .and_then(|x| x.last());

        if e.is_builder() {
            return Self {
                description: fl!("failed-to-create-http-client"),
                source: Some(Box::new(e)),
            };
        }

        if let Some(filename) = filename {
            if filename.len() <= 256 {
                return Self {
                    description: fl!("download-failed", filename = filename.to_string()),
                    source: Some(Box::new(e)),
                };
            }
        }

        Self {
            description: fl!("download-failed-no-name"),
            source: None,
        }
    }
}

fn oma_download_error(e: DownloadError) -> OutputError {
    debug!("{:?}", e);
    match e {
        DownloadError::ChecksumMismatch(filename) => OutputError {
            description: fl!("checksum-mismatch", filename = filename),
            source: None,
        },
        DownloadError::IOError(s, e) => OutputError {
            description: fl!("download-failed", filename = s),
            source: Some(Box::new(e)),
        },
        DownloadError::ReqwestError(e) => OutputError::from(e),
        DownloadError::ChecksumError(e) => oma_checksum_error(e),
        DownloadError::FailedOpenLocalSourceFile(path, e) => OutputError {
            description: fl!("can-not-parse-sources-list", path = path.to_string()),
            source: Some(Box::new(e)),
        },
        DownloadError::InvalidURL(s) => OutputError {
            description: fl!("invalid-url", url = s),
            source: None,
        },
        DownloadError::EmptySources => OutputError {
            description: e.to_string(),
            source: None,
        },
    }
}

fn oma_checksum_error(e: ChecksumError) -> OutputError {
    debug!("{:?}", e);
    match e {
        ChecksumError::FailedToOpenFile(s, e) => OutputError {
            description: fl!("failed-to-open-to-checksum", path = s),
            source: Some(Box::new(e)),
        },
        ChecksumError::ChecksumIOError(e) => OutputError {
            description: fl!("can-not-checksum"),
            source: Some(Box::new(e)),
        },
        ChecksumError::BadLength => OutputError {
            description: fl!("sha256-bad-length"),
            source: None,
        },
        ChecksumError::HexError(e) => OutputError {
            description: e.to_string(),
            source: None,
        },
    }
}

fn oma_database_error(e: MatcherError) -> OutputError {
    debug!("{:?}", e);
    match e {
        MatcherError::AptError(e) => OutputError {
            description: fl!("apt-error"),
            source: Some(Box::new(e)),
        },
        MatcherError::AptErrors(e) => OutputError::from(e),
        MatcherError::AptCxxException(e) => OutputError {
            description: fl!("apt-error"),
            source: Some(Box::new(AptErrors::from(e))),
        },
        MatcherError::InvalidPattern(s) => OutputError {
            description: fl!("invalid-pattern", p = s),
            source: None,
        },
        MatcherError::NoPackage(s) => OutputError {
            description: fl!("can-not-get-pkg-from-database", name = s),
            source: None,
        },
        MatcherError::NoVersion(pkg, ver) => OutputError {
            description: fl!("pkg-unavailable", pkg = pkg, ver = ver),
            source: None,
        },
        MatcherError::NoPath(s) => OutputError {
            description: fl!("invalid-path", p = s),
            source: None,
        },
        MatcherError::NoCandidate(s) => OutputError {
            description: fl!("no-candidate-ver", pkg = s),
            source: None,
        },
        MatcherError::PtrIsNone(_) => OutputError {
            description: e.to_string(),
            source: None,
        },
    }
}

impl From<HistoryError> for OutputError {
    fn from(value: HistoryError) -> Self {
        debug!("{:?}", value);
        match value {
            HistoryError::FailedOperateDirOrFile(s, e) => Self {
                description: fl!("failed-to-operate-path", p = s),
                source: Some(Box::new(e)),
            },
            HistoryError::ConnectError(e) => Self {
                description: fl!("failed-to-connect-history-database"),
                source: Some(Box::new(e)),
            },
            HistoryError::ExecuteError(e) => Self {
                description: fl!("failed-to-execute-query-stmt"),
                source: Some(Box::new(e)),
            },
            HistoryError::ParseError(e) => Self {
                description: fl!("failed-to-parse-history-object"),
                source: Some(Box::new(e)),
            },
            HistoryError::ParseDbError(e) => Self {
                description: fl!("failed-to-parse-history-object"),
                source: Some(Box::new(e)),
            },
            HistoryError::NoResult(id) => Self {
                description: format!("No result by id: {id}"),
                source: None,
            },
            HistoryError::HistoryEmpty => Self {
                description: fl!("oma-history-is-empty"),
                source: None,
            },
            HistoryError::FailedParentPath(p) => Self {
                description: fl!("failed-to-get-parent-path", p = p),
                source: None,
            },
        }
    }
}
