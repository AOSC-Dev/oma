use chrono::{DateTime, FixedOffset, ParseError, Utc};
use deb822_fast::{FromDeb822, FromDeb822Paragraph, Paragraph};
use oma_apt_sources_lists::Signature;
use oma_repo_verify::verify_release_by_sysroot;
use once_cell::sync::OnceCell;
use std::{
    borrow::Cow,
    fs,
    io::{self, ErrorKind},
    num::ParseIntError,
    path::Path,
    str::FromStr,
};
use thiserror::Error;
use tracing::{debug, trace};

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ChecksumItem {
    pub name: String,
    pub size: u64,
    pub checksum: String,
}

#[derive(Debug, thiserror::Error)]
pub enum InReleaseError {
    #[error("Mirror is not signed by trusted keyring.")]
    NotTrusted,
    #[error(transparent)]
    VerifyError(#[from] oma_repo_verify::VerifyError),
    #[error("Bad InRelease Data")]
    BadInReleaseData,
    #[error("Bad valid until")]
    BadInReleaseValidUntil,
    #[error("Earlier signature")]
    EarlierSignature,
    #[error("Expired signature")]
    ExpiredSignature,
    #[error("Bad InRelease")]
    InReleaseSyntaxError,
    #[error("Unsupported file type in path")]
    UnsupportedFileType,
    #[error(transparent)]
    ParseIntError(ParseIntError),
    #[error("InRelease is broken")]
    BrokenInRelease,
    #[error("Failed to read release.gpg file: {1}")]
    ReadGPGFileName(std::io::Error, String),
}

pub type InReleaseParserResult<T> = Result<T, InReleaseError>;

#[derive(Clone, Copy)]
pub enum InReleaseChecksum {
    Sha256,
    Sha512,
    Md5,
}

const COMPRESS: &[&str] = &[".gz", ".xz", ".zst", ".bz2"];

pub struct Release {
    source: InReleaseEntry,
    acquire_by_hash: OnceCell<bool>,
    checksum_type_and_list: OnceCell<(InReleaseChecksum, Vec<ChecksumItem>)>,
}

#[derive(Debug, FromDeb822)]
struct InReleaseEntry {
    #[deb822(field = "Date")]
    date: Option<String>,
    #[deb822(field = "Valid-Until")]
    valid_until: Option<String>,
    #[deb822(field = "Acquire-By-Hash")]
    acquire_by_hash: Option<String>,
    #[deb822(field = "MD5Sum")]
    md5sum: Option<String>,
    #[deb822(field = "SHA256")]
    sha256: Option<String>,
    #[deb822(field = "SHA512")]
    sha512: Option<String>,
}

impl FromStr for Release {
    type Err = InReleaseError;

    fn from_str(input: &str) -> Result<Self, Self::Err> {
        let source: Paragraph = input.parse().map_err(|_| InReleaseError::BrokenInRelease)?;
        let source: InReleaseEntry = FromDeb822Paragraph::from_paragraph(&source)
            .map_err(|_| InReleaseError::BrokenInRelease)?;

        Ok(Self {
            source,
            acquire_by_hash: OnceCell::new(),
            checksum_type_and_list: OnceCell::new(),
        })
    }
}

impl Release {
    pub fn get_or_try_init_checksum_type_and_list(
        &self,
    ) -> Result<&(InReleaseChecksum, Vec<ChecksumItem>), InReleaseError> {
        self.checksum_type_and_list.get_or_try_init(|| {
            let (checksum_type, checksums) = if let Some(sha256) = &self.source.sha256 {
                (InReleaseChecksum::Sha256, get_checksums_inner(sha256)?)
            } else if let Some(sha512) = &self.source.sha512 {
                (InReleaseChecksum::Sha512, get_checksums_inner(sha512)?)
            } else if let Some(md5) = &self.source.md5sum {
                (InReleaseChecksum::Md5, get_checksums_inner(md5)?)
            } else {
                return Err(InReleaseError::BrokenInRelease);
            };

            Ok((checksum_type, checksums))
        })
    }

    pub fn checksum_type_and_list(&self) -> &(InReleaseChecksum, Vec<ChecksumItem>) {
        self.get_or_try_init_checksum_type_and_list()
            .expect("checksum type and list does not init")
    }

    pub fn acquire_by_hash(&self) -> bool {
        *self.acquire_by_hash.get_or_init(|| {
            self.source
                .acquire_by_hash
                .as_ref()
                .is_some_and(|x| x.eq_ignore_ascii_case("yes"))
        })
    }

    pub fn check_date(&self, now: &DateTime<Utc>) -> Result<(), InReleaseError> {
        let date = self
            .source
            .date
            .as_ref()
            .ok_or(InReleaseError::BadInReleaseData)?;

        let date = parse_date(date).map_err(|e| {
            debug!("Parse data failed: {}", e);
            InReleaseError::BadInReleaseData
        })?;

        if now < &date {
            return Err(InReleaseError::EarlierSignature);
        }

        Ok(())
    }

    pub fn check_valid_until(&self, now: &DateTime<Utc>) -> Result<(), InReleaseError> {
        // Check if the `Valid-Until` field is valid only when it is defined.
        if let Some(valid_until_date) = &self.source.valid_until {
            let valid_until = parse_date(valid_until_date).map_err(|e| {
                debug!("Parse valid_until failed: {}", e);
                InReleaseError::BadInReleaseValidUntil
            })?;

            if now > &valid_until {
                return Err(InReleaseError::ExpiredSignature);
            }
        }

        Ok(())
    }
}

impl FromStr for ChecksumItem {
    type Err = InReleaseError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        trace!("Parsing line: {s}");

        let mut line = s.split_ascii_whitespace();

        let checksum = line
            .next()
            .ok_or(InReleaseError::BrokenInRelease)?
            .to_string();

        trace!("checksum is: {checksum}");

        let size = line.next().ok_or(InReleaseError::BrokenInRelease)?;

        trace!("size is: {size}");

        let size = size.parse::<u64>().map_err(InReleaseError::ParseIntError)?;

        let name = line
            .next()
            .ok_or(InReleaseError::BrokenInRelease)?
            .to_string();

        if line.next().is_some() {
            return Err(InReleaseError::BrokenInRelease);
        }

        Ok(Self {
            name,
            size,
            checksum,
        })
    }
}

fn get_checksums_inner(checksum_str: &str) -> Result<Vec<ChecksumItem>, InReleaseError> {
    checksum_str
        .trim()
        .lines()
        .map(ChecksumItem::from_str)
        .collect::<Result<Vec<_>, InReleaseError>>()
}

pub fn verify_inrelease<'a>(
    inrelease: &'a str,
    signed_by: Option<&Signature>,
    rootfs: impl AsRef<Path>,
    file: impl AsRef<Path>,
    trusted: bool,
) -> Result<Cow<'a, str>, InReleaseError> {
    if inrelease.starts_with("-----BEGIN PGP SIGNED MESSAGE-----") {
        Ok(Cow::Owned(oma_repo_verify::verify_inrelease_by_sysroot(
            inrelease, signed_by, rootfs, trusted,
        )?))
    } else {
        if trusted {
            return Ok(Cow::Borrowed(inrelease));
        }

        let inrelease_path = file.as_ref();

        let mut file_name = inrelease_path
            .file_name()
            .map(|x| x.to_string_lossy().to_string())
            .ok_or_else(|| {
                InReleaseError::ReadGPGFileName(
                    io::Error::new(ErrorKind::InvalidInput, "Failed to get file name"),
                    inrelease_path.display().to_string(),
                )
            })?;

        file_name.push_str(".gpg");

        let pub_file = inrelease_path.with_file_name(&file_name);

        debug!("Read GPG file: {}", pub_file.display());
        let bytes = fs::read(pub_file)
            .map_err(|e| InReleaseError::ReadGPGFileName(e, file_name.to_string()))?;

        verify_release_by_sysroot(inrelease, &bytes, signed_by, rootfs, trusted).map_err(|e| {
            debug!("{e}");
            InReleaseError::NotTrusted
        })?;

        Ok(Cow::Borrowed(inrelease))
    }
}

pub(crate) fn split_ext_and_filename(x: &str) -> (Cow<'_, str>, String) {
    let path = Path::new(x);
    let ext = path.extension().unwrap_or_default().to_string_lossy();
    let name = path.with_extension("");
    let name = name.to_string_lossy().to_string();

    (ext, name)
}

pub(crate) fn file_is_compress(name: &str) -> bool {
    for i in COMPRESS {
        if name.ends_with(i) {
            return true;
        }
    }

    false
}

#[derive(Debug, Error)]
enum ParseDateError {
    #[error(transparent)]
    ParseError(#[from] ParseError),
    #[error("Could not parse date: {0}")]
    BadDate(ParseIntError),
}

fn parse_date(date: &str) -> Result<DateTime<FixedOffset>, ParseDateError> {
    match DateTime::parse_from_rfc2822(date) {
        Ok(res) => Ok(res),
        Err(e) => {
            debug!("Parse {} failed: {e}, try to use date hack.", date);
            let hack_date = date_hack(date).map_err(ParseDateError::BadDate)?;
            Ok(DateTime::parse_from_rfc2822(&hack_date)?)
        }
    }
}

/// Replace RFC 1123/822/2822 non-compliant "UTC" marker with RFC 2822-compliant "+0000" whilst parsing InRelease.
/// and for non-standard X:YY:ZZ conversion to XX:YY:ZZ.
///
/// - Some third-party repositories (such as those generated with Aptly) uses "UTC" to denote the Coordinated Universal
///   Time, which is not allowed in RFC 1123 or 822/2822 (all calls for "GMT" or "UT", 822 allows "Z", and 2822 allows
///   "+0000").
/// - This is used by many commercial software vendors, such as Google, Microsoft, and Spotify.
/// - This is allowed in APT's RFC 1123 parser. However, as chrono requires full compliance with the
///   aforementioned RFC documents, "UTC" is considered illegal.
///
/// Replace the "UTC" marker at the end of date strings to make it palatable to chronos.
///
/// and for non-standard X:YY:ZZ conversion to XX:YY:ZZ to make it palatable to chronos.
fn date_hack(date: &str) -> Result<String, ParseIntError> {
    let mut split_time = date
        .split_ascii_whitespace()
        .map(|x| x.to_string())
        .collect::<Vec<_>>();

    for c in split_time.iter_mut() {
        if c.is_empty() || !c.contains(':') {
            continue;
        }

        let mut time_split = c.split(':').map(|x| x.to_string()).collect::<Vec<_>>();

        // X:YY:ZZ conversion to XX:YY:ZZ to make it palatable to chronos
        for k in time_split.iter_mut() {
            match k.parse::<u64>()? {
                0..=9 if k.len() == 1 => {
                    *k = "0".to_string() + k;
                }
                _ => continue,
            }
        }

        *c = time_split.join(":");
    }

    let date = split_time.join(" ");

    Ok(date.replace("UTC", "+0000"))
}

#[test]
fn test_date_hack() {
    let a = "Thu, 02 May 2024  9:58:03 UTC";
    let hack = date_hack(&a).unwrap();
    assert_eq!(hack, "Thu, 02 May 2024 09:58:03 +0000");
    let b = DateTime::parse_from_rfc2822(&hack);
    assert!(b.is_ok());

    let a = "Thu, 02 May 2024 09:58:03 +0000";
    let hack = date_hack(&a).unwrap();
    assert_eq!(hack, "Thu, 02 May 2024 09:58:03 +0000");
    let b = DateTime::parse_from_rfc2822(&hack);
    assert!(b.is_ok());

    let a = "Thu, 02 May 2024  0:58:03 +0000";
    let hack = date_hack(&a).unwrap();
    assert_eq!(hack, "Thu, 02 May 2024 00:58:03 +0000");
    let b = DateTime::parse_from_rfc2822(&hack);
    assert!(b.is_ok());
}

#[test]
fn test_split_name_and_ext() {
    let example1 = "main/dep11/icons-128x128.tar.gz";
    let res = split_ext_and_filename(&example1);
    assert_eq!(
        res,
        ("gz".into(), "main/dep11/icons-128x128.tar".to_string())
    );

    let example2 = "main/i18n/Translation-bg.xz";
    let res = split_ext_and_filename(&example2);
    assert_eq!(res, ("xz".into(), "main/i18n/Translation-bg".to_string()));

    let example2 = "main/i18n/Translation-bg";
    let res = split_ext_and_filename(&example2);
    assert_eq!(res, ("".into(), "main/i18n/Translation-bg".to_string()));
}

#[test]
fn test_checksum_parse() {
    let entry = "87c803ffdc2655fd4df8779707ae7713b8e1e2dba44fea4a68b4783b7d8aa6c9           392728 Contents-amd64";
    assert_eq!(
        ChecksumItem::from_str(entry).unwrap(),
        ChecksumItem {
            name: "Contents-amd64".to_string(),
            size: 392728,
            checksum: "87c803ffdc2655fd4df8779707ae7713b8e1e2dba44fea4a68b4783b7d8aa6c9"
                .to_string()
        }
    );
}
