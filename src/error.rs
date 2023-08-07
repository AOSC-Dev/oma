use std::error::Error;
use std::fmt::Display;

use oma_console::console::style;
use oma_console::OmaConsoleError;
use oma_contents::OmaContentsError;
use oma_fetch::checksum::ChecksumError;
use oma_fetch::DownloadError;
use oma_pm::apt::{AptArgsBuilderError, OmaAptArgsBuilderError};
use oma_pm::search::OmaSearchError;
use oma_pm::{apt::OmaAptError, query::OmaDatabaseError};
use oma_refresh::db::RefreshError;
use oma_refresh::decompress::DecompressError;
use oma_refresh::inrelease::InReleaseParserError;
use oma_refresh::verify::VerifyError;
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
        Self(oma_database_error(value))
    }
}

impl From<RefreshError> for OutputError {
    fn from(value: RefreshError) -> Self {
        let s = match value {
            RefreshError::InvaildUrl(s) => fl!("invaild-url", url = s),
            RefreshError::ParseDistroRepoDataError(path, e) => {
                fl!("can-not-parse-sources-list", path = path, e = e)
            }
            RefreshError::ScanSourceError(e) => e,
            RefreshError::UnsupportedProtocol(s) => fl!("unsupport-protocol", url = s),
            RefreshError::FetcherError(e) => oma_download_error(e),
            RefreshError::ReqwestError(e) => e.to_string(),
            RefreshError::TopicsError(e) => todo!(),
            RefreshError::NoInReleaseFile(s) => fl!("not-found", url = s),
            RefreshError::InReleaseParserError(e) => match e {
                InReleaseParserError::FailedToOpenInRelease(p, e) => {
                    fl!("can-nnot-read-inrelease-file", path = p, e = e)
                }
                InReleaseParserError::VerifyError(e) => match e {
                    VerifyError::CertParseFileError(p) => {
                        fl!("fail-load-certs-from-file", path = p)
                    }
                    VerifyError::BadCertFile(p) => {
                        fl!("cert-file-is-bad", path = p)
                    }
                    VerifyError::TrustedDirNotExist => e.to_string(),
                    VerifyError::IOError(e) => {
                        fl!("io-error", e = e.to_string())
                    }
                    VerifyError::Anyhow(e) => e.to_string(),
                },
                InReleaseParserError::BadInReleaseData => todo!(),
                InReleaseParserError::BadInReleaseVaildUntil => todo!(),
                InReleaseParserError::EarlierSignature(p) => fl!("earlier-signature", filename = p),
                InReleaseParserError::ExpiredSignature(p) => fl!("expired-signature", filename = p),
                InReleaseParserError::BadSha256Value(p) => todo!(),
                InReleaseParserError::BadChecksumEntry(line) => {
                    fl!("inrelease-checksum-can-not-parse", i = line)
                }
                InReleaseParserError::InReleaseSyntaxError(_, _) => todo!(),
                InReleaseParserError::UnsupportFileType => {
                    fl!("inrelease-parse-unsupport-file-type")
                }
                InReleaseParserError::SizeShouldIsNumber(e) => e.to_string(),
            },
            RefreshError::DpkgArchError(e) => OutputError::from(e).to_string(),
            RefreshError::JoinError(e) => e.to_string(),
            RefreshError::DecompressError(e) => match e {
                DecompressError::IOError(e) => fl!("io-error", e = e.to_string()),
                DecompressError::UnsupportedFileType(p) => {
                    fl!("unsupport-decompress-file", name = p)
                }
                DecompressError::TemplateError(e) => e.to_string(),
                DecompressError::FileNameError => e.to_string(),
            },
            RefreshError::TemplateError(e) => e.to_string(),
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
            OmaContentsError::ContentsNotExist => fl!(
                "contents-does-not-exist",
                cmd = style("oma refresh").green().to_string()
            ),
            OmaContentsError::ExecuteRgFailed(e) => fl!("execute-ripgrep-failed", e = e),
            OmaContentsError::IOError(e) => OutputError::from(e).to_string(),
            OmaContentsError::RgParseFailed { input, err } => {
                fl!("parse-rg-result-failed", i = input, e = err)
            }
            OmaContentsError::ContentsEntryMissingPathList(e) => {
                fl!("contents-entry-missing-path-list", entry = e)
            }
            OmaContentsError::CnfWrongArgument => value.to_string(),
            OmaContentsError::RgWithError => fl!("rg-non-zero"),
            OmaContentsError::GrepBuilderError(e) => e.to_string(),
            OmaContentsError::NoResult => "".to_string(),
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
