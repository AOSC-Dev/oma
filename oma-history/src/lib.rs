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
    #[error("Database no result by id: {0}")]
    NoResult(i64),
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

pub struct HistoryListEntry {
    pub id: i64,
    pub t: SummaryType,
    pub time: i64,
    pub is_success: bool,
}

pub fn list_history(conn: &Connection) -> HistoryResult<Vec<HistoryListEntry>> {
    let mut res = vec![];
    let mut stmt = conn
        .prepare("SELECT id, typ, time, is_success FROM \"history_oma_1.2\" ORDER BY id DESC")
        .map_err(HistoryError::ExecuteError)?;

    let res_iter = stmt
        .query_map([], |row| {
            let id: i64 = row.get(0)?;
            let t: String = row.get(1)?;
            let time: i64 = row.get(2)?;
            let is_success: i64 = row.get(3)?;

            Ok((id, t, time, is_success))
        })
        .map_err(HistoryError::ExecuteError)?;

    for i in res_iter {
        let (id, t, time, is_success) = i.map_err(HistoryError::ParseDbError)?;
        res.push(HistoryListEntry {
            id,
            t: serde_json::from_str(&t).map_err(HistoryError::ParseError)?,
            time,
            is_success: if is_success == 1 { true } else { false },
        });
    }

    Ok(res)
}

pub fn find_history_by_id(conn: &Connection, id: i64) -> HistoryResult<OmaOperation> {
    let mut stmt = conn
        .prepare("SELECT install_packages, remove_packages, disk_size, total_download_size FROM \"history_oma_1.2\" WHERE id = (?1)")
        .map_err(HistoryError::ExecuteError)?;

    let res_iter = stmt
        .query_map([id], |row| {
            let install_packages: String = row.get(0)?;
            let remove_packages: String = row.get(1)?;
            let disk_size: i64 = row.get(2)?;
            let total_download_size: u64 = row.get(3)?;

            Ok((
                install_packages,
                remove_packages,
                disk_size,
                total_download_size,
            ))
        })
        .map_err(HistoryError::ExecuteError)?;

    let mut res = None;

    for i in res_iter {
        let (install_packages, remove_packages, disk_size, total_download_size) =
            i.map_err(HistoryError::ParseDbError)?;

        let install_package: Vec<InstallEntry> =
            serde_json::from_str(&install_packages).map_err(HistoryError::ParseError)?;
        let remove_package: Vec<RemoveEntry> =
            serde_json::from_str(&remove_packages).map_err(HistoryError::ParseError)?;
        let disk_size = if disk_size >= 0 {
            ("+".to_string(), disk_size as u64)
        } else {
            ("-".to_string(), 0 - disk_size as u64)
        };

        res = Some(OmaOperation {
            install: install_package,
            remove: remove_package,
            disk_size,
            total_download_size,
        });
        break;
    }

    res.ok_or_else(|| HistoryError::NoResult(id))
}
