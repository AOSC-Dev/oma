use chrono::{DateTime, Utc};
use small_map::SmallMap;
use smallvec::{smallvec, SmallVec};
use std::{borrow::Cow, num::ParseIntError, path::Path};

use crate::verify;

pub struct InReleaseParser {
    _source: Vec<SmallMap<16, String, String>>,
    pub checksums: SmallVec<[ChecksumItem; 32]>,
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
    CompressContents(String),
    PackageList,
    CompressPackageList(String),
    Release,
}

#[derive(Debug, thiserror::Error)]
pub enum InReleaseParserError {
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
    BadSha256Value(String),
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

impl InReleaseParser {
    pub fn new<P: AsRef<Path>>(
        s: &str,
        trust_files: Option<&str>,
        mirror: &str,
        arch: &str,
        is_flat: bool,
        p: P,
        rootfs: P,
    ) -> InReleaseParserResult<Self> {
        let p = p.as_ref();
        let s = if s.starts_with("-----BEGIN PGP SIGNED MESSAGE-----") {
            Cow::Owned(verify::verify(s, trust_files, mirror, rootfs)?)
        } else {
            Cow::Borrowed(s)
        };

        let source = debcontrol_from_str(&s)?;

        let source_first = source.first();

        if !is_flat {
            let date = source_first
                .and_then(|x| x.get("Date"))
                .take()
                .ok_or_else(|| InReleaseParserError::BadInReleaseData)?
                .clone();
            let date = DateTime::parse_from_rfc2822(&utc_tzname_quirk(&date))
                .map_err(|_| InReleaseParserError::BadInReleaseData)?;

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
            if let Some(valid_until_data) = valid_until {
                let valid_until = DateTime::parse_from_rfc2822(&utc_tzname_quirk(valid_until_data))
                    .map_err(|_| InReleaseParserError::BadInReleaseData)?;
                if now > valid_until {
                    return Err(InReleaseParserError::ExpiredSignature(
                        p.display().to_string(),
                    ));
                }
            }
        }

        let sha256 = source_first
            .and_then(|x| x.get("SHA256"))
            .take()
            .ok_or_else(|| InReleaseParserError::BadSha256Value(p.display().to_string()))?;

        let mut checksums = sha256.split('\n');

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
            .filter(|(name, _, _)| name.contains("all") || name.contains(arch))
            .collect::<Vec<_>>();

        let c = if c.is_empty() { c_res_clone } else { c };

        for i in c {
            let t = match i.0 {
                x if x.contains("BinContents") => DistFileType::BinaryContents,
                x if x.contains("/Contents-") && x.contains('.') => {
                    DistFileType::CompressContents(x.split_once('.').unwrap().0.to_string())
                }
                x if x.contains("/Contents-") && !x.contains('.') => DistFileType::Contents,
                x if x.contains("Packages") && !x.contains('.') => DistFileType::PackageList,
                x if x.contains("Packages") && x.contains('.') => {
                    DistFileType::CompressPackageList(x.split_once('.').unwrap().0.to_string())
                }
                x if x.contains("Release") => DistFileType::Release,
                _ => {
                    return Err(InReleaseParserError::UnsupportFileType);
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
        })
    }
}

/// Replace RFC 1123/822/2822 non-compliant "UTC" marker with RFC 2822-compliant "+0000" whilst parsing InRelease.
///
/// - Some third-party repositories (such as those generated with Aptly) uses "UTC" to denote the Coordinated Universal
/// Time, which is not allowed in RFC 1123 or 822/2822 (all calls for "GMT" or "UT", 822 allows "Z", and 2822 allows
/// "+0000").
/// - This is used by many commercial software vendors, such as Google, Microsoft, and Spotify.
/// - This is allowed in APT's RFC 1123 parser. However, as chrono requires full compliance with the
/// aforementioned RFC documents, "UTC" is considered illegal.
///
/// Replace the "UTC" marker at the end of date strings to make it palatable to chronos.
fn utc_tzname_quirk(date: &String) -> String {
    if date.ends_with("UTC") {
        return date.replace("UTC", "+0000");
    }

    date.to_string()
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
