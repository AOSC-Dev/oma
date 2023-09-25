use std::error::Error;
use std::fmt::Display;

use oma_console::OmaConsoleError;
use oma_contents::OmaContentsError;
use oma_fetch::checksum::ChecksumError;
use oma_fetch::DownloadError;
use oma_pm::apt::{AptArgsBuilderError, OmaAptArgsBuilderError};
use oma_pm::search::OmaSearchError;
use oma_pm::{apt::OmaAptError, query::OmaDatabaseError};
use oma_refresh::db::RefreshError;
use oma_refresh::inrelease::InReleaseParserError;
use oma_refresh::verify::VerifyError;
use oma_utils::dbus::zError;
use oma_utils::dpkg::DpkgError;

#[cfg(feature = "aosc")]
use oma_topics::OmaTopicsError;

use crate::fl;
use crate::table::handle_unmet_dep;

#[derive(Debug)]
pub struct OutputError((String, Option<String>));

impl OutputError {
    pub fn inner(self) -> (String, Option<String>) {
        self.0
    }

    pub fn new(error: String, due_to: Option<String>) -> OutputError {
        OutputError((error, due_to))
    }
}

impl Display for OutputError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let (err, due_to) = &self.0;
        f.write_str(err)?;

        if let Some(due_to) = due_to {
            f.write_str(&format!(" (due to: {due_to})"))?;
        }

        Ok(())
    }
}

impl Error for OutputError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        None
    }

    fn description(&self) -> &str {
        let (err, _) = &self.0;

        err
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

impl From<OmaAptArgsBuilderError> for OutputError {
    fn from(value: OmaAptArgsBuilderError) -> Self {
        OutputError((value.to_string(), None))
    }
}

impl From<zError> for OutputError {
    fn from(value: zError) -> Self {
        Self((value.to_string(), None))
    }
}

impl From<AptArgsBuilderError> for OutputError {
    fn from(value: AptArgsBuilderError) -> Self {
        OutputError((value.to_string(), None))
    }
}

impl From<OmaConsoleError> for OutputError {
    fn from(value: OmaConsoleError) -> Self {
        let s = match value {
            OmaConsoleError::IoError(e) => (fl!("io-error", e = e.to_string()), None),
            OmaConsoleError::StdinDoesNotExist => (
                fl!("io-error", e = "stdin does not exist".to_string()),
                None,
            ),
        };

        Self(s)
    }
}

impl From<OmaDatabaseError> for OutputError {
    fn from(value: OmaDatabaseError) -> Self {
        Self(oma_database_error(value))
    }
}

impl From<RefreshError> for OutputError {
    fn from(value: RefreshError) -> Self {
        let s = match value {
            RefreshError::InvaildUrl(s) => (fl!("invaild-url", url = s), Some(fl!("bug"))),
            RefreshError::ParseDistroRepoDataError(path, e) => (
                fl!("can-not-parse-sources-list", path = path, e = e),
                Some(fl!("check-sources-list")),
            ),
            RefreshError::ScanSourceError(e) => (e, None),
            RefreshError::UnsupportedProtocol(s) => (
                fl!("unsupport-protocol", url = s),
                Some(fl!("support-protocol")),
            ),
            RefreshError::FetcherError(e) => oma_download_error(e),
            RefreshError::ReqwestError(e) => (e.to_string(), Some(fl!("check-network-settings"))),
            #[cfg(feature = "aosc")]
            RefreshError::TopicsError(e) => oma_topics_error(e),
            #[cfg(not(feature = "aosc"))]
            RefreshError::TopicsError(_) => unreachable!(),
            RefreshError::NoInReleaseFile(s) => (
                fl!("not-found", url = s),
                Some(fl!("check-network-settings")),
            ),
            RefreshError::InReleaseParserError(e) => match e {
                InReleaseParserError::FailedToOpenInRelease(p, e) => (
                    fl!("can-nnot-read-inrelease-file", path = p, e = e),
                    Some(fl!("mirror-data-maybe-broken")),
                ),
                InReleaseParserError::VerifyError(e) => match e {
                    VerifyError::CertParseFileError(p) => (
                        fl!("fail-load-certs-from-file", path = p),
                        Some(fl!("mirror-data-maybe-broken")),
                    ),
                    VerifyError::BadCertFile(p) => (
                        fl!("cert-file-is-bad", path = p),
                        Some(fl!("mirror-data-maybe-broken")),
                    ),
                    VerifyError::TrustedDirNotExist => {
                        (e.to_string(), Some(fl!("mirror-data-maybe-broken")))
                    }
                    VerifyError::IOError(e) => (fl!("io-error", e = e.to_string()), None),
                    VerifyError::Anyhow(e) => (e.to_string(), None),
                },
                InReleaseParserError::BadInReleaseData => (
                    fl!("can-not-parse-date"),
                    Some(fl!("mirror-data-maybe-broken")),
                ),
                InReleaseParserError::BadInReleaseVaildUntil => (
                    fl!("can-not-parse-valid-until"),
                    Some(fl!("mirror-data-maybe-broken")),
                ),
                InReleaseParserError::EarlierSignature(p) => (
                    fl!("earlier-signature", filename = p),
                    Some(fl!("mirror-data-maybe-expire")),
                ),
                InReleaseParserError::ExpiredSignature(p) => (
                    fl!("expired-signature", filename = p),
                    Some(fl!("mirror-data-maybe-expire")),
                ),
                InReleaseParserError::BadSha256Value(_) => (
                    fl!("inrelease-sha256-empty"),
                    Some(fl!("mirror-data-maybe-broken")),
                ),
                InReleaseParserError::BadChecksumEntry(line) => (
                    fl!("inrelease-checksum-can-not-parse", i = line),
                    Some(fl!("mirror-data-maybe-broken")),
                ),
                InReleaseParserError::InReleaseSyntaxError(p, e) => (
                    fl!("inrelease-syntax-error", path = p, e = e),
                    Some(fl!("mirror-data-maybe-broken")),
                ),
                InReleaseParserError::UnsupportFileType => {
                    (fl!("inrelease-parse-unsupport-file-type"), Some(fl!("bug")))
                }
                InReleaseParserError::SizeShouldIsNumber(e) => (e.to_string(), Some(fl!("bug"))),
            },
            RefreshError::DpkgArchError(e) => {
                let (err, dueto) = OutputError::from(e).0;
                (err, dueto)
            }
            RefreshError::JoinError(e) => (e.to_string(), None),
            RefreshError::DownloadEntryBuilderError(e) => (e.to_string(), None),
            RefreshError::ChecksumError(e) => oma_checksum_error(e),
            RefreshError::IOError(e) => {
                let (err, dueto) = OutputError::from(e).0;
                (err, dueto)
            }
        };

        Self(s)
    }
}

#[cfg(feature = "aosc")]
impl From<OmaTopicsError> for OutputError {
    fn from(value: OmaTopicsError) -> Self {
        Self(oma_topics_error(value))
    }
}

#[cfg(feature = "aosc")]
fn oma_topics_error(e: OmaTopicsError) -> (String, Option<String>) {
    match e {
        OmaTopicsError::SerdeError(e) => (e.to_string(), Some(fl!("bug"))),
        OmaTopicsError::IOError(e) => (OutputError::from(e).0 .0.to_string(), None),
        OmaTopicsError::CanNotFindTopic(topic) => (
            fl!("can-not-find-specified-topic", topic = topic),
            Some(fl!("maybe-mirror-syncing")),
        ),
        OmaTopicsError::FailedToEnableTopic(topic) => (
            fl!("failed-to-enable-following-topics", topic = topic),
            None,
        ),
        OmaTopicsError::ReqwestError(e) => (e.to_string(), Some(fl!("check-network-settings"))),
        OmaTopicsError::SoutceListError(e) => (e.to_string(), Some(fl!("check-sources-list"))),
    }
}

impl From<std::io::Error> for OutputError {
    fn from(e: std::io::Error) -> Self {
        let s = (fl!("io-error", e = e.to_string()), None);

        Self(s)
    }
}

impl From<DpkgError> for OutputError {
    fn from(value: DpkgError) -> Self {
        let s = (
            fl!("can-not-run-dpkg-print-arch", e = value.to_string()),
            Some(fl!("dpkg-data-is-broken")),
        );

        Self(s)
    }
}

impl From<DownloadError> for OutputError {
    fn from(value: DownloadError) -> Self {
        let s = oma_download_error(value);

        Self(s)
    }
}

impl From<OmaContentsError> for OutputError {
    fn from(value: OmaContentsError) -> Self {
        #[cfg(feature = "contents-without-rg")]
        let s = match value {
            OmaContentsError::ContentsNotExist => (
                fl!("contents-does-not-exist"),
                Some(fl!("contents-does-not-exist-dueto")),
            ),
            OmaContentsError::IOError(e) => (OutputError::from(e).to_string(), None),
            OmaContentsError::RgParseFailed { input, err } => (
                fl!("parse-rg-result-failed", i = input, e = err),
                Some(fl!("bug")),
            ),
            OmaContentsError::ContentsEntryMissingPathList(e) => {
                (fl!("contents-entry-missing-path-list", entry = e), None)
            }
            OmaContentsError::CnfWrongArgument => (value.to_string(), Some(fl!("bug"))),
            OmaContentsError::RgWithError => (fl!("rg-non-zero"), None),
            OmaContentsError::NoResult => ("".to_string(), None),
            OmaContentsError::WhichError(e) => (e.to_string(), None),
            OmaContentsError::LzzzErr(e) => (e.to_string(), None),
        };

        #[cfg(not(feature = "contents-without-rg"))]
        let s = match value {
            OmaContentsError::ContentsNotExist => (
                fl!("contents-does-not-exist"),
                Some(fl!("contents-does-not-exist-dueto")),
            ),
            OmaContentsError::ExecuteRgFailed(e) => (
                fl!("execute-ripgrep-failed", e = e),
                Some(fl!("ripgrep-right-installed")),
            ),
            OmaContentsError::IOError(e) => (OutputError::from(e).to_string(), None),
            OmaContentsError::RgParseFailed { input, err } => (
                fl!("parse-rg-result-failed", i = input, e = err),
                Some(fl!("bug")),
            ),
            OmaContentsError::ContentsEntryMissingPathList(e) => {
                (fl!("contents-entry-missing-path-list", entry = e), None)
            }
            OmaContentsError::CnfWrongArgument => (value.to_string(), Some(fl!("bug"))),
            OmaContentsError::RgWithError => (fl!("rg-non-zero"), None),
            OmaContentsError::NoResult => ("".to_string(), None),
            OmaContentsError::WhichError(e) => (e.to_string(), None),
        };

        Self(s)
    }
}

impl From<anyhow::Error> for OutputError {
    fn from(value: anyhow::Error) -> Self {
        Self((value.to_string(), None))
    }
}

pub fn oma_apt_error_to_output(err: OmaAptError) -> OutputError {
    let err = match err {
        OmaAptError::RustApt(e) => (fl!("apt-error", e = e.to_string()), None),
        OmaAptError::OmaDatabaseError(e) => oma_database_error(e),
        OmaAptError::MarkReinstallError(pkg, version) => (
            fl!("can-not-mark-reinstall", name = pkg, version = version),
            Some(fl!("reinstall-failed-info")),
        ),
        OmaAptError::DependencyIssue(ref v) => match v {
            v if v.is_empty() || handle_unmet_dep(v).is_err() => {
                (err.to_string(), Some(fl!("bug")))
            }
            _ => ("".to_string(), None),
        },
        OmaAptError::PkgIsEssential(s) => (fl!("pkg-is-essential", name = s), None),
        OmaAptError::PkgNoCandidate(s) => (fl!("no-candidate-ver", pkg = s), None),
        OmaAptError::PkgNoChecksum(s) => (fl!("pkg-no-checksum", name = s), None),
        OmaAptError::InvalidFileName(s) => (fl!("invaild-filename", name = s), None),
        OmaAptError::DownlaodError(e) => oma_download_error(e),
        OmaAptError::IOError(e) => (fl!("io-error", e = e.to_string()), None),
        OmaAptError::InstallEntryBuilderError(e) => (e.to_string(), None),
        OmaAptError::DpkgFailedConfigure(e) => (fl!("dpkg-configure-a-non-zero", e = e), None),
        OmaAptError::DiskSpaceInsufficient(need, avail) => (
            fl!(
                "need-more-size",
                a = avail.to_string(),
                n = need.to_string()
            ),
            Some(fl!("clean-storage")),
        ),
        OmaAptError::DownloadEntryBuilderError(e) => (e.to_string(), None),
        OmaAptError::Anyhow(e) => (e.to_string(), None),
        OmaAptError::MarkPkgNotInstalled(pkg) => (fl!("pkg-is-not-installed", pkg = pkg), None),
        OmaAptError::DpkgError(e) => (OutputError::from(e).to_string(), None),
    };

    OutputError(err)
}

fn oma_download_error(e: DownloadError) -> (String, Option<String>) {
    match e {
        DownloadError::ChecksumMisMatch(url, dir) => (
            fl!("checksum-mismatch", filename = url, dir = dir),
            Some(fl!("check-network-settings")),
        ),
        DownloadError::NotFound(s) => (
            fl!("not-found-other", url = s),
            Some(fl!("maybe-mirror-syncing")),
        ),
        DownloadError::IOError(e) => (fl!("io-error", e = e.to_string()), None),
        DownloadError::ReqwestError(e) => (
            format!("Reqwest Error: {e}"),
            Some(fl!("check-network-settings")),
        ),
        DownloadError::ChecksumError(e) => oma_checksum_error(e),
        DownloadError::FailedOpenLocalSourceFile(path, e) => (
            fl!("can-not-parse-sources-list", path = path.to_string(), e = e),
            Some(fl!("check-sources-list")),
        ),
        DownloadError::DownloadAllFailed(s, e) => (
            fl!("can-not-get-file", name = s, e = e),
            Some(fl!("check-network-settings")),
        ),
        DownloadError::DownloadSourceBuilderError(e) => (e.to_string(), None),
        DownloadError::InvaildURL(s) => (
            fl!("invaild-url", url = s),
            Some(fl!("mirror-data-maybe-broken")),
        ),
    }
}

fn oma_checksum_error(e: ChecksumError) -> (String, Option<String>) {
    match e {
        ChecksumError::FailedToOpenFile(s) => (fl!("failed-to-open-to-checksum", path = s), None),
        ChecksumError::ChecksumIOError(e) => (fl!("can-not-checksum", e = e), None),
        ChecksumError::BadLength => (
            fl!("sha256-bad-length"),
            Some(fl!("check-network-settings")),
        ),
        ChecksumError::HexError(e) => (e.to_string(), None),
    }
}

fn oma_database_error(e: OmaDatabaseError) -> (String, Option<String>) {
    match e {
        OmaDatabaseError::RustApt(e) => (fl!("apt-error", e = e.to_string()), None),
        OmaDatabaseError::InvaildPattern(s) => {
            (fl!("invaild-pattern", p = s), Some(fl!("right-pattern")))
        }
        OmaDatabaseError::NoPackage(s) => (fl!("can-not-get-pkg-from-database", name = s), None),
        OmaDatabaseError::NoVersion(pkg, ver) => (
            fl!(
                "can-not-get-pkg-version-from-database",
                name = pkg,
                version = ver
            ),
            None,
        ),
        OmaDatabaseError::NoPath(s) => (fl!("invaild-path", p = s), None),
        OmaDatabaseError::OmaSearchError(e) => match e {
            OmaSearchError::RustApt(e) => (fl!("apt-error", e = e.to_string()), None),
            OmaSearchError::NoResult(e) => (fl!("could-not-find-pkg-from-keyword", c = e), None),
            OmaSearchError::FailedGetCandidate(s) => (fl!("no-candidate-ver", pkg = s), None),
        },
        OmaDatabaseError::NoCandidate(s) => (fl!("no-candidate-ver", pkg = s), None),
    }
}
