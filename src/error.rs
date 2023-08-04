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
use oma_utils::DpkgArchError;

use crate::fl;

#[derive(Debug)]
pub struct OutputError(String);

impl Display for OutputError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

impl Error for OutputError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        None
    }

    fn description(&self) -> &str {
        &self.0
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
        OutputError(value.to_string())
    }
}

impl From<AptArgsBuilderError> for OutputError {
    fn from(value: AptArgsBuilderError) -> Self {
        OutputError(value.to_string())
    }
}

impl From<OmaConsoleError> for OutputError {
    fn from(value: OmaConsoleError) -> Self {
        let s = match value {
            OmaConsoleError::IoError(e) => fl!("io-error", e = e.to_string()),
            OmaConsoleError::StdinDoesNotExist => {
                fl!("io-error", e = "stdin does not exist".to_string())
            }
        };

        Self(s)
    }
}

impl From<OmaDatabaseError> for OutputError {
    fn from(value: OmaDatabaseError) -> Self {
        let s = oma_database_error(value);

        Self(s)
    }
}

impl From<RefreshError> for OutputError {
    fn from(value: RefreshError) -> Self {
        let s = match value {
            RefreshError::InvaildUrl(_) => todo!(),
            RefreshError::ParseDistroRepoDataErrpr(_) => todo!(),
            RefreshError::ReadFileError { path, error } => todo!(),
            RefreshError::ScanSourceError(_) => todo!(),
            RefreshError::UnsupportedProtocol(_) => todo!(),
            RefreshError::FetcherError(_) => todo!(),
            RefreshError::ReqwestError(_) => todo!(),
            RefreshError::TopicsError(_) => todo!(),
            RefreshError::NoInReleaseFile(_) => todo!(),
            RefreshError::InReleaseParserError(_) => todo!(),
            RefreshError::DpkgArchError(_) => todo!(),
            RefreshError::JoinError(_) => todo!(),
            RefreshError::DecompressError(_) => todo!(),
            RefreshError::TemplateError(_) => todo!(),
        };

        Self(s)
    }
}

impl From<std::io::Error> for OutputError {
    fn from(e: std::io::Error) -> Self {
        let s = fl!("io-error", e = e.to_string());

        Self(s)
    }
}

impl From<DpkgArchError> for OutputError {
    fn from(value: DpkgArchError) -> Self {
        let s = fl!("can-not-run-dpkg-print-arch", e = value.to_string());

        Self(s)
    }
}

impl From<OmaContentsError> for OutputError {
    fn from(value: OmaContentsError) -> Self {
        let s = match value {
            OmaContentsError::ContentsNotExist => todo!(),
            OmaContentsError::ExecuteRgFailed(_) => todo!(),
            OmaContentsError::IOError(_) => todo!(),
            OmaContentsError::RgParseFailed { input, err } => todo!(),
            OmaContentsError::ContentsEntryMissingPathList(_) => todo!(),
            OmaContentsError::CnfWrongArgument => todo!(),
            OmaContentsError::RgWithError => todo!(),
            OmaContentsError::GrepBuilderError(_) => todo!(),
            OmaContentsError::NoResult => todo!(),
        };

        Self(s)
    }
}

impl From<anyhow::Error> for OutputError {
    fn from(value: anyhow::Error) -> Self {
        Self(value.to_string())
    }
}

pub fn oma_apt_error_to_output(err: OmaAptError) -> OutputError {
    let err = match err {
        OmaAptError::RustApt(e) => fl!("apt-error", e = e.to_string()),
        OmaAptError::OmaDatabaseError(e) => oma_database_error(e),
        OmaAptError::MarkReinstallError(pkg, version) => {
            fl!("can-not-mark-reinstall", name = pkg, version = version)
        }
        OmaAptError::DependencyIssue(_) => "".to_string(),
        OmaAptError::PkgIsEssential(s) => fl!("pkg-is-essential", name = s),
        OmaAptError::PkgNoCandidate(s) => fl!("no-candidate-ver", pkg = s),
        OmaAptError::PkgNoChecksum(s) => fl!("pkg-no-checksum", name = s),
        OmaAptError::InvalidFileName(s) => fl!("invaild-filename", name = s),
        OmaAptError::DownlaodError(e) => oma_download_error(e),
        OmaAptError::IOError(e) => fl!("io-error", e = e.to_string()),
        OmaAptError::InstallEntryBuilderError(e) => e.to_string(),
        OmaAptError::DpkgFailedConfigure(e) => {
            fl!("dpkg-configure-a-non-zero", e = e)
        }
        OmaAptError::DiskSpaceInsufficient(need, avail) => {
            fl!(
                "need-more-size",
                a = avail.to_string(),
                n = need.to_string()
            )
        }
    };

    OutputError(err)
}

fn oma_download_error(e: DownloadError) -> String {
    match e {
        DownloadError::ChecksumMisMatch(url, dir) => {
            fl!("checksum-mismatch", filename = url, dir = dir)
        }
        DownloadError::NotFound(s) => fl!("not-found-other", url = s),
        DownloadError::IOError(e) => fl!("io-error", e = e.to_string()),
        DownloadError::ReqwestError(e) => format!("Reqwest Error: {e}"),
        DownloadError::ChecksumError(e) => oma_checksum_error(e),
        DownloadError::TemplateError(e) => e.to_string(),
        DownloadError::FailedOpenLocalSourceFile(path, e) => {
            fl!("can-not-parse-sources-list", path = path, e = e)
        }
        DownloadError::DownloadAllFailed(s, e) => {
            fl!("can-not-get-file", name = s, e = e)
        }
    }
}

fn oma_checksum_error(e: ChecksumError) -> String {
    match e {
        ChecksumError::FailedToOpenFile(s) => fl!("failed-to-open-to-checksum", path = s),
        ChecksumError::ChecksumIOError(e) => fl!("can-not-checksum", e = e),
        ChecksumError::BadLength => fl!("sha256-bad-length"),
        ChecksumError::HexError(e) => e.to_string(),
    }
}

fn oma_database_error(e: OmaDatabaseError) -> String {
    match e {
        OmaDatabaseError::RustApt(e) => fl!("apt-error", e = e.to_string()),
        OmaDatabaseError::InvaildPattern(s) => fl!("invaild-pattern", p = s),
        OmaDatabaseError::NoPackage(s) => fl!("can-not-get-pkg-from-database", name = s),
        OmaDatabaseError::NoVersion(pkg, ver) => fl!(
            "can-not-get-pkg-version-from-database",
            name = pkg,
            version = ver
        ),
        OmaDatabaseError::NoPath(s) => fl!("invaild-path", p = s),
        OmaDatabaseError::OmaSearchError(e) => match e {
            OmaSearchError::RustApt(e) => fl!("apt-error", e = e.to_string()),
            OmaSearchError::NoResult(e) => fl!("could-not-find-pkg-from-keyword", c = e),
            OmaSearchError::FailedGetCandidate(s) => fl!("no-candidate-ver", pkg = s),
        },
    }
}
