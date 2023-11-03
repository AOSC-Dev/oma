use std::{fs, path::Path};

use oma_pm::apt::OmaOperation;
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

    let conn = Connection::open(db_path).map_err(|e| HistoryError::ConnectError(e))?;

    if write {
        conn.execute(
            "CREATE TABLE IF NOT EXISTS history (
                id    INTEGER PRIMARY KEY,
                data  BLOB,
                time  INTEGER
            )",
            (), // empty list of parameters.
        )
        .map_err(|e| HistoryError::ExecuteError(e))?;
    }

    Ok(conn)
}

pub fn write_history_entry(
    summary: OmaOperation,
    typ: SummaryType,
    conn: Connection,
    dry_run: bool,
    start_time: i64,
) -> HistoryResult<()> {
    if dry_run {
        debug!("In dry-run mode, oma will not write history entries");
        return Ok(());
    }

    let entry = SummaryLog { op: summary, typ };
    let buf = serde_json::to_vec(&entry).map_err(|e| HistoryError::ParseError(e))?;

    conn.execute(
        "INSERT INTO history (data, time) VALUES (?1, ?2)",
        (buf, start_time),
    )
    .map_err(|e| HistoryError::ExecuteError(e))?;

    // success!("{}", fl!("history-tips-1"));
    // info!("{}", fl!("history-tips-2"));

    Ok(())
}

pub fn list_history(conn: Connection) -> HistoryResult<Vec<(SummaryLog, i64)>> {
    let mut res = vec![];
    let mut stmt = conn
        .prepare("SELECT data, time FROM history ORDER BY id DESC")
        .map_err(|e| HistoryError::ExecuteError(e))?;
    let res_iter = stmt
        .query_map([], |row| {
            let data: Vec<u8> = row.get(0)?;
            let time: i64 = row.get(1)?;
            Ok((data, time))
        })
        .map_err(|e| HistoryError::ExecuteError(e))?;

    for i in res_iter {
        let (data, time) = i.map_err(|e| HistoryError::ExecuteError(e))?;
        res.push((
            serde_json::from_slice(&data).map_err(|e| HistoryError::ParseError(e))?,
            time,
        ));
    }

    Ok(res)
}
