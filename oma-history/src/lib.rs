use std::{
    collections::HashMap,
    fs,
    path::{Path, PathBuf},
};

use num_enum::{IntoPrimitive, TryFromPrimitive};
use oma_pm_operation_type::{InstallOperation, OmaOperation, RemoveTag};
use rusqlite::{Connection, Error, Result};
use thiserror::Error;
use tracing::debug;

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, IntoPrimitive, TryFromPrimitive)]
#[repr(i64)]
pub enum SummaryType {
    Install = 1,
    Upgrade = 2,
    Remove = 3,
    FixBroken = 4,
    Undo = 5,
}

pub struct HistoryEntry {
    pub install: Vec<InstallHistoryEntry>,
    pub remove: Vec<RemoveHistoryEntry>,
    pub disk_size: (Box<str>, u64),
    pub total_download_size: u64,
    pub is_success: bool,
}

#[derive(Debug)]
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
            "CREATE TABLE IF NOT EXISTS \"history_oma_1.14\" (
                id INTEGER PRIMARY KEY,
                install_type INTEGER NOT NULL,
                time INTEGER NOT NULL,
                is_success INTEGER NOT NULL,
                disk_size INTEGER NOT NULL,
                total_download_size INTEGER
            )",
            (), // empty list of parameters.
        )
        .map_err(HistoryError::ExecuteError)?;

        conn.execute(
            "CREATE TABLE IF NOT EXISTS \"history_install_package_oma_1.14\" (
            history_id INTEGER NOT NULL,
            package_name TEXT NOT NULL,
            old_version TEXT,
            new_version TEXT NOT NULL,
            old_size INTEGER,
            new_size INTEGER NOT NULL,
            download_size INTEGER NOT NULL,
            arch TEXT NOT NULL,
            operation INTEGER NOT NULL
        )",
            (),
        )
        .map_err(HistoryError::ExecuteError)?;

        conn.execute(
            "CREATE TABLE IF NOT EXISTS \"history_remove_package_oma_1.14\" (
            history_id INTEGER NOT NULL,
            package_name TEXT NOT NULL,
            version TEXT NOT NULL,
            size INTEGER NOT NULL,
            arch TEXT NOT NULL
        )",
            (),
        )
        .map_err(HistoryError::ExecuteError)?;

        conn.execute(
            "CREATE TABLE IF NOT EXISTS \"history_remove_package_detail_oma_1.14\" (
            history_id INTEGER NOT NULL,
            package_name TEXT NOT NULL,
            autoremove INTEGER NOT NULL,
            purge INTEGER NOT NULL,
            resolver INTEGER NOT NULL
        )",
            (),
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
    install_type: SummaryType,
    conn: Connection,
    dry_run: bool,
    start_time: i64,
    success: bool,
) -> HistoryResult<()> {
    if dry_run {
        debug!("In dry-run mode, oma will not write history entries");
        return Ok(());
    }

    let install_type: i64 = install_type.into();

    let id: i64 = conn.query_row(
        r#"INSERT INTO "history_oma_1.14" (install_type, time, is_success, disk_size, total_download_size)
                VALUES (?1, ?2, ?3, ?4, ?5)
                RETURNING id;"#,
        (install_type,
        start_time,
        if success { 1 } else { 0 },
        match (summary.disk_size.0.as_ref(), summary.disk_size.1) {
            ("+", x) => x as i64,
            ("-", x) => 0 - x as i64,
            _ => unreachable!()
        },
        summary.total_download_size),
        |row| row.get(0)
    )
    .map_err(HistoryError::ExecuteError)?;

    for i in &summary.install {
        let op: u8 = (*i.op()).into();
        let op = op as i64;
        conn.execute(
            r#"INSERT INTO "history_install_package_oma_1.14" (history_id, package_name, old_version, new_version, old_size, new_size, download_size, arch, operation)
                    VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)"#,
            (id, i.name(), i.old_version(), i.new_version(), i.old_size(), i.new_size(), i.download_size(), i.arch(), op),
        ).map_err(HistoryError::ExecuteError)?;
    }

    for i in &summary.remove {
        let Some(version) = i.version() else {
            // 仅为删除配置文件时，版本为空，因此不记录
            continue;
        };

        conn.execute(
            r#"INSERT INTO "history_remove_package_oma_1.14" (history_id, package_name, version, size, arch)
                    VALUES (?1, ?2, ?3, ?4, ?5)"#,
            (id, i.name(), version, i.size(), i.arch()),
        ).map_err(HistoryError::ExecuteError)?;

        conn.execute(
            r#"INSERT INTO "history_remove_package_detail_oma_1.14" (history_id, package_name, autoremove, purge, resolver)
                    VALUES (?1, ?2, ?3, ?4, ?5)"#,
            (id,
                i.name(),
                if i.details().contains(&RemoveTag::AutoRemove) { 1 } else { 0 },
                if i.details().contains(&RemoveTag::Purge) { 1 } else { 0 },
                if i.details().contains(&RemoveTag::Resolver) { 1 } else { 0 }
            ),
        ).map_err(HistoryError::ExecuteError)?;
    }

    Ok(())
}

pub struct HistoryListEntry {
    pub id: i64,
    pub summary_type: SummaryType,
    pub time: i64,
    pub is_success: bool,
}

pub struct InstallHistoryEntry {
    pub pkg_name: String,
    pub old_version: Option<String>,
    pub new_version: String,
    pub old_size: Option<i64>,
    pub new_size: i64,
    pub download_size: i64,
    pub arch: String,
    pub operation: InstallOperation,
}

pub struct RemoveHistoryEntryTmp {
    pub pkg_name: String,
    pub version: String,
    pub size: i64,
    pub arch: String,
}

pub struct RemoveHistoryEntry {
    pub pkg_name: String,
    pub version: String,
    pub size: i64,
    pub arch: String,
    pub tags: Vec<RemoveTag>,
}

pub fn list_history(conn: &Connection) -> HistoryResult<Vec<HistoryListEntry>> {
    let mut res = vec![];
    let stmt = conn.prepare(
        "SELECT id, install_type, time, is_success FROM \"history_oma_1.14\" ORDER BY id DESC",
    );

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
            let t: i64 = row.get(1)?;
            let time: i64 = row.get(2)?;
            let is_success: i64 = row.get(3)?;

            Ok((id, t, time, is_success))
        })
        .map_err(HistoryError::ExecuteError)?;

    for i in res_iter {
        let (id, t, time, is_success) = i.map_err(HistoryError::ParseDbError)?;

        let install_type: SummaryType = t.try_into().unwrap();

        res.push(HistoryListEntry {
            id,
            summary_type: install_type,
            time,
            is_success: is_success == 1,
        });
    }

    if res.is_empty() {
        return Err(HistoryError::HistoryEmpty);
    }

    Ok(res)
}

pub fn find_history_by_id(conn: &Connection, id: i64) -> HistoryResult<HistoryEntry> {
    let mut query_history_table = conn
        .prepare("SELECT is_success, disk_size, total_download_size FROM \"history_oma_1.14\" WHERE id = (?1)")
        .map_err(HistoryError::ExecuteError)?;

    let mut res_iter = query_history_table
        .query_map([id], |row| {
            let is_success: i64 = row.get(0)?;
            let disk_size: i64 = row.get(1)?;
            let total_download_size: u64 = row.get(2)?;

            Ok((is_success, disk_size, total_download_size))
        })
        .map_err(HistoryError::ExecuteError)?;

    let mut query_install_table = conn.prepare(r#"SELECT package_name, old_version, new_version, old_size, new_size, download_size, arch, operation FROM "history_install_package_oma_1.14"
    WHERE history_id = (?1)"#)
    .map_err(HistoryError::ExecuteError)?;

    let res_install_iter = query_install_table
        .query_map([id], |row| {
            let pkg_name: String = row.get(0)?;
            let old_version: Option<String> = row.get(1)?;
            let new_version: String = row.get(2)?;
            let old_size: Option<i64> = row.get(3)?;
            let new_size: i64 = row.get(4)?;
            let download_size: i64 = row.get(5)?;
            let arch: String = row.get(6)?;
            let operation: i64 = row.get(7)?;
            let operation = InstallOperation::from(operation as u8);

            Ok(InstallHistoryEntry {
                pkg_name,
                old_version,
                new_version,
                old_size,
                new_size,
                download_size,
                arch,
                operation,
            })
        })
        .map_err(HistoryError::ExecuteError)?;

    let mut query_remove_table = conn
        .prepare(
            r#"SELECT package_name, version, size, arch FROM "history_remove_package_oma_1.14"
    WHERE history_id = (?1)"#,
        )
        .map_err(HistoryError::ExecuteError)?;

    let res_remove_iter = query_remove_table
        .query_map([id], |row| {
            let pkg_name: String = row.get(0)?;
            let version: String = row.get(1)?;
            let size: i64 = row.get(2)?;
            let arch: String = row.get(3)?;

            Ok(RemoveHistoryEntryTmp {
                pkg_name,
                version,
                size,
                arch,
            })
        })
        .map_err(HistoryError::ExecuteError)?;

    let mut query_remove_details_table = conn
        .prepare(
            r#"SELECT package_name, autoremove, purge, resolver FROM "history_remove_package_detail_oma_1.14"
    WHERE history_id = (?1)"#,
        )
        .map_err(HistoryError::ExecuteError)?;

    let res_remove_details = query_remove_details_table
        .query_map([id], |row| {
            let mut remove_tag = vec![];
            let package_name: String = row.get(0)?;
            let autoremove: i64 = row.get(1)?;
            let purge: i64 = row.get(2)?;
            let resolver: i64 = row.get(3)?;

            if autoremove == 1 {
                remove_tag.push(RemoveTag::AutoRemove);
            }

            if purge == 1 {
                remove_tag.push(RemoveTag::Purge);
            }

            if resolver == 1 {
                remove_tag.push(RemoveTag::Resolver);
            }

            Ok((package_name, remove_tag))
        })
        .map_err(HistoryError::ExecuteError)?
        .collect::<Result<HashMap<_, _>>>()
        .map_err(HistoryError::ParseDbError)?;

    let mut res = None;

    if let Some(i) = res_iter.next() {
        let (is_success, disk_size, total_download_size) = i.map_err(HistoryError::ParseDbError)?;

        let is_success = is_success == 1;

        let disk_size: (Box<str>, u64) = if disk_size >= 0 {
            ("+".into(), disk_size as u64)
        } else {
            ("-".into(), (0 - disk_size) as u64)
        };

        let mut install = vec![];
        let mut remove = vec![];

        for i in res_install_iter {
            let i = i.map_err(HistoryError::ParseDbError)?;

            install.push(i);
        }

        for i in res_remove_iter {
            let RemoveHistoryEntryTmp {
                pkg_name,
                version,
                size,
                arch,
            } = i.map_err(HistoryError::ParseDbError)?;

            let tags = res_remove_details.get(&pkg_name).unwrap().to_owned();

            remove.push(RemoveHistoryEntry {
                pkg_name,
                version,
                size,
                arch,
                tags,
            });
        }

        res = Some(HistoryEntry {
            install,
            remove,
            disk_size,
            total_download_size,
            is_success,
        })
    }

    res.ok_or_else(|| HistoryError::NoResult(id))
}
