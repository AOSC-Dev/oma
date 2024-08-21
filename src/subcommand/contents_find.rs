use std::path::Path;

use oma_console::indicatif::ProgressBar;
use oma_console::pb::spinner_style;
use oma_contents::searcher::{pure_search, ripgrep_search, Mode};

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
        let (style, inv) = spinner_style();
        pb.set_style(style);
        pb.enable_steady_tick(inv);
        pb.set_message(fl!("searching"));

        Some(pb)
    } else {
        None
    };

    let mode = match mode {
        "provides" if is_bin => Mode::BinProvides,
        "provides" => Mode::Provides,
        "files" if is_bin => Mode::BinFiles,
        "files" => Mode::Files,
        _ => unreachable!(),
    };

    let cb = move |count: usize| {
        if let Some(pb) = &pb {
            pb.set_message(fl!("search-with-result-count", count = count));
        }
    };

    let res = if which::which("rg").is_ok() {
        ripgrep_search(
            Path::new(&sysroot).join("var/lib/apt/lists"),
            mode,
            input,
            cb,
        )
    } else {
        pure_search(
            Path::new(&sysroot).join("var/lib/apt/lists"),
            mode,
            input,
            cb,
        )
    }?;

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
