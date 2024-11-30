use crate::fl;
use crate::table::oma_display_with_normal_output;
use crate::{config::Config, error::OutputError};
use clap::Args;
use indexmap::IndexSet;
use oma_console::indicatif::ProgressBar;
use oma_console::pb::spinner_style;
use oma_contents::searcher::Mode;
use std::io::{stdout, Write};
use std::path::PathBuf;

use super::utils::contents_search;

use crate::args::CliExecuter;

enum CliMode {
    Provides,
    Files,
}

#[derive(Debug, Args)]
pub struct Files {
    /// Search binary of package(s)
    #[arg(long)]
    bin: bool,
    /// Package to display a list files of
    package: String,
    /// Output result to stdout, not pager
    #[arg(long, visible_alias = "println")]
    no_pager: bool,
    /// Set sysroot target directory
    #[arg(from_global)]
    sysroot: PathBuf,
}

impl CliExecuter for Files {
    fn execute(self, _config: &Config, no_progress: bool) -> Result<i32, OutputError> {
        let Files {
            bin,
            package,
            no_pager,
            sysroot,
        } = self;
        execute(
            CliMode::Files,
            bin,
            &package,
            no_progress,
            sysroot.to_string_lossy().to_string(),
            no_pager,
        )
    }
}

#[derive(Debug, Args)]
pub struct Provides {
    /// Search binary of package(s)
    #[arg(long)]
    bin: bool,
    /// Keywords, parts of a path, executable names to search
    pattern: String,
    /// Output result to stdout, not pager
    #[arg(long, visible_alias = "println")]
    no_pager: bool,
    /// Set sysroot target directory
    #[arg(from_global)]
    sysroot: PathBuf,
}

impl CliExecuter for Provides {
    fn execute(self, _config: &Config, no_progress: bool) -> Result<i32, OutputError> {
        let Provides {
            bin,
            pattern,
            no_pager,
            sysroot,
        } = self;
        execute(
            CliMode::Provides,
            bin,
            &pattern,
            no_progress,
            sysroot.to_string_lossy().to_string(),
            no_pager,
        )
    }
}

fn execute(
    mode: CliMode,
    is_bin: bool,
    input: &str,
    no_progress: bool,
    sysroot: String,
    no_pager: bool,
) -> Result<i32, OutputError> {
    let pb = if !no_progress && !no_pager {
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
        CliMode::Provides if is_bin => Mode::BinProvides,
        CliMode::Provides => Mode::Provides,
        CliMode::Files if is_bin => Mode::BinFiles,
        CliMode::Files => Mode::Files,
    };

    let mut res = IndexSet::with_hasher(ahash::RandomState::new());
    let mut count = 0;

    let cb = |line: (String, String)| {
        if no_pager {
            writeln!(stdout(), "{}: {}", line.0, line.1).ok();
        } else if !res.contains(&line) {
            res.insert(line);
            count += 1;
            if let Some(pb) = &pb {
                pb.set_message(fl!("search-with-result-count", count = count));
            }
        }
    };

    contents_search(sysroot, mode, input, cb)?;

    if let Some(pb) = &pb {
        pb.finish_and_clear();
    }

    if no_pager {
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
