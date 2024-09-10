use crate::error::OutputError;
use crate::fl;
use crate::table::oma_display_with_normal_output;
use indexmap::IndexSet;
use oma_console::indicatif::ProgressBar;
use oma_console::pb::spinner_style;
use oma_contents::searcher::{pure_search, ripgrep_search, Mode};
use std::{
    io::{stdout, Write},
    path::Path,
};

pub fn execute(
    mode: &str,
    is_bin: bool,
    input: &str,
    no_progress: bool,
    sysroot: String,
    println: bool,
) -> Result<i32, OutputError> {
    let pb = if !no_progress && !println {
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

    let mut res = IndexSet::with_hasher(ahash::RandomState::new());
    let mut count = 0;

    let cb = |line: (String, String)| {
        if println {
            writeln!(stdout(), "{}: {}", line.0, line.1).ok();
        } else if !res.contains(&line) {
            res.insert(line);
            count += 1;
            if let Some(pb) = &pb {
                pb.set_message(fl!("search-with-result-count", count = count));
            }
        }
    };

    if which::which("rg").is_ok() {
        ripgrep_search(
            Path::new(&sysroot).join("var/lib/apt/lists"),
            mode,
            input,
            cb,
        )?;
    } else {
        pure_search(
            Path::new(&sysroot).join("var/lib/apt/lists"),
            mode,
            input,
            cb,
        )?;
    };

    if let Some(pb) = &pb {
        pb.finish_and_clear();
    }

    if println {
        return Ok(0);
    }

    let mut pager = oma_display_with_normal_output(false, res.len())?;
    let mut out = pager.get_writer().map_err(|e| OutputError {
        description: "Failed to create writer".to_string(),
        source: Some(Box::new(e)),
    })?;

    for (pkg, file) in res {
        writeln!(out, "{pkg}: {file}").ok();
    }

    drop(out);

    let exit = pager.wait_for_exit().map_err(|e| OutputError {
        description: "Failed to wait exit".to_string(),
        source: Some(Box::new(e)),
    })?;

    Ok(exit.into())
}
