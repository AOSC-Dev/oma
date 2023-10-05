use std::{fs::create_dir_all, path::Path};

use crate::fl;
use anyhow::Result;
use oma_console::{debug, info, success};
use oma_pm::apt::OmaOperation;
use rusqlite::Connection;
use serde::{Deserialize, Serialize};

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

pub fn connect_or_create_db(write: bool) -> Result<Connection> {
    let dir = Path::new("/var/log/oma");
    let db_path = dir.join("history.db");
    if !dir.exists() {
        create_dir_all(dir)?;
    }

    if !db_path.exists() {
        std::fs::File::create(&db_path)?;
    }

    let conn = Connection::open(db_path)?;

    if write {
        conn.execute(
            "CREATE TABLE IF NOT EXISTS history (
                id    INTEGER PRIMARY KEY,
                data  BLOB,
                time  INTEGER
            )",
            (), // empty list of parameters.
        )?;
    }

    Ok(conn)
}

pub fn write_history_entry(
    summary: OmaOperation,
    typ: SummaryType,
    conn: Connection,
    dry_run: bool,
    start_time: i64,
) -> Result<()> {
    if dry_run {
        debug!("In dry-run mode, oma will not write history entries");
        return Ok(());
    }

    let entry = SummaryLog { op: summary, typ };
    let buf = serde_json::to_vec(&entry)?;

    conn.execute(
        "INSERT INTO history (data, time) VALUES (?1, ?2)",
        (buf, start_time),
    )?;

    success!("{}", fl!("history-tips-1"));
    info!("{}", fl!("history-tips-2"));

    Ok(())
}

pub fn list_history(conn: Connection) -> Result<Vec<(SummaryLog, i64)>> {
    let mut res = vec![];
    let mut stmt = conn.prepare("SELECT data, time FROM history ORDER BY id DESC")?;
    let res_iter = stmt.query_map([], |row| {
        let data: Vec<u8> = row.get(0)?;
        let time: i64 = row.get(1)?;
        Ok((data, time))
    })?;

    for i in res_iter {
        let (data, time) = i?;
        res.push((serde_json::from_slice(&data)?, time));
    }

    Ok(res)
}
