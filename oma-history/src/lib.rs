mod migrations;

use std::{
    collections::HashMap,
    env::args,
    fs,
    path::{Path, PathBuf},
};

use migrations::create_and_maybe_migration_from_oma_db_v2;
use oma_pm_operation_type::{InstallOperation, OmaOperation, RemoveTag};
use rusqlite::{Connection, Error, Result};
use serde::Deserialize;
use spdlog::debug;
use thiserror::Error;

pub struct HistoryEntryInner {
    pub install: Vec<InstallHistoryEntry>,
    pub remove: Vec<RemoveHistoryEntry>,
    pub disk_size: i64,
    pub total_download_size: i64,
    pub is_success: bool,
}

type HistoryResult<T> = Result<T, HistoryError>;

#[derive(Debug, Error)]
pub enum HistoryError {
    #[error("Failed to create dir or file: {0}")]
    FailedOperateDirOrFile(String, std::io::Error),
    #[error("Failed to connect database")]
    ConnectError(Error),
    #[error("Failed to create transaction")]
    CreateTransaction(Error),
    #[error("Failed to execute sqlte stmt")]
    ExecuteError(Error),
    #[error("Failed to parser object")]
    ParseDbError(Error),
    #[error("History database is empty")]
    HistoryEmpty,
    #[error("Database no result by id: {0}")]
    NoResult(i64),
    #[error("Failed to get parent path: {0}")]
    FailedParentPath(String),
    #[error("Has no upgrade system log in this machine")]
    NoUpgradeSystemLog,
}

pub const DATABASE_PATH: &str = "var/lib/oma/history.db";
pub(crate) const INSERT_NEW_MAIN_TABLE: &str = r#"INSERT INTO "history_oma_1.14" (command, time, is_success, disk_size, total_download_size, install_count, remove_count, upgrade_count, downgrade_count, reinstall_count, is_fixbroken, is_undo)
    VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12)
    RETURNING id;"#;
pub(crate) const INSERT_INSTALL_TABLE: &str = r#"INSERT INTO "history_install_package_oma_1.14" (history_id, package_name, old_version, new_version, old_size, new_size, download_size, arch, operation)
    VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)"#;
pub(crate) const INSERT_REMOVE_TABLE: &str = r#"INSERT INTO "history_remove_package_oma_1.14" (history_id, package_name, version, size, arch)
    VALUES (?1, ?2, ?3, ?4, ?5)"#;
pub(crate) const INSERT_REMOVE_DETAIL_TABLE: &str = r#"INSERT INTO "history_remove_package_detail_oma_1.14" (history_id, package_name, autoremove, purge, resolver)
    VALUES (?1, ?2, ?3, ?4, ?5)"#;

pub fn connect_db<P: AsRef<Path>>(db_path: P, write: bool) -> HistoryResult<Connection> {
    let conn = Connection::open(db_path);

    let mut conn = match conn {
        Ok(conn) => conn,
        Err(e) => match e {
            Error::SqliteFailure(err, _) if [1, 14].contains(&err.extended_code) => {
                return Err(HistoryError::HistoryEmpty);
            }
            e => return Err(HistoryError::ConnectError(e)),
        },
    };

    if write {
        create_and_maybe_migration_from_oma_db_v2(&mut conn)?;
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

pub struct HistoryInfo<'a> {
    pub summary: &'a OmaOperation,
    pub start_time: i64,
    pub success: bool,
    pub is_fix_broken: bool,
    pub is_undo: bool,
    pub topics_enabled: Vec<String>,
    pub topics_disabled: Vec<String>,
}

pub fn write_history_entry(
    mut conn: Connection,
    dry_run: bool,
    info: HistoryInfo<'_>,
) -> HistoryResult<()> {
    let HistoryInfo {
        summary,
        start_time,
        success,
        is_fix_broken,
        is_undo,
        topics_enabled,
        topics_disabled,
    } = info;

    if dry_run {
        debug!("In dry-run mode, oma will not write history entries");
        return Ok(());
    }

    let transaction = conn
        .transaction()
        .map_err(HistoryError::CreateTransaction)?;

    let command = args().collect::<Vec<_>>().join(" ");

    let id: i64 = transaction
        .query_row(
            INSERT_NEW_MAIN_TABLE,
            (
                if command.is_empty() {
                    None
                } else {
                    Some(command)
                },
                start_time,
                if success { 1 } else { 0 },
                summary.disk_size_delta,
                summary.total_download_size as i64,
                summary
                    .install
                    .iter()
                    .filter(|x| x.op() == &InstallOperation::Install)
                    .count() as i64,
                summary.remove.len() as i64,
                summary
                    .install
                    .iter()
                    .filter(|x| x.op() == &InstallOperation::Upgrade)
                    .count() as i64,
                summary
                    .install
                    .iter()
                    .filter(|x| x.op() == &InstallOperation::Downgrade)
                    .count() as i64,
                summary
                    .install
                    .iter()
                    .filter(|x| x.op() == &InstallOperation::ReInstall)
                    .count() as i64,
                if is_fix_broken { 1 } else { 0 },
                if is_undo { 1 } else { 0 },
            ),
            |row| row.get(0),
        )
        .map_err(HistoryError::ExecuteError)?;

    for i in &summary.install {
        let op: u8 = (*i.op()).into();
        let op = op as i64;

        transaction
            .execute(
                INSERT_INSTALL_TABLE,
                (
                    id,
                    i.name(),
                    i.old_version(),
                    i.new_version(),
                    i.old_size().map(|n| n as i64),
                    i.new_size() as i64,
                    i.download_size() as i64,
                    i.arch(),
                    op,
                ),
            )
            .map_err(HistoryError::ExecuteError)?;
    }

    for i in &summary.remove {
        let Some(version) = i.version() else {
            // 仅为删除配置文件时，版本为空，因此不记录
            continue;
        };

        transaction
            .execute(
                INSERT_REMOVE_TABLE,
                (id, i.name(), version, i.size() as i64, i.arch()),
            )
            .map_err(HistoryError::ExecuteError)?;

        transaction
            .execute(
                INSERT_REMOVE_DETAIL_TABLE,
                (
                    id,
                    i.name(),
                    if i.details().contains(&RemoveTag::AutoRemove) {
                        1
                    } else {
                        0
                    },
                    if i.details().contains(&RemoveTag::Purge) {
                        1
                    } else {
                        0
                    },
                    if i.details().contains(&RemoveTag::Resolver) {
                        1
                    } else {
                        0
                    },
                ),
            )
            .map_err(HistoryError::ExecuteError)?;
    }

    if !topics_enabled.is_empty() || !topics_disabled.is_empty() {
        for i in topics_enabled {
            transaction
                .execute(
                    r#"INSERT INTO "history_topic_oma_1.14" (history_id, topic_name, enable)
                        VALUES (?1, ?2, ?3)"#,
                    (id, i, 1),
                )
                .map_err(HistoryError::ExecuteError)?;
        }

        for i in topics_disabled {
            transaction
                .execute(
                    r#"INSERT INTO "history_topic_oma_1.14" (history_id, topic_name, enable)
                        VALUES (?1, ?2, ?3)"#,
                    (id, i, 0),
                )
                .map_err(HistoryError::ExecuteError)?;
        }
    }

    transaction.commit().map_err(HistoryError::ExecuteError)?;

    Ok(())
}

pub struct HistoryEntry {
    pub id: i64,
    pub time: i64,
    pub command: String,
    pub is_success: bool,
    pub install_count: i64,
    pub remove_count: i64,
    pub upgrade_count: i64,
    pub downgrade_count: i64,
    pub reinstall_count: i64,
    pub is_fixbroken: bool,
    pub is_undo: bool,
}

#[derive(Deserialize)]
pub struct InstallHistoryEntry {
    #[serde(rename = "name")]
    pub pkg_name: String,
    pub old_version: Option<String>,
    pub new_version: String,
    pub old_size: Option<i64>,
    pub new_size: i64,
    pub download_size: i64,
    pub arch: String,
    #[serde(rename = "op")]
    pub operation: InstallOperation,
}

pub struct RemoveHistoryEntryTmp {
    pub pkg_name: String,
    pub version: String,
    pub size: i64,
    pub arch: String,
}

#[derive(Deserialize)]
pub struct RemoveHistoryEntry {
    #[serde(rename = "name")]
    pub pkg_name: String,
    pub version: String,
    pub size: i64,
    pub arch: String,
    #[serde(rename = "details")]
    pub tags: Vec<RemoveTag>,
}

pub fn list_history(conn: &Connection) -> HistoryResult<Vec<HistoryEntry>> {
    let mut res = vec![];
    let stmt = conn.prepare(
        r#"SELECT id, command, time, is_success, install_count, remove_count, upgrade_count, downgrade_count, reinstall_count, is_fixbroken, is_undo
        FROM "history_oma_1.14"
        ORDER BY id DESC"#,
    );

    let mut stmt = match stmt {
        Ok(stmt) => stmt,
        Err(e) => match e {
            Error::SqliteFailure(err, _) if [1, 14].contains(&err.extended_code) => {
                return Err(HistoryError::HistoryEmpty);
            }
            e => return Err(HistoryError::ConnectError(e)),
        },
    };

    let res_iter = stmt
        .query_map([], |row| {
            let id: i64 = row.get(0)?;
            let command: String = row.get(1)?;
            let time: i64 = row.get(2)?;
            let is_success: i64 = row.get(3)?;
            let install_count: i64 = row.get(4)?;
            let remove_count: i64 = row.get(5)?;
            let upgrade_count: i64 = row.get(6)?;
            let downgrade_count: i64 = row.get(7)?;
            let reinstall_count: i64 = row.get(8)?;
            let is_fixbroken: i64 = row.get(9)?;
            let is_undo: i64 = row.get(10)?;

            Ok((
                id,
                command,
                time,
                is_success,
                install_count,
                remove_count,
                upgrade_count,
                downgrade_count,
                reinstall_count,
                is_fixbroken,
                is_undo,
            ))
        })
        .map_err(HistoryError::ExecuteError)?;

    for i in res_iter {
        let (
            id,
            command,
            time,
            is_success,
            install_count,
            remove_count,
            upgrade_count,
            downgrade_count,
            reinstall_count,
            is_fixbroken,
            is_undo,
        ) = i.map_err(HistoryError::ParseDbError)?;

        res.push(HistoryEntry {
            id,
            command,
            time,
            is_success: is_success == 1,
            install_count,
            remove_count,
            upgrade_count,
            downgrade_count,
            reinstall_count,
            is_fixbroken: is_fixbroken == 1,
            is_undo: is_undo == 1,
        });
    }

    if res.is_empty() {
        return Err(HistoryError::HistoryEmpty);
    }

    Ok(res)
}

pub fn find_history_topics_status_by_id(
    conn: &Connection,
    id: i64,
) -> HistoryResult<(Vec<String>, Vec<String>)> {
    let mut query_history_table = conn
        .prepare(
            "SELECT topic_name, enable FROM \"history_topic_oma_1.14\" WHERE history_id = (?1)",
        )
        .map_err(HistoryError::ExecuteError)?;

    let res_iter = query_history_table
        .query_map([id], |row| {
            let topic_name: String = row.get(0)?;
            let enabled: i64 = row.get(1)?;
            Ok((topic_name, enabled == 1))
        })
        .map_err(HistoryError::ExecuteError)?;

    let (mut enabled_topics, mut disabled) = (vec![], vec![]);

    for i in res_iter {
        let (name, enabled) = i.map_err(HistoryError::ParseDbError)?;

        if enabled {
            enabled_topics.push(name);
        } else {
            disabled.push(name);
        }
    }

    Ok((enabled_topics, disabled))
}

pub fn find_history_by_id(conn: &Connection, id: i64) -> HistoryResult<HistoryEntryInner> {
    let mut query_history_table = conn
        .prepare("SELECT is_success, disk_size, total_download_size FROM \"history_oma_1.14\" WHERE id = (?1)")
        .map_err(HistoryError::ExecuteError)?;

    let mut res_iter = query_history_table
        .query_map([id], |row| {
            let is_success: i64 = row.get(0)?;
            let disk_size: i64 = row.get(1)?;
            let total_download_size: i64 = row.get(2)?;

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

        res = Some(HistoryEntryInner {
            install,
            remove,
            disk_size,
            total_download_size,
            is_success,
        })
    }

    res.ok_or_else(|| HistoryError::NoResult(id))
}

pub fn last_upgrade_timestamp(conn: &Connection) -> HistoryResult<i64> {
    let mut prepare = conn
        .prepare(
            r#"SELECT command, time FROM "history_oma_1.14"
    WHERE command LIKE ?"#,
        )
        .map_err(HistoryError::ExecuteError)?;

    let query_str = "% upgrade%";
    let res_iter = prepare
        .query_map([query_str], |row| row.get(1))
        .map_err(HistoryError::ExecuteError)?;

    if let Some(Ok(n)) = res_iter.last() {
        return Ok(n);
    }

    Err(HistoryError::NoUpgradeSystemLog)
}
