use std::{
    fs::{create_dir_all, File},
    io::{BufReader, Read, Write, Seek},
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
    Undo
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SummaryLog {
    pub typ: SummaryType,
    pub op: OmaOperation,
}

pub fn db_file() -> Result<File> {
    let dir = Path::new("/var/log/oma");
    let db_path = dir.join("history.json");
    if !dir.exists() {
        create_dir_all(dir)?;
    }

    let f = std::fs::OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .append(false)
        .open(db_path)?;

    Ok(f)
}

pub fn write_history_entry(summary: OmaOperation, typ: SummaryType, mut f: File) -> Result<()> {
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
    f.seek(std::io::SeekFrom::Start(0))?;
    f.write_all(&buf)?;

    success!("{}", fl!("history-tips-1"));
    info!("{}", fl!("history-tips-2"));

    Ok(())
}

pub fn list_history(f: File) -> Result<Vec<SummaryLog>> {
    let reader = BufReader::new(f);
    let db: Vec<SummaryLog> = serde_json::from_reader(reader)?;

    Ok(db)
}

