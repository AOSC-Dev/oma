use std::{
    io::{Read, Seek, SeekFrom, Write},
    sync::atomic::Ordering,
};

use crate::fl;
use crate::{
    cli::InstallOptions,
    error, info,
    oma::{apt_handler, Action, InstallError, InstallResult, Oma},
    pkg::{mark_delete, mark_install},
    utils::needs_root,
    warn, ARGS, DRYRUN, TIME_OFFSET,
};
use anyhow::{anyhow, Context, Result};
use dialoguer::{theme::ColorfulTheme, Select};
use rust_apt::{cache::Cache, config::Config as AptConfig, new_cache};
use serde::{Deserialize, Serialize};
use time::OffsetDateTime;

const HISTORY_DB_FILE: &'static str = "/var/log/oma/history.json";

#[derive(Serialize, Deserialize, Debug)]
struct History {
    start_date: String,
    end_date: String,
    args: String,
    action: Action,
}

pub fn log_to_file(action: &Action, start_time: &str, end_time: &str) -> Result<()> {
    if DRYRUN.load(Ordering::Relaxed) {
        return Ok(());
    }

    std::fs::create_dir_all("/var/log/oma")
        .map_err(|e| anyhow!("Can not create oma log directory, why: {e}"))?;

    let mut f = std::fs::OpenOptions::new()
        .append(true)
        .create(true)
        .open("/var/log/oma/history")
        .map_err(|e| anyhow!("Can not create oma history file, why: {e}"))?;

    f.write_all(format!("Start-Date: {start_time}\n").as_bytes())?;
    f.write_all(format!("Action: {}\n{action:#?}", *ARGS).as_bytes())?;
    f.write_all(format!("End-Date: {end_time}\n\n").as_bytes())?;

    drop(f);

    let json = History {
        start_date: start_time.to_string(),
        end_date: end_time.to_string(),
        args: (*ARGS.clone()).to_string(),
        action: action.clone(),
    };

    let mut f = std::fs::OpenOptions::new()
        .create(true)
        .read(true)
        .write(true)
        .open(HISTORY_DB_FILE)
        .map_err(|e| anyhow!("Can not read and create oma history database file, why: {e}"))?;

    let mut buf = Vec::new();
    f.read_to_end(&mut buf)?;

    let mut history_db: Vec<History> = if !buf.is_empty() {
        serde_json::from_reader(&*buf).map_err(|e| {
            anyhow!("Can not read oma history database, why: {e}, database is broken?")
        })?
    } else {
        vec![]
    };

    history_db.insert(0, json);

    let buf = serde_json::to_vec(&history_db)
        .map_err(|e| anyhow!("BUG: Can not serialize database, why: {e}, Please report to upstream: https://github.com/aosc-dev/oma"))?;

    f.seek(SeekFrom::Start(0))?;
    f.write_all(&buf)?;

    Ok(())
}

pub fn run(index: Option<usize>, is_undo: bool) -> Result<()> {
    needs_root()?;

    let buf = std::fs::read(HISTORY_DB_FILE)
        .map_err(|e| anyhow!("Can not read history database, why: {e}"))?;

    let db: Vec<History> = serde_json::from_reader(&*buf)
        .map_err(|e| anyhow!("Can not read oma history database, why: {e}, database is broken?"))?;

    let action = if let Some(index) = index {
        let history = db
            .get(index)
            .context(format!("Can not get index: {index}"))?;

        let action = &history.action;
        if action.is_empty() {
            info!("index: {} does not any action", index);
            return Ok(());
        }

        action
    } else {
        let theme = ColorfulTheme::default();
        let mut dialoguer = Select::with_theme(&theme);

        let db_with_args = db
            .iter()
            .enumerate()
            .map(|(i, x)| format!("[{}]: {}", i + 1, x.args))
            .collect::<Vec<_>>();

        dialoguer.items(&db_with_args);
        dialoguer.with_prompt(format!(
            "Select an operation ID to {}:",
            if is_undo { "undo" } else { "redo" }
        ));
        dialoguer.default(0);
        let index = dialoguer.interact()?;

        let history = db.get(index).unwrap();

        let action = &history.action;
        if action.is_empty() {
            info!("index: {} does not any action", index);
            return Ok(());
        }

        action
    };

    let mut count = 1;

    loop {
        match do_inner(action, count, is_undo) {
            Ok((a, s, e)) => return log_to_file(&a, &s, &e),
            Err(e) => {
                match e {
                    InstallError::Anyhow(e) => return Err(e),
                    InstallError::RustApt(e) => {
                        warn!("apt has retrn non-zero code, retrying {count} times");
                        // Retry 3 times, if Error is rust_apt return
                        if count == 3 {
                            return Err(e.into());
                        }
                        count += 1;
                    }
                }
            }
        }
    }
}

fn do_inner(
    action: &Action,
    count: usize,
    is_undo: bool,
) -> InstallResult<(Action, String, String)> {
    let cache = new_cache!()?;
    let (action, len) = if is_undo {
        undo_inner(action, &cache)?
    } else {
        redo_inner(action, &cache)?
    };

    let start_time = OffsetDateTime::now_utc()
        .to_offset(*TIME_OFFSET)
        .to_string();

    Oma::build_async_runtime()?.action_to_install(
        AptConfig::new_clear(),
        &action,
        count,
        cache,
        len,
        &InstallOptions::default(),
    )?;

    let end_time = OffsetDateTime::now_utc()
        .to_offset(*TIME_OFFSET)
        .to_string();

    Ok((action, start_time, end_time))
}

fn undo_inner(action: &Action, cache: &Cache) -> Result<(Action, usize), InstallError> {
    for i in &action.update {
        let pkg = cache.get(&i.name_no_color);
        if let Some(pkg) = pkg {
            if let Some(v) = pkg.get_version(&i.old_version.as_ref().unwrap()) {
                mark_install(cache, pkg.name(), v.unique(), false, false, None)?;
                continue;
            }
        }

        error!(
            "{}",
            fl!(
                "can-not-get-pkg-version-from-database",
                name = i.name_no_color.to_string(),
                version = i.old_version.as_ref().unwrap().to_string()
            )
        );
    }
    for i in &action.downgrade {
        let pkg = cache.get(&i.name_no_color);
        if let Some(pkg) = pkg {
            if let Some(v) = pkg.get_version(&i.old_version.as_ref().unwrap()) {
                mark_install(cache, pkg.name(), v.unique(), false, false, None)?;
                continue;
            }
        }

        error!(
            "{}",
            fl!(
                "can-not-get-pkg-version-from-database",
                name = i.name_no_color.to_string(),
                version = i.old_version.as_ref().unwrap().to_string()
            )
        );
    }
    for i in &action.del {
        let pkg = cache.get(&i.name_no_color);
        if let Some(pkg) = pkg {
            if let Some(v) = pkg.get_version(&i.version) {
                mark_install(cache, pkg.name(), v.unique(), false, false, None)?;
                continue;
            }
        }

        error!(
            "{}",
            fl!(
                "can-not-get-pkg-version-from-database",
                name = i.name_no_color.to_string(),
                version = i.version.to_string()
            )
        );
    }
    for i in &action.install {
        let pkg = cache.get(&i.name_no_color);
        if let Some(pkg) = pkg {
            if let Some(v) = pkg.installed() {
                if v.version() == i.new_version {
                    mark_delete(&pkg, false)?;
                    continue;
                }
            }
        }

        error!(
            "{}",
            fl!(
                "can-not-get-pkg-version-from-database",
                name = i.name_no_color.to_string(),
                version = i.version.to_string()
            )
        );
    }

    let (action, len, _) = apt_handler(cache, false, false, true)?;

    Ok((action, len))
}

fn redo_inner(action: &Action, cache: &Cache) -> Result<(Action, usize), InstallError> {
    for i in &action.update {
        let pkg = cache.get(&i.name_no_color);
        if let Some(pkg) = pkg {
            if let Some(v) = pkg.get_version(&i.new_version) {
                mark_install(cache, pkg.name(), v.unique(), false, false, None)?;
                continue;
            }
        }

        error!(
            "{}",
            fl!(
                "can-not-get-pkg-version-from-database",
                name = i.name_no_color.to_string(),
                version = i.new_version.to_string()
            )
        );
    }
    for i in &action.downgrade {
        let pkg = cache.get(&i.name_no_color);
        if let Some(pkg) = pkg {
            if let Some(v) = pkg.get_version(&i.new_version) {
                mark_install(cache, pkg.name(), v.unique(), false, false, None)?;
                continue;
            }
        }

        error!(
            "{}",
            fl!(
                "can-not-get-pkg-version-from-database",
                name = i.name_no_color.to_string(),
                version = i.new_version.to_string()
            )
        );
    }
    for i in &action.del {
        let pkg = cache.get(&i.name_no_color);
        if let Some(pkg) = pkg {
            mark_delete(&pkg, false)?;
            continue;
        }

        error!(
            "{}",
            fl!(
                "can-not-get-pkg-version-from-database",
                name = i.name_no_color.to_string(),
                version = i.version.to_string()
            )
        );
    }
    for i in &action.install {
        let pkg = cache.get(&i.name_no_color);
        if let Some(pkg) = pkg {
            if let Some(v) = pkg.installed() {
                if v.version() == i.new_version {
                    mark_install(cache, pkg.name(), v.unique(), false, false, None)?;
                    continue;
                }
            }
        }

        error!(
            "{}",
            fl!(
                "can-not-get-pkg-version-from-database",
                name = i.name_no_color.to_string(),
                version = i.version.to_string()
            )
        );
    }

    let (action, len, _) = apt_handler(cache, false, false, true)?;

    Ok((action, len))
}
