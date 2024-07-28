use chrono::{DateTime, FixedOffset, ParseError, Utc};
use small_map::SmallMap;
use smallvec::{smallvec, SmallVec};
use std::{borrow::Cow, num::ParseIntError, path::Path};
use thiserror::Error;
use tracing::debug;

use crate::verify;

pub struct InReleaseParser {
    _source: Vec<SmallMap<16, String, String>>,
    pub checksums: SmallVec<[ChecksumItem; 32]>,
    pub acquire_by_hash: bool,
    pub checksum_type: ChecksumType,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChecksumItem {
    pub name: String,
    pub size: u64,
    pub checksum: String,
    pub file_type: DistFileType,
}

#[derive(Debug, PartialEq, Clone, Eq)]
pub enum DistFileType {
    BinaryContents,
    Contents,
    CompressContents(String, String),
    PackageList,
    CompressPackageList(String, String),
    Release,
}

#[derive(Debug, thiserror::Error)]
pub enum InReleaseParserError {
    #[error("Mirror {0} is not signed by trusted keyring.")]
    NotTrusted(String),
    #[error(transparent)]
    VerifyError(#[from] crate::verify::VerifyError),
    #[error("Bad InRelease Data")]
    BadInReleaseData,
    #[error("Bad vaild until")]
    BadInReleaseVaildUntil,
    #[error("Earlier signature: {0}")]
    EarlierSignature(String),
    #[error("Expired signature: {0}")]
    ExpiredSignature(String),
    #[error("Bad SHA256 value: {0}")]
    BadChecksumValue(String),
    #[error("Bad checksum entry: {0}")]
    BadChecksumEntry(String),
    #[error("Bad InRelease")]
    InReleaseSyntaxError,
    #[error("Unsupport file type in path")]
    UnsupportFileType,
    #[error(transparent)]
    ParseIntError(ParseIntError),
}

pub type InReleaseParserResult<T> = Result<T, InReleaseParserError>;

pub struct InRelease<'a> {
    pub inrelease: &'a str,
    pub signed_by: Option<&'a str>,
    pub mirror: &'a str,
    pub archs: &'a [String],
    pub is_flat: bool,
    pub p: &'a Path,
    pub rootfs: &'a Path,
    pub components: &'a [String],
    pub trusted: bool,
}

#[derive(Clone, Copy)]
pub enum ChecksumType {
    Sha256,
    Sha512,
    Md5,
}

impl InReleaseParser {
    pub fn new(in_release: InRelease<'_>) -> InReleaseParserResult<Self> {
        let InRelease {
            inrelease: s,
            signed_by,
            mirror,
            archs,
            is_flat,
            p,
            rootfs,
            components,
            trusted,
        } = in_release;

        let s = if s.starts_with("-----BEGIN PGP SIGNED MESSAGE-----") {
            Cow::Owned(verify::verify(s, signed_by, rootfs)?)
        } else {
            if !trusted {
                return Err(InReleaseParserError::NotTrusted(mirror.to_string()));
            }
            Cow::Borrowed(s)
        };

        let source = debcontrol_from_str(&s)?;

        let source_first = source.first();

        debug!("InRelease is: {source:#?}");

        if !is_flat {
            let date = source_first
                .and_then(|x| x.get("Date"))
                .take()
                .ok_or_else(|| InReleaseParserError::BadInReleaseData)?;

            let date = parse_date(date).map_err(|e| {
                debug!("Parse data failed: {}", e);
                InReleaseParserError::BadInReleaseData
            })?;

            let now = Utc::now();

            // Make `Valid-Until` field optional.
            // Some third-party repos do not have such field in their InRelease files.
            let valid_until = source_first.and_then(|x| x.get("Valid-Until")).take();
            if now < date {
                return Err(InReleaseParserError::EarlierSignature(
                    p.display().to_string(),
                ));
            }

            // Check if the `Valid-Until` field is valid only when it is defined.
            if let Some(valid_until_date) = valid_until {
                let valid_until = parse_date(valid_until_date).map_err(|e| {
                    debug!("Parse valid_until failed: {}", e);
                    InReleaseParserError::BadInReleaseVaildUntil
                })?;

                if now > valid_until {
                    return Err(InReleaseParserError::ExpiredSignature(
                        p.display().to_string(),
                    ));
                }
            }
        }

        let acquire_by_hash = source_first
            .and_then(|x| x.get("Acquire-By-Hash"))
            .map(|x| x.to_lowercase() == "yes")
            .unwrap_or(false);

        let sha256 = source_first.and_then(|x| x.get("SHA256")).take();
        let sha512 = source_first.and_then(|x| x.get("SHA512")).take();
        let md5 = source_first.and_then(|x| x.get("MD5Sum")).take();

        let checksum_type = if sha256.is_some() {
            ChecksumType::Sha256
        } else if sha512.is_some() {
            ChecksumType::Sha512
        } else {
            ChecksumType::Md5
        };

        let mut checksums = sha256
            .or(sha512)
            .or(md5)
            .ok_or_else(|| InReleaseParserError::BadChecksumValue(s.to_string()))?
            .lines();

        // remove first item, It's empty
        checksums.next();

        let mut checksums_res = vec![];

        for i in checksums {
            let mut checksum_entry = i.split_whitespace();
            let checksum = checksum_entry
                .next()
                .ok_or_else(|| InReleaseParserError::BadChecksumEntry(i.to_owned()))?;
            let size = checksum_entry
                .next()
                .ok_or_else(|| InReleaseParserError::BadChecksumEntry(i.to_owned()))?;
            let name = checksum_entry
                .next()
                .ok_or_else(|| InReleaseParserError::BadChecksumEntry(i.to_owned()))?;
            checksums_res.push((name, size, checksum));
        }

        let mut res: SmallVec<[_; 32]> = smallvec![];

        let c_res_clone = checksums_res.clone();

        let c = checksums_res
            .into_iter()
            .filter(|(name, _, _)| {
                let mut name_split = name.split('/');
                let component = name_split.next();
                let component_type = name_split.next();

                // debian-installer 是为 Debian 安装器专门准备的源，应该没有人把 oma 用在这种场景上面
                let is_debian_installer = component_type
                    .map(|x| x == "debian-installer")
                    .unwrap_or(false);

                if let Some(c) = component {
                    if c != *name {
                        components.contains(&c.to_string())
                            && ((name.contains("all") || archs.iter().any(|x| name.contains(x)))
                                && !is_debian_installer)
                    } else {
                        name.contains("all") || archs.iter().any(|x| name.contains(x))
                    }
                } else {
                    name.contains("all") || archs.iter().any(|x| name.contains(x))
                }
            })
            .collect::<Vec<_>>();

        let c = if c.is_empty() { c_res_clone } else { c };

        for i in c {
            let t = match i.0 {
                x if x.contains("BinContents") => DistFileType::BinaryContents,
                x if x.contains("Contents-") && file_is_compress(x) && !x.contains("udeb") => {
                    let s = x.split_once('.').unwrap();
                    DistFileType::CompressContents(s.0.to_string(), s.1.to_string())
                }
                x if x.contains("Contents-") && !x.contains('.') && !x.contains("udeb") => {
                    DistFileType::Contents
                }
                x if x.contains("Packages") && !x.contains('.') => DistFileType::PackageList,
                x if x.contains("Packages") && file_is_compress(x) => {
                    let s = x.split_once('.').unwrap();
                    DistFileType::CompressPackageList(s.0.to_string(), s.1.to_string())
                }
                x if x.contains("Release") => DistFileType::Release,
                x => {
                    debug!("Unknown file type: {x:?}");
                    continue;
                }
            };

            res.push(ChecksumItem {
                name: i.0.to_owned(),
                size: i
                    .1
                    .parse::<u64>()
                    .map_err(InReleaseParserError::ParseIntError)?,
                checksum: i.2.to_owned(),
                file_type: t,
            })
        }

        Ok(Self {
            _source: source,
            checksums: res,
            acquire_by_hash,
            checksum_type,
        })
    }
}

fn file_is_compress(name: &str) -> bool {
    name.ends_with(".gz") || name.ends_with(".bz2") || name.ends_with(".xz")
}

#[derive(Debug, Error)]
pub enum ParseDateError {
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

        let mut time_split = c.splitn(2, ':').map(|x| x.to_string()).collect::<Vec<_>>();

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

fn debcontrol_from_str(s: &str) -> InReleaseParserResult<Vec<SmallMap<16, String, String>>> {
    let mut res = vec![];

    let debcontrol =
        oma_debcontrol::parse_str(s).map_err(|_| InReleaseParserError::InReleaseSyntaxError)?;

    for i in debcontrol {
        let mut item = SmallMap::<16, _, _>::new();
        let field = i.fields;

        for j in field {
            item.insert(j.name.to_string(), j.value.to_string());
        }

        res.push(item);
    }

    Ok(res)
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
