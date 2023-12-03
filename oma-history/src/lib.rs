use std::{fs, path::Path};

use oma_pm_operation_type::{InstallEntry, OmaOperation, RemoveEntry};
use rusqlite::{Connection, Result};
use serde::{Deserialize, Serialize};
use thiserror::Error;
use tracing::debug;

#[derive(Debug, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
pub enum SummaryType {
    Install(Vec<String>),
    Upgrade(Vec<String>),
    Remove(Vec<String>),
    FixBroken,
    TopicsChanged {
        add: Vec<String>,
        remove: Vec<String>,
    },
    Undo,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SummaryLog {
    pub typ: SummaryType,
    pub op: OmaOperation,
    pub is_success: bool,
}

type HistoryResult<T> = Result<T, HistoryError>;

#[derive(Debug, Error)]
pub enum HistoryError {
    #[error("Failed to create dir or file: {0}: {1}")]
    FailedOperateDirOrFile(String, std::io::Error),
    #[error("Failed to connect database: {0}")]
    ConnectError(rusqlite::Error),
    #[error("Failed to execute sqlte stmt, kind: {0}")]
    ExecuteError(rusqlite::Error),
    #[error("Failed to parser object: {0}")]
    ParseError(serde_json::Error),
    #[error("Failed to parser object: {0}")]
    ParseDbError(rusqlite::Error),
}

pub fn connect_or_create_db(write: bool, sysroot: String) -> HistoryResult<Connection> {
    let dir = Path::new(&sysroot).join("var/log/oma");
    let db_path = dir.join("history.db");
    if !dir.exists() {
        fs::create_dir_all(&dir)
            .map_err(|e| HistoryError::FailedOperateDirOrFile(dir.display().to_string(), e))?;
    }

    if !db_path.exists() {
        fs::File::create(&db_path)
            .map_err(|e| HistoryError::FailedOperateDirOrFile(db_path.display().to_string(), e))?;
    }

    let conn = Connection::open(db_path).map_err(HistoryError::ConnectError)?;

    if write {
        conn.execute(
            "CREATE TABLE IF NOT EXISTS \"history_oma_1.2\" (
                id INTEGER PRIMARY KEY,
                typ BLOB NOT NULL,
                time INTEGER NOT NULL,
                is_success INTEGER NOT NULL,
                install_packages BLOB,
                remove_packages BLOB,
                disk_size INTERGER NOT NULL,
                total_download_size INTEGER
            )",
            (), // empty list of parameters.
        )
        .map_err(HistoryError::ExecuteError)?;
    }

    Ok(conn)
}

pub fn write_history_entry(
    summary: OmaOperation,
    typ: SummaryType,
    conn: Connection,
    dry_run: bool,
    start_time: i64,
    success: bool,
) -> HistoryResult<()> {
    if dry_run {
        debug!("In dry-run mode, oma will not write history entries");
        return Ok(());
    }

    conn.execute(
        "INSERT INTO \"history_oma_1.2\" (typ, time, is_success, install_packages, remove_packages, disk_size, total_download_size) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
        (serde_json::to_string(&typ).map_err(HistoryError::ParseError)?,
        start_time,
        if success { 1 } else { 0 },
        serde_json::to_string(&summary.install).map_err(HistoryError::ParseError)?,
        serde_json::to_string(&summary.remove).map_err(HistoryError::ParseError)?,
        match (summary.disk_size.0.as_str(), summary.disk_size.1) {
            ("+", x) => x as i64,
            ("-", x) => 0 - x as i64,
            _ => unreachable!()
        },
        summary.total_download_size),
    )
    .map_err(HistoryError::ExecuteError)?;

    Ok(())
}

struct UnparseDbEntry {
    t: String,
    time: i64,
    is_success: bool,
    install_packages: String,
    remove_packages: String,
    disk_size: i64,
    total_download_size: u64,
}

impl TryFrom<&UnparseDbEntry> for SummaryLog {
    type Error = HistoryError;

    fn try_from(value: &UnparseDbEntry) -> std::prelude::v1::Result<Self, Self::Error> {
        let install_package: Vec<InstallEntry> =
            serde_json::from_str(&value.install_packages).map_err(HistoryError::ParseError)?;
        let remove_package: Vec<RemoveEntry> =
            serde_json::from_str(&value.remove_packages).map_err(HistoryError::ParseError)?;
        let disk_size = if value.disk_size >= 0 {
            ("+".to_string(), value.disk_size as u64)
        } else {
            ("-".to_string(), 0 - value.disk_size as u64)
        };
        let typ = serde_json::from_str(&value.t).map_err(HistoryError::ParseError)?;

        Ok(SummaryLog {
            typ,
            op: OmaOperation {
                install: install_package,
                remove: remove_package,
                disk_size,
                total_download_size: value.total_download_size,
            },
            is_success: value.is_success,
        })
    }
}

pub fn list_history(conn: Connection) -> HistoryResult<Vec<(SummaryLog, i64)>> {
    let mut res = vec![];
    let mut stmt = conn
        .prepare("SELECT typ, time, is_success, install_packages, remove_packages, disk_size, total_download_size FROM \"history_oma_1.2\" ORDER BY id DESC")
        .map_err(HistoryError::ExecuteError)?;

    let res_iter = stmt
        .query_map([], |row| {
            let t: String = row.get(0).unwrap();
            let time: i64 = row.get(1)?;
            let is_success: i64 = row.get(2)?;
            let install_packages: String = row.get(3)?;
            let remove_packages: String = row.get(4)?;
            let disk_size: i64 = row.get(5)?;
            let total_download_size: u64 = row.get(6)?;

            Ok(UnparseDbEntry {
                t,
                time,
                is_success: if is_success == 0 { false } else { true },
                install_packages,
                remove_packages,
                disk_size,
                total_download_size,
            })
        })
        .map_err(HistoryError::ExecuteError)?;

    for i in res_iter {
        let i = i.map_err(HistoryError::ParseDbError)?;
        res.push((SummaryLog::try_from(&i)?, i.time));
    }

    Ok(res)
}
