use std::borrow::Cow;

use oma_console::{indicatif::ProgressBar, pager::Pager, pb::spinner_style};
use oma_pm::{
    apt::{AptConfig, OmaApt, OmaAptArgs},
    matches::{PackagesMatcher, SearchEngine},
};
use oma_utils::dpkg::dpkg_arch;
use tracing::warn;

use crate::{error::OutputError, table::oma_display_with_normal_output};
use crate::{fl, utils::SearchResultDisplay};

pub fn execute(
    args: &[String],
    no_progress: bool,
    sysroot: String,
    engine: Cow<String>,
    no_pager: bool,
    json: bool,
    another_apt_options: Vec<String>,
) -> Result<i32, OutputError> {
    let arch = dpkg_arch(&sysroot)?;

    let oma_apt_args = OmaAptArgs::builder()
        .another_apt_options(another_apt_options)
        .sysroot(sysroot)
        .build();

    let apt = OmaApt::new(vec![], oma_apt_args, false, AptConfig::new())?;
    let matcher = PackagesMatcher::builder()
        .cache(&apt.cache)
        .native_arch(&arch)
        .build();

    let s = args.concat();

    let (sty, inv) = spinner_style();

    let pb = if !no_progress && !json {
        let pb = ProgressBar::new_spinner().with_style(sty);
        pb.enable_steady_tick(inv);
        pb.set_message(fl!("searching"));

        Some(pb)
    } else {
        None
    };

    let res = matcher.search(
        &s,
        match engine.as_str() {
            "indicium" => SearchEngine::Indicium(Box::new(|_| {})),
            "strsim" => SearchEngine::Strsim,
            "text" => SearchEngine::Text,
            x => {
                warn!("Unsupported mode: {x}, fallback to indicium ...");
                SearchEngine::Indicium(Box::new(|_| {}))
            }
        },
    )?;

    if let Some(pb) = pb {
        pb.finish_and_clear();
    }

    let mut pager = if !no_pager && !json {
        oma_display_with_normal_output(false, res.len() * 2)?
    } else {
        Pager::plain()
    };

    let mut writer = pager.get_writer().map_err(|e| OutputError {
        description: "Failed to get writer".to_string(),
        source: Some(Box::new(e)),
    })?;

    if !json {
        for i in res {
            write!(writer, "{}", SearchResultDisplay(&i)).ok();
        }
    } else {
        writeln!(
            writer,
            "{}",
            serde_json::to_string(&res).map_err(|e| OutputError {
                description: e.to_string(),
                source: None,
            })?
        )
        .ok();
    }

    drop(writer);
    let exit = pager.wait_for_exit().map_err(|e| OutputError {
        description: "Failed to wait exit".to_string(),
        source: Some(Box::new(e)),
    })?;

    Ok(exit.into())
}
