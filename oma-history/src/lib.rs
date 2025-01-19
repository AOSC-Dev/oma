use std::{
    fs,
    path::{Path, PathBuf},
};

use oma_pm_operation_type::{InstallEntry, OmaOperation, RemoveEntry};
use rusqlite::{Connection, Error, Result};
use serde::{Deserialize, Serialize};
use thiserror::Error;
use tracing::debug;

#[derive(Debug, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
pub enum SummaryType {
    Install(Vec<String>),
    Upgrade(Vec<String>),
    Remove(Vec<String>),
    Changes,
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
    ConnectError(Error),
    #[error("Failed to execute sqlte stmt, kind: {0}")]
    ExecuteError(Error),
    #[error("Failed to parser object: {0}")]
    ParseError(serde_json::Error),
    #[error("Failed to parser object: {0}")]
    ParseDbError(Error),
    #[error("History database is empty")]
    HistoryEmpty,
    #[error("Database no result by id: {0}")]
    NoResult(i64),
    #[error("Failed to get parent path: {0}")]
    FailedParentPath(String),
}

pub const DATABASE_PATH: &str = "var/lib/oma/history.db";

pub fn connect_db<P: AsRef<Path>>(db_path: P, write: bool) -> HistoryResult<Connection> {
    let conn = Connection::open(db_path);

    let conn = match conn {
        Ok(conn) => conn,
        Err(e) => match e {
            Error::SqliteFailure(err, _) if [1, 14].contains(&err.extended_code) => {
                return Err(HistoryError::HistoryEmpty)
            }
            e => return Err(HistoryError::ConnectError(e)),
        },
    };

    if write {
        conn.execute(
            "CREATE TABLE IF NOT EXISTS \"history_oma_1.2\" (
                id INTEGER PRIMARY KEY,
                typ BLOB NOT NULL,
                time INTEGER NOT NULL,
                is_success INTEGER NOT NULL,
                install_packages BLOB,
                remove_packages BLOB,
                disk_size INTEGER NOT NULL,
                total_download_size INTEGER
            )",
            (), // empty list of parameters.
        )
        .map_err(HistoryError::ExecuteError)?;
    }

    Ok(conn)
}

pub fn create_db_file<P: AsRef<Path>>(sysroot: P) -> HistoryResult<PathBuf> {
    let db_path = sysroot.as_ref().join(DATABASE_PATH);
    let dir = db_path
        .parent()
        .ok_or_else(|| HistoryError::FailedParentPath(db_path.display().to_string()))?;

    if !dir.exists() {
        fs::create_dir_all(dir)
            .map_err(|e| HistoryError::FailedOperateDirOrFile(dir.display().to_string(), e))?;
    }

    if !db_path.exists() {
        fs::File::create(&db_path)
            .map_err(|e| HistoryError::FailedOperateDirOrFile(db_path.display().to_string(), e))?;
    }

    Ok(db_path)
}

pub fn write_history_entry(
    summary: &OmaOperation,
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
        match (summary.disk_size.0.as_ref(), summary.disk_size.1) {
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
    let stmt =
        conn.prepare("SELECT id, typ, time, is_success FROM \"history_oma_1.2\" ORDER BY id DESC");

    let mut stmt = match stmt {
        Ok(stmt) => stmt,
        Err(e) => match e {
            Error::SqliteFailure(err, _) if [1, 14].contains(&err.extended_code) => {
                return Err(HistoryError::HistoryEmpty)
            }
            e => return Err(HistoryError::ConnectError(e)),
        },
    };

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
            is_success: is_success == 1,
        });
    }

    if res.is_empty() {
        return Err(HistoryError::HistoryEmpty);
    }

    Ok(res)
}

pub fn find_history_by_id(conn: &Connection, id: i64) -> HistoryResult<OmaOperation> {
    let mut stmt = conn
        .prepare("SELECT install_packages, remove_packages, disk_size, total_download_size FROM \"history_oma_1.2\" WHERE id = (?1)")
        .map_err(HistoryError::ExecuteError)?;

    let mut res_iter = stmt
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

    if let Some(i) = res_iter.next() {
        let (install_packages, remove_packages, disk_size, total_download_size) =
            i.map_err(HistoryError::ParseDbError)?;

        let install_package: Vec<InstallEntry> =
            serde_json::from_str(&install_packages).map_err(HistoryError::ParseError)?;
        let remove_package: Vec<RemoveEntry> =
            serde_json::from_str(&remove_packages).map_err(HistoryError::ParseError)?;
        let disk_size = if disk_size >= 0 {
            ("+".into(), disk_size as u64)
        } else {
            ("-".into(), (0 - disk_size) as u64)
        };

        res = Some(OmaOperation {
            install: install_package,
            remove: remove_package,
            disk_size,
            total_download_size,
            // 不记录 autoremovable
            autoremovable: (0, 0),
            suggest: vec![],
            recommend: vec![],
        });
    }

    res.ok_or_else(|| HistoryError::NoResult(id))
}
