use anyhow::anyhow;
use std::sync::atomic::Ordering;

use crate::{
    error::OutputError,
    history::{connect_or_create_db, list_history},
    ALLOWCTRLC, table::table_for_history_pending,
};

use super::utils::{dialoguer_select_history, format_summary_log};

pub fn execute() -> Result<i32, OutputError> {
    let conn = connect_or_create_db(false)?;
    let list = list_history(conn)?;
    let display_list = format_summary_log(&list, false);

    ALLOWCTRLC.store(true, Ordering::Relaxed);

    let mut old_selected = 0;

    loop {
        let selected =
            dialoguer_select_history(&display_list, old_selected).map_err(|_| anyhow!(""))?;
        old_selected = selected;

        let selected = &list[selected].0;
        let op = &selected.op;
        let install = &op.install;
        let remove = &op.remove;
        let disk_size = &op.disk_size;

        table_for_history_pending(install, remove, disk_size)?;
    }
}
