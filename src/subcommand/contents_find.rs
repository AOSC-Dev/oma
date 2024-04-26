use std::path::Path;

use dialoguer::console;
use oma_console::writer::bar_writeln;
use oma_console::{indicatif::ProgressBar, pb::oma_spinner};
use oma_contents::{ContentsEvent, QueryMode};
use oma_utils::dpkg::dpkg_arch;

use crate::error::OutputError;
use crate::fl;
use crate::table::oma_display_with_normal_output;

pub fn execute(
    mode: &str,
    is_bin: bool,
    input: &str,
    no_progress: bool,
    sysroot: String,
) -> Result<i32, OutputError> {
    let pb = if !no_progress {
        let pb = ProgressBar::new_spinner();
        let (style, inv) = oma_spinner(false);
        pb.set_style(style);
        pb.enable_steady_tick(inv);
        pb.set_message(fl!("searching"));

        Some(pb)
    } else {
        None
    };

    let query_mode = match mode {
        "files" => QueryMode::ListFiles(is_bin),
        "provides" => QueryMode::Provides(is_bin),
        _ => unreachable!(),
    };

    let arch = dpkg_arch(&sysroot)?;

    let res = oma_contents::find(
        input,
        query_mode,
        &Path::new(&sysroot).join("var/lib/apt/lists"),
        &arch,
        move |c| match c {
            ContentsEvent::Progress(c) => {
                if let Some(pb) = &pb {
                    pb.set_message(fl!("search-with-result-count", count = c));
                }
            }
            ContentsEvent::ContentsMayNotBeAccurate => {
                if let Some(pb) = &pb {
                    bar_writeln(
                        |s| pb.println(s),
                        &console::style("WARNING").yellow().bold().to_string(),
                        &fl!("contents-may-not-be-accurate-1"),
                    );
                    bar_writeln(
                        |s| pb.println(s),
                        &console::style("WARNING").yellow().bold().to_string(),
                        &fl!("contents-may-not-be-accurate-2"),
                    );
                }
            }
            ContentsEvent::Done => {
                if let Some(pb) = &pb {
                    pb.finish_and_clear();
                }
            }
        },
        arch != "mips64r6el",
    )?;

    let mut pager = oma_display_with_normal_output(false, res.len())?;
    let mut out = pager.get_writer().map_err(|e| OutputError {
        description: "Failed to create writer".to_string(),
        source: Some(Box::new(e)),
    })?;

    for (pkg, file) in res {
        writeln!(out, "{pkg}: {file}").ok();
    }

    drop(out);
    pager.wait_for_exit().map_err(|e| OutputError {
        description: "Failed to wait exit".to_string(),
        source: Some(Box::new(e)),
    })?;

    Ok(0)
}
