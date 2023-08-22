use std::{
    fs::create_dir_all,
    io::{BufReader, Read, Write},
    path::Path,
};

use crate::fl;
use anyhow::Result;
use oma_console::{info, success};
use oma_pm::apt::OmaOperation;
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
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SummaryLog {
    pub typ: SummaryType,
    pub op: OmaOperation,
}

pub fn write_history_entry(summary: OmaOperation, typ: SummaryType) -> Result<()> {
    let dir = Path::new("/var/log/oma");
    let db_path = dir.join("history.json");
    if !dir.exists() {
        create_dir_all(dir)?;
    }

    let mut f = std::fs::OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .open(db_path)?;

    let mut buf = vec![];
    f.read_to_end(&mut buf)?;

    let mut db: Vec<SummaryLog> = if buf.is_empty() {
        vec![]
    } else {
        serde_json::from_slice(&buf)?
    };

    let entry = SummaryLog { op: summary, typ };
    db.insert(0, entry);

    let buf = serde_json::to_vec(&db)?;
    f.write_all(&buf)?;

    success!("{}", fl!("history-tips-1"));
    info!("{}", fl!("history-tips-2"));

    Ok(())
}

pub fn list_history() -> Result<Vec<SummaryLog>> {
    let f = std::fs::File::open("/var/log/oma/history.json")?;
    let reader = BufReader::new(f);
    let db: Vec<SummaryLog> = serde_json::from_reader(reader)?;

    Ok(db)
}
