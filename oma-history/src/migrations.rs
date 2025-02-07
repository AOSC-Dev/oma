use oma_pm_operation_type::{InstallOperation, RemoveTag};
use rusqlite::Connection;
use serde::Deserialize;
use tracing::{debug, info, warn};

use crate::{
    HistoryEntryInner, HistoryError, HistoryResult, InstallHistoryEntry, RemoveHistoryEntry,
    INSERT_INSTALL_TABLE, INSERT_NEW_MAIN_TABLE, INSERT_REMOVE_DETAIL_TABLE, INSERT_REMOVE_TABLE,
};

pub fn create_and_maybe_migration_from_oma_db_v2(conn: &Connection) -> HistoryResult<()> {
    create_history_table(conn)?;

    let old_db_count: i32 = conn
        .query_row(
            "SELECT COUNT(name) FROM sqlite_schema WHERE name = 'history_oma_1.2'",
            [],
            |row| row.get(0),
        )
        .map_err(HistoryError::ExecuteError)?;

    let new_db_count: i32 = conn
        .query_row("SELECT COUNT(id) FROM 'history_oma_1.14'", [], |row| {
            row.get(0)
        })
        .map_err(HistoryError::ExecuteError)?;

    if old_db_count != 0 && new_db_count == 0 {
        info!("Migrating oma history database, this may take a few minutes ...");
        migration_from_oma_db_v2(conn)?;
    }

    Ok(())
}

struct OldTableEntry {
    inner: HistoryEntryInner,
    _id: i64,
    time: i64,
    summary_type: OldSummaryType,
}

fn handle_packages_items(items: &[String]) -> String {
    items
        .iter()
        .map(|x| {
            x.split_once(" ")
                .map(|x| (x.0.to_string(), x.1.to_string()))
                .unwrap_or_else(|| (x.to_string(), "".to_string()))
                .0
        })
        .collect::<Vec<_>>()
        .join(" ")
}

fn migration_from_oma_db_v2(conn: &Connection) -> HistoryResult<()> {
    let table = get_old_table(conn)?;

    for entry in table {
        let command = match &entry.summary_type {
            OldSummaryType::Install(items) => {
                format!("oma install {}", handle_packages_items(items))
            }
            OldSummaryType::Upgrade(items) => {
                format!("oma upgrade {}", handle_packages_items(items))
            }
            OldSummaryType::Remove(items) => format!("oma remove {}", handle_packages_items(items)),
            OldSummaryType::Changes => "oma tui".to_string(),
            OldSummaryType::FixBroken => "oma fix-broken".to_string(),
            OldSummaryType::TopicsChanged { add, remove } => {
                let mut s = "oma topics".to_string();
                if !add.is_empty() {
                    s.push_str(" --opt-in ");
                    s.push_str(&add.join(" "));
                }
                if !remove.is_empty() {
                    s.push_str(" --opt-out ");
                    s.push_str(&remove.join(" "));
                }
                s
            }
            OldSummaryType::Undo => "oma undo".to_string(),
        };

        let id: i64 = conn
            .query_row(
                INSERT_NEW_MAIN_TABLE,
                (
                    command,
                    entry.time,
                    entry.inner.is_success,
                    entry.inner.disk_size,
                    entry.inner.total_download_size,
                    entry
                        .inner
                        .install
                        .iter()
                        .filter(|x| x.operation == InstallOperation::Install)
                        .count(),
                    entry.inner.remove.len(),
                    entry
                        .inner
                        .install
                        .iter()
                        .filter(|x| x.operation == InstallOperation::Upgrade)
                        .count(),
                    entry
                        .inner
                        .install
                        .iter()
                        .filter(|x| x.operation == InstallOperation::Downgrade)
                        .count(),
                    entry
                        .inner
                        .install
                        .iter()
                        .filter(|x| x.operation == InstallOperation::ReInstall)
                        .count(),
                    entry.summary_type == OldSummaryType::FixBroken,
                    entry.summary_type == OldSummaryType::Undo,
                ),
                |row| row.get(0),
            )
            .map_err(HistoryError::ExecuteError)?;

        for i in &entry.inner.install {
            let op: u8 = i.operation.into();
            let op = op as i64;

            conn.execute(
                INSERT_INSTALL_TABLE,
                (
                    id,
                    &i.pkg_name,
                    &i.old_version,
                    &i.new_version,
                    i.old_size,
                    i.new_size,
                    i.download_size,
                    &i.arch,
                    op,
                ),
            )
            .map_err(HistoryError::ExecuteError)?;
        }

        for j in &entry.inner.remove {
            conn.execute(
                INSERT_REMOVE_TABLE,
                (id, &j.pkg_name, &j.version, j.size, &j.arch),
            )
            .map_err(HistoryError::ExecuteError)?;

            conn.execute(
                INSERT_REMOVE_DETAIL_TABLE,
                (
                    id,
                    &j.pkg_name,
                    if j.tags.contains(&RemoveTag::AutoRemove) {
                        1
                    } else {
                        0
                    },
                    if j.tags.contains(&RemoveTag::Purge) {
                        1
                    } else {
                        0
                    },
                    if j.tags.contains(&RemoveTag::Resolver) {
                        1
                    } else {
                        0
                    },
                ),
            )
            .map_err(HistoryError::ExecuteError)?;
        }
    }

    Ok(())
}

#[derive(Debug, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
pub enum OldSummaryType {
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

fn get_old_table(conn: &Connection) -> Result<Vec<OldTableEntry>, HistoryError> {
    let mut stmt = conn
        .prepare("SELECT id, time, install_packages, remove_packages, disk_size, total_download_size, is_success, typ FROM \"history_oma_1.2\" ORDER BY id ASC")
        .map_err(HistoryError::ExecuteError)?;
    let res_iter = stmt
        .query_map([], |row| {
            let id: i64 = row.get(0)?;
            let time: i64 = row.get(1)?;
            let install_packages: String = row.get(2)?;
            let remove_packages: String = row.get(3)?;
            let disk_size: i64 = row.get(4)?;
            let total_download_size: u64 = row.get(5)?;
            let is_success: i64 = row.get(6)?;
            let summary_type: String = row.get(7)?;

            Ok((
                id,
                time,
                install_packages,
                remove_packages,
                disk_size,
                total_download_size,
                is_success,
                summary_type,
            ))
        })
        .map_err(HistoryError::ExecuteError)?;
    let mut res = vec![];
    for i in res_iter {
        let (
            id,
            time,
            install_packages,
            remove_packages,
            disk_size,
            total_download_size,
            is_success,
            summary_type,
        ) = i.map_err(HistoryError::ExecuteError)?;

        let install_packages =
            match serde_json::from_str::<Vec<InstallHistoryEntry>>(&install_packages) {
                Ok(i) => i,
                Err(e) => {
                    warn!("Failed to parse item: {e}");
                    debug!("install packages: {}", install_packages);
                    continue;
                }
            };

        let remove_packages =
            match serde_json::from_str::<Vec<RemoveHistoryEntry>>(&remove_packages) {
                Ok(i) => i,
                Err(e) => {
                    warn!("Failed to parse item: {e}");
                    debug!("remove packages: {}", &remove_packages);
                    continue;
                }
            };

        let summary_type = match serde_json::from_str::<OldSummaryType>(&summary_type) {
            Ok(s) => s,
            Err(e) => {
                warn!("Failed to parse item: {e}");
                debug!("summary type: {}", &summary_type);
                continue;
            }
        };

        res.push(OldTableEntry {
            inner: HistoryEntryInner {
                install: install_packages,
                remove: remove_packages,
                disk_size,
                total_download_size,
                is_success: is_success == 1,
            },
            _id: id,
            time,
            summary_type,
        })
    }

    Ok(res)
}

fn create_history_table(conn: &Connection) -> HistoryResult<()> {
    conn.execute(
        "CREATE TABLE IF NOT EXISTS \"history_oma_1.14\" (
                id INTEGER PRIMARY KEY,
                command TEXT,
                time INTEGER NOT NULL,
                is_success INTEGER NOT NULL,
                disk_size INTEGER NOT NULL,
                total_download_size INTEGER,
                install_count INTEGER NOT NULL,
                remove_count INTEGER NOT NULL,
                upgrade_count INTEGER NOT NULL,
                downgrade_count INTEGER NOT NULL,
                reinstall_count INTEGER NOT NULL,
                is_fixbroken INTEGER NOT NULL,
                is_undo INTEGER NOT NULL
            )",
        (),
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

    conn.execute(
        "CREATE TABLE IF NOT EXISTS \"history_topic_oma_1.14\" (
            history_id INTEGER NOT NULL,
            topic_name TEXT NOT NULL,
            enable INTEGER NOT NULL
        )",
        (),
    )
    .map_err(HistoryError::ExecuteError)?;

    Ok(())
}
