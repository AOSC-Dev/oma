use anyhow::anyhow;
use std::sync::atomic::Ordering;

use crate::{
    error::OutputError,
    history::{connect_db, list_history},
    table::table_pending_inner,
    ALLOWCTRLC,
};

use super::utils::{dialoguer_select_history, format_summary_log};
use crate::fl;

pub fn execute() -> Result<i32, OutputError> {
    let conn = connect_db(false)?;
    let list = list_history(conn)?;
    let display_list = format_summary_log(&list, false);

    let has_x11 = std::env::var("DISPLAY");
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

        let tips = if has_x11.is_ok() {
            fl!("normal-tips-with-x11")
        } else {
            fl!("normal-tips")
        };

        table_pending_inner(true, tips, false, remove, install, disk_size, false)?;
    }
}
