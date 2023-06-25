use std::{collections::HashMap, path::Path};

use time::{format_description::well_known::Rfc2822, OffsetDateTime};

use crate::verify;

#[derive(Debug)]
pub struct InReleaseParser {
    _source: Vec<HashMap<String, String>>,
    pub checksums: Vec<ChecksumItem>,
}

#[derive(Debug, Clone)]
struct ChecksumItem {
    pub name: String,
    pub size: u64,
    checksum: String,
    pub file_type: DistFileType,
}

#[derive(Debug, PartialEq, Clone)]
pub enum DistFileType {
    BinaryContents,
    Contents,
    CompressContents,
    PackageList,
    CompressPackageList,
    Release,
}

#[derive(Debug, thiserror::Error)]
pub enum InReleaseParserError {
    #[error("Failed to open InRelease {0}: {1}")]
    FailedToOpenInRelease(String, String),
    #[error(transparent)]
    VerifyError(#[from] crate::verify::VerifyError),
    #[error("Bad InRelease Data")]
    BadInReleaseData,
    #[error("Bad vaild until")]
    BadInReleaseVaildUntil,
    #[error("Earlier signature")]
    EarlierSignature,
    #[error("Expired signature")]
    ExpiredSignature,
    #[error("Bad SHA256 value")]
    BadSha256Value,
    #[error("Bad checksum entry: {0}")]
    BadChecksumEntry(String),
    #[error("Bad InRelease {0}: {1}")]
    InReleaseSyntaxError(String, String),
    #[error("Unsupport file type")]
    UnsupportFileType,
    #[error("Size should is number: {0}")]
    SizeShouldIsNumber(String),
}

pub type InReleaseParserResult<T> = Result<T, InReleaseParserError>;

impl InReleaseParser {
    pub fn new(
        p: &Path,
        trust_files: Option<&str>,
        mirror: &str,
        arch: &str,
    ) -> InReleaseParserResult<Self> {
        let s = std::fs::read_to_string(p).map_err(|e| {
            InReleaseParserError::FailedToOpenInRelease(p.display().to_string(), e.to_string())
        })?;

        let s = if s.starts_with("-----BEGIN PGP SIGNED MESSAGE-----") {
            verify::verify(&s, trust_files, mirror)?
        } else {
            s
        };

        let source = debcontrol_from_str(&s)?;

        let source_first = source.first();

        let date = source_first
            .and_then(|x| x.get("Date"))
            .take()
            .ok_or_else(|| InReleaseParserError::BadInReleaseData)?;

        let valid_until = source_first
            .and_then(|x| x.get("Valid-Until"))
            .take()
            .ok_or_else(|| InReleaseParserError::BadInReleaseVaildUntil)?;

        let date = OffsetDateTime::parse(date, &Rfc2822)
            .map_err(|_| InReleaseParserError::BadInReleaseData)?;

        let valid_until = OffsetDateTime::parse(valid_until, &Rfc2822)
            .map_err(|_| InReleaseParserError::BadInReleaseVaildUntil)?;

        let now = OffsetDateTime::now_utc();

        if now < date {
            return Err(InReleaseParserError::EarlierSignature);
        }

        if now > valid_until {
            return Err(InReleaseParserError::ExpiredSignature);
        }

        let sha256 = source_first
            .and_then(|x| x.get("SHA256"))
            .take()
            .ok_or_else(|| InReleaseParserError::BadSha256Value)?;

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

        let mut res = vec![];

        let c_res_clone = checksums_res.clone();

        let c = checksums_res
            .into_iter()
            .filter(|(name, _, _)| name.contains("all") || name.contains(arch))
            .collect::<Vec<_>>();

        let c = if c.is_empty() { c_res_clone } else { c };

        for i in c {
            let t = if i.0.contains("BinContents") {
                DistFileType::BinaryContents
            } else if i.0.contains("/Contents-") && i.0.contains('.') {
                DistFileType::CompressContents
            } else if i.0.contains("/Contents-") && !i.0.contains('.') {
                DistFileType::Contents
            } else if i.0.contains("Packages") && !i.0.contains('.') {
                DistFileType::PackageList
            } else if i.0.contains("Packages") && i.0.contains('.') {
                DistFileType::CompressPackageList
            } else if i.0.contains("Release") {
                DistFileType::Release
            } else {
                return Err(InReleaseParserError::UnsupportFileType);
            };

            res.push(ChecksumItem {
                name: i.0.to_owned(),
                size: i
                    .1
                    .parse::<u64>()
                    .map_err(|e| InReleaseParserError::SizeShouldIsNumber(i.1.to_string()))?,
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

fn debcontrol_from_str(s: &str) -> InReleaseParserResult<Vec<HashMap<String, String>>> {
    let mut res = vec![];

    let debcontrol = debcontrol::parse_str(s)
        .map_err(|e| InReleaseParserError::InReleaseSyntaxError(s.to_string(), e.to_string()))?;

    for i in debcontrol {
        let mut item = HashMap::new();
        let field = i.fields;

        for j in field {
            item.insert(j.name.to_string(), j.value.to_string());
        }

        res.push(item);
    }

    Ok(res)
}
