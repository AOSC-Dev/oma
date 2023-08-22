use std::{
    fs::{create_dir_all, File},
    io::{BufRead, BufReader, Write},
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
    Undo,
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
        .append(true)
        .open(db_path)?;

    Ok(f)
}

pub fn write_history_entry(summary: OmaOperation, typ: SummaryType, mut f: File) -> Result<()> {
    let entry = SummaryLog { op: summary, typ };
    let mut buf = serde_json::to_vec(&entry)?;
    buf.push(b'\n');

    f.write_all(&buf)?;

    success!("{}", fl!("history-tips-1"));
    info!("{}", fl!("history-tips-2"));

    Ok(())
}

pub fn list_history(f: File) -> Result<Vec<SummaryLog>> {
    let reader = BufReader::new(f);
    let mut db = vec![];
    for line in reader.lines() {
        let summary_line: SummaryLog = serde_json::from_str(&line?)?;
        db.insert(0, summary_line);
    }

    Ok(db)
}
