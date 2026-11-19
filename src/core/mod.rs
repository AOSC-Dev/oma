use std::path::Path;

use fs_extra::dir::get_size as get_dir_size;
use oma_logger::warn;
use oma_pm::apt::OmaApt;
use oma_utils::human_bytes::HumanBytes;

use crate::{color::Colorize, fl};

pub mod commit_changes;
pub mod refresh;

/// Warn below this much free space. Afterglow systems often have storage
/// devices with less than 5GiB of space to begin with, so lower the
/// threshold for Afterglow.
#[cfg(feature = "afterglow")]
/// 256MiB (in bytes).
const SPACE_WARN_THRESHOLD: u64 = 256 * 1024 * 1024;
#[cfg(not(feature = "afterglow"))]
/// 5GiB (in bytes).
const SPACE_WARN_THRESHOLD: u64 = 5 * 1024 * 1024 * 1024;

pub fn space_tips(apt: &OmaApt, sysroot: impl AsRef<Path>) {
    let space = match fs4::available_space(&sysroot) {
        Ok(space) => space,
        Err(e) => {
            warn!("Unable to get available space: {e}");
            return;
        }
    };

    if space >= SPACE_WARN_THRESHOLD {
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
        let cmd = "oma clean".secondary_color();

        warn!("{}", fl!("space-warn", size = human_space, cmd = cmd));
    } else {
        warn!("{}", fl!("space-warn-with-zero"));
    }
}
