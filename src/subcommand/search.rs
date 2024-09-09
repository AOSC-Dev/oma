use std::borrow::Cow;

use dialoguer::console::style;
use oma_console::{
    indicatif::ProgressBar,
    pb::spinner_style,
    print::Action,
    writer::{gen_prefix, writeln_inner, MessageType},
    WRITER,
};
use oma_pm::{
    apt::{AptConfig, OmaApt, OmaAptArgs},
    query::{OmaDatabase, SearchEngine},
    PackageStatus,
};
use tracing::warn;

use crate::fl;
use crate::{color_formatter, error::OutputError, table::oma_display_with_normal_output};

use super::utils::check_unsupport_stmt;

pub fn execute(
    args: &[String],
    no_progress: bool,
    sysroot: String,
    engine: Cow<String>,
) -> Result<i32, OutputError> {
    for arg in args {
        check_unsupport_stmt(arg);
    }

    let oma_apt_args = OmaAptArgs::builder().sysroot(sysroot).build();
    let apt = OmaApt::new(vec![], oma_apt_args, false, AptConfig::new())?;
    let db = OmaDatabase::new(&apt.cache)?;
    let s = args.concat();

    let (sty, inv) = spinner_style();

    let pb = if !no_progress {
        let pb = ProgressBar::new_spinner().with_style(sty);
        pb.enable_steady_tick(inv);
        pb.set_message(fl!("searching"));

        Some(pb)
    } else {
        None
    };

    let res = db.search(
        &s,
        match engine.as_str() {
            "indicium" => SearchEngine::Indicium(Box::new(|_| {})),
            "strsim" => SearchEngine::Strsim,
            x => {
                warn!("Unsupport mode: {x}, fallback to indicium ...");
                SearchEngine::Indicium(Box::new(|_| {}))
            }
        },
    )?;

    if let Some(pb) = pb {
        pb.finish_and_clear();
    }

    let mut pager = oma_display_with_normal_output(false, res.len() * 2)?;

    let mut writer = pager.get_writer().map_err(|e| OutputError {
        description: "Failed to get writer".to_string(),
        source: Some(Box::new(e)),
    })?;

    for i in res {
        let mut pkg_info_line = if i.is_base {
            color_formatter()
                .color_str(&i.name, Action::Purple)
                .bold()
                .to_string()
        } else {
            color_formatter()
                .color_str(&i.name, Action::Emphasis)
                .bold()
                .to_string()
        };

        pkg_info_line.push(' ');

        if i.status == PackageStatus::Upgrade {
            pkg_info_line.push_str(&format!(
                "{} -> {}",
                color_formatter().color_str(i.old_version.unwrap(), Action::WARN),
                color_formatter().color_str(&i.new_version, Action::EmphasisSecondary)
            ));
        } else {
            pkg_info_line.push_str(
                &color_formatter()
                    .color_str(&i.new_version, Action::EmphasisSecondary)
                    .to_string(),
            );
        }

        let mut pkg_tags = vec![];

        if i.dbg_package {
            pkg_tags.push(fl!("debug-symbol-available"));
        }

        if i.full_match {
            pkg_tags.push(fl!("full-match"))
        }

        if !pkg_tags.is_empty() {
            pkg_info_line.push(' ');
            pkg_info_line.push_str(
                &color_formatter()
                    .color_str(format!("[{}]", pkg_tags.join(",")), Action::Note)
                    .to_string(),
            );
        }

        let prefix = match i.status {
            PackageStatus::Avail => style("AVAIL").dim(),
            PackageStatus::Installed => {
                color_formatter().color_str("INSTALLED", Action::Foreground)
            }
            PackageStatus::Upgrade => color_formatter().color_str("UPGRADE", Action::WARN),
        }
        .to_string();

        writeln!(writer, "{}{}", gen_prefix(&prefix, 10), pkg_info_line).ok();

        writeln_inner(
            &i.desc,
            "",
            WRITER.get_max_len().into(),
            WRITER.get_prefix_len(),
            |t, s| {
                match t {
                    MessageType::Msg => {
                        writeln!(
                            writer,
                            "{}",
                            color_formatter().color_str(s.trim(), Action::Secondary)
                        )
                    }
                    MessageType::Prefix => write!(writer, "{}", gen_prefix(s, 10)),
                }
                .ok();
            },
        );
    }

    drop(writer);
    pager.wait_for_exit().map_err(|e| OutputError {
        description: "Failed to wait pager exit".to_string(),
        source: Some(Box::new(e)),
    })?;

    Ok(0)
}
