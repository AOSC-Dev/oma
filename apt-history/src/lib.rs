use std::path::Path;

use chrono::NaiveDateTime;
use deb822_fast::{FromDeb822, FromDeb822Paragraph, ToDeb822};

#[derive(Debug, thiserror::Error)]
pub enum AptHistoryError {
    #[error("Failed to parse date")]
    DateParseError(#[from] chrono::ParseError),
    #[error(transparent)]
    Deb822Error(#[from] deb822_fast::Error),
    #[error("Failed to parse AptHistory from deb822: {0}")]
    Deb822ParseError(String),
    #[error("Failed to read file: {0}")]
    FileReadError(#[from] std::io::Error),
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
    #[deb822(field = "Install", deserialize_with = deserialize_vector, serialize_with = serialize_vector)]
    pub install: Option<Vec<String>>,
    #[deb822(field = "Upgrade", deserialize_with = deserialize_vector, serialize_with = serialize_vector)]
    pub upgrade: Option<Vec<String>>,
    #[deb822(field = "Remove", deserialize_with = deserialize_vector, serialize_with = serialize_vector)]
    pub remove: Option<Vec<String>>,
    #[deb822(field = "Reinstall", deserialize_with = deserialize_vector, serialize_with = serialize_vector)]
    pub reinstall: Option<Vec<String>>,
    #[deb822(field = "Downgrade", deserialize_with = deserialize_vector, serialize_with = serialize_vector)]
    pub downgrade: Option<Vec<String>>,
    #[deb822(field = "Purge", deserialize_with = deserialize_vector, serialize_with = serialize_vector)]
    pub purge: Option<Vec<String>>,
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
    let source: deb822_fast::Deb822 = s.parse().map_err(|e| AptHistoryError::Deb822Error(e))?;

    let mut result = source
        .into_iter()
        .map(|p| {
            AptHistory::from_paragraph(&p)
                .map_err(|e| AptHistoryError::Deb822ParseError(e.to_string()))
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

fn deserialize_vector(vec: &str) -> Result<Vec<String>, AptHistoryError> {
    Ok(vec.split("), ").map(|s| format!("{s})")).collect())
}

fn serialize_vector(vec: &[String]) -> String {
    vec.join(", ")
}
