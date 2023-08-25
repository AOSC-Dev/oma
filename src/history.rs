use std::{fs::create_dir_all, path::Path};

use crate::fl;
use anyhow::Result;
use oma_console::{info, success};
use oma_pm::apt::OmaOperation;
use rusqlite::Connection;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
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

pub fn connect_db(write: bool) -> Result<Connection> {
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
                data  BLOB
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
) -> Result<()> {
    let entry = SummaryLog { op: summary, typ };
    let buf = serde_json::to_vec(&entry)?;

    conn.execute("INSERT INTO history (data) VALUES (?1)", [buf])?;

    success!("{}", fl!("history-tips-1"));
    info!("{}", fl!("history-tips-2"));

    Ok(())
}

pub fn list_history(conn: Connection) -> Result<Vec<SummaryLog>> {
    let mut res = vec![];
    let mut stmt = conn.prepare("SELECT data FROM history ORDER BY id DESC")?;
    let res_iter = stmt.query_map([], |row| {
        let data: Vec<u8> = row.get(0)?;
        Ok(data)
    })?;

    for i in res_iter {
        res.push(serde_json::from_slice(&i?)?);
    }

    Ok(res)
}
