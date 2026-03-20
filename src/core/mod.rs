use std::path::Path;

use fs_extra::dir::get_size as get_dir_size;
use oma_console::{indicatif::HumanBytes, print::Action};
use oma_pm::apt::OmaApt;
use spdlog::warn;

use crate::{color_formatter, fl};

pub mod commit_changes;
pub mod refresh;

pub fn space_tips(apt: &OmaApt, sysroot: impl AsRef<Path>) {
    let space = match fs4::available_space(&sysroot) {
        Ok(space) => space,
        Err(e) => {
            warn!("Unable to get available space: {e}");
            return;
        }
    };

    if space >= 5 * 1024 * 1024 * 1024 {
        return;
    }

    let archive_dir_space = match get_dir_size(apt.get_archive_dir()) {
        Ok(size) => size,
        Err(e) => {
            warn!("Unable to get archive dir space: {e}");
            return;
        }
    };

    if archive_dir_space != 0 {
        let human_space = HumanBytes(archive_dir_space).to_string();
        let cmd = color_formatter()
            .color_str("oma clean", Action::Secondary)
            .to_string();

        warn!("{}", fl!("space-warn", size = human_space, cmd = cmd));
    } else {
        warn!("{}", fl!("space-warn-with-zero"));
    }
}
