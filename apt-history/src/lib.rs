use std::path::Path;

use chrono::NaiveDateTime;
use deb822_fast::{FromDeb822, FromDeb822Paragraph, ToDeb822};

#[derive(Debug, thiserror::Error)]
pub enum AptHistoryError {
    #[error("Failed to parse date")]
    DateParseError(#[from] chrono::ParseError),
    #[error(transparent)]
    ParseFromString(#[from] deb822_fast::Error),
    #[error("Failed to parse AptHistory from deb822: {0}")]
    ParseFromParagraph(String),
    #[error("Failed to read file: {0}")]
    FileReadError(#[from] std::io::Error),
    #[error("Failed to parse Install entry: {0}")]
    ParseOperationLine(String),
}

const DATE_FORMAT: &str = "%Y-%m-%d  %H:%M:%S";

#[derive(FromDeb822, ToDeb822, Clone, PartialEq, Debug, Default)]
pub struct AptHistory {
    #[deb822(
        field = "Start-Date",
        deserialize_with = deserialize_date,
        serialize_with = serialize_date
    )]
    pub start_date: NaiveDateTime,
    #[deb822(
        field = "End-Date",
        deserialize_with = deserialize_date,
        serialize_with = serialize_date
    )]
    pub end_date: Option<NaiveDateTime>,
    #[deb822(field = "Commandline")]
    pub command_line: Option<String>,
    #[deb822(field = "Requested-By")]
    pub requested_by: Option<String>,
    #[deb822(field = "Install", deserialize_with = deserialize_install_and_reinstall, serialize_with = serialize_install_and_reinstall)]
    pub install: Option<Vec<Install>>,
    #[deb822(field = "Upgrade", deserialize_with = deserialize_upgrade_and_downgrade, serialize_with = serialize_upgrade_and_downgrade)]
    pub upgrade: Option<Vec<Upgrade>>,
    #[deb822(field = "Remove", deserialize_with = deserialize_remove_and_purge, serialize_with = serialize_remove_and_purge)]
    pub remove: Option<Vec<Remove>>,
    #[deb822(field = "Reinstall", deserialize_with = deserialize_install_and_reinstall, serialize_with = serialize_install_and_reinstall)]
    pub reinstall: Option<Vec<Install>>,
    #[deb822(field = "Downgrade", deserialize_with = deserialize_upgrade_and_downgrade, serialize_with = serialize_upgrade_and_downgrade)]
    pub downgrade: Option<Vec<Upgrade>>,
    #[deb822(field = "Purge", deserialize_with = deserialize_remove_and_purge, serialize_with = serialize_remove_and_purge)]
    pub purge: Option<Vec<Remove>>,
    #[deb822(field = "Error")]
    pub error: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Operation {
    Install,
    Upgrade,
    Remove,
    Reinstall,
    Downgrade,
    Purge,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Install {
    package: String,
    version: String,
    auto: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Upgrade {
    package: String,
    old_version: String,
    new_version: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Remove {
    package: String,
    version: String,
}

impl AptHistory {
    pub fn action(&self) -> Vec<Operation> {
        let mut actions = vec![];

        if self.install.as_ref().is_some_and(|i| !i.is_empty()) {
            actions.push(Operation::Install);
        }

        if self.upgrade.as_ref().is_some_and(|u| !u.is_empty()) {
            actions.push(Operation::Upgrade);
        }

        if self.remove.as_ref().is_some_and(|r| !r.is_empty()) {
            actions.push(Operation::Remove);
        }

        if self.reinstall.as_ref().is_some_and(|r| !r.is_empty()) {
            actions.push(Operation::Reinstall);
        }

        if self.downgrade.as_ref().is_some_and(|d| !d.is_empty()) {
            actions.push(Operation::Downgrade);
        }

        if self.purge.as_ref().is_some_and(|p| !p.is_empty()) {
            actions.push(Operation::Purge);
        }

        actions
    }

    pub fn changes(&self) -> usize {
        self.install.as_ref().map_or(0, |v| v.len())
            + self.downgrade.as_ref().map_or(0, |v| v.len())
            + self.upgrade.as_ref().map_or(0, |v| v.len())
            + self.remove.as_ref().map_or(0, |v| v.len())
            + self.reinstall.as_ref().map_or(0, |v| v.len())
            + self.purge.as_ref().map_or(0, |v| v.len())
    }
}

pub fn parse_from_file(path: impl AsRef<Path>) -> Result<Vec<AptHistory>, AptHistoryError> {
    let content = std::fs::read_to_string(path).map_err(AptHistoryError::FileReadError)?;

    parse_from_str(&content)
}

pub fn parse_from_str(s: &str) -> Result<Vec<AptHistory>, AptHistoryError> {
    let source: deb822_fast::Deb822 = s.parse().map_err(|e| AptHistoryError::ParseFromString(e))?;

    let mut result = source
        .into_iter()
        .map(|p| {
            AptHistory::from_paragraph(&p)
                .map_err(|e| AptHistoryError::ParseFromParagraph(e.to_string()))
        })
        .collect::<Result<Vec<_>, _>>()?;

    result.sort_unstable_by_key(|h| h.start_date);

    Ok(result)
}

fn deserialize_date(date: &str) -> Result<chrono::NaiveDateTime, AptHistoryError> {
    chrono::NaiveDateTime::parse_from_str(date, DATE_FORMAT)
        .map_err(AptHistoryError::DateParseError)
}

fn serialize_date(date: &NaiveDateTime) -> String {
    date.format(DATE_FORMAT).to_string()
}

fn deserialize_install_and_reinstall(s: &str) -> Result<Vec<Install>, AptHistoryError> {
    let entries = s.split("), ");

    let mut result = Vec::new();
    for entry in entries {
        let entry = entry.trim();
        let entry = entry.strip_suffix(')').unwrap_or(entry);

        let (pkg, rest) = entry
            .split_once('(')
            .ok_or_else(|| AptHistoryError::ParseOperationLine(entry.to_string()))?;

        let (version, auto_flag) = match rest.split_once(", ") {
            Some((v, a)) => (v, Some(a)),
            None => (rest, None),
        };

        result.push(Install {
            package: pkg.trim().to_string(),
            version: version.trim().to_string(),
            auto: auto_flag.map(|a| a.trim() == "automatic").unwrap_or(false),
        });
    }
    Ok(result)
}

fn serialize_install_and_reinstall(vec: &[Install]) -> String {
    vec.iter()
        .map(|i| {
            if i.auto {
                format!("{} ({}, automatic)", i.package, i.version)
            } else {
                format!("{} ({})", i.package, i.version)
            }
        })
        .collect::<Vec<_>>()
        .join(", ")
}

fn deserialize_upgrade_and_downgrade(vec: &str) -> Result<Vec<Upgrade>, AptHistoryError> {
    let entries = vec.split("), ");
    let mut result = vec![];

    for entry in entries {
        let entry = entry.trim();
        let entry = entry.strip_suffix(')').unwrap_or(entry);

        let (pkg, rest) = entry
            .split_once('(')
            .ok_or_else(|| AptHistoryError::ParseOperationLine(entry.to_string()))?;

        let (old_version, new_version) = rest
            .split_once(", ")
            .ok_or_else(|| AptHistoryError::ParseOperationLine(entry.to_string()))?;

        result.push(Upgrade {
            package: pkg.trim().to_string(),
            old_version: old_version.trim().to_string(),
            new_version: new_version.trim().to_string(),
        });
    }
    Ok(result)
}

fn serialize_upgrade_and_downgrade(vec: &[Upgrade]) -> String {
    vec.iter()
        .map(|u| format!("{} ({}, {})", u.package, u.old_version, u.new_version))
        .collect::<Vec<_>>()
        .join(", ")
}

fn deserialize_remove_and_purge(vec: &str) -> Result<Vec<Remove>, AptHistoryError> {
    let entries = vec.split("), ");
    let mut result = vec![];

    for entry in entries {
        let entry = entry.trim();
        let entry = entry.strip_suffix(')').unwrap_or(entry);

        let (pkg, version) = entry
            .split_once('(')
            .ok_or_else(|| AptHistoryError::ParseOperationLine(entry.to_string()))?;

        result.push(Remove {
            package: pkg.trim().to_string(),
            version: version.trim().to_string(),
        });
    }
    Ok(result)
}

fn serialize_remove_and_purge(vec: &[Remove]) -> String {
    vec.iter()
        .map(|r| format!("{} ({})", r.package, r.version))
        .collect::<Vec<_>>()
        .join(", ")
}
