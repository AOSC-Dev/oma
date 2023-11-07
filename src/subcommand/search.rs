use std::sync::atomic::Ordering;

use dialoguer::console::style;
use oma_console::{
    indicatif::ProgressBar,
    pb::oma_spinner,
    writer::{gen_prefix, writeln_inner, MessageType},
    WRITER,
};
use oma_pm::{
    apt::{OmaApt, OmaAptArgsBuilder},
    query::OmaDatabase,
    PackageStatus,
};

use crate::{error::OutputError, table::oma_display_with_normal_output};
use crate::{fl, AILURUS};

use super::utils::check_unsupport_stmt;

pub fn execute(args: &[String], no_progress: bool, sysroot: String) -> Result<i32, OutputError> {
    for arg in args {
        check_unsupport_stmt(arg);
    }

    let oma_apt_args = OmaAptArgsBuilder::default().sysroot(sysroot).build()?;
    let apt = OmaApt::new(vec![], oma_apt_args, false)?;
    let db = OmaDatabase::new(&apt.cache)?;
    let s = args.join(" ");

    let (sty, inv) = oma_spinner(AILURUS.load(Ordering::Relaxed));

    let pb = if !no_progress {
        let pb = ProgressBar::new_spinner().with_style(sty);
        pb.enable_steady_tick(inv);
        pb.set_message(fl!("searching"));

        Some(pb)
    } else {
        None
    };

    let res = db.search(&s)?;

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
            style(&i.name).bold().color256(141).to_string()
        } else {
            style(&i.name).bold().color256(148).to_string()
        };

        pkg_info_line.push(' ');

        if i.status == PackageStatus::Upgrade {
            pkg_info_line.push_str(&format!(
                "{} -> {}",
                style(i.old_version.unwrap()).color256(214),
                style(&i.new_version).color256(114)
            ));
        } else {
            pkg_info_line.push_str(&style(&i.new_version).color256(114).to_string());
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
                &style(format!("[{}]", pkg_tags.join(",")))
                    .color256(178)
                    .to_string(),
            );
        }

        let prefix = match i.status {
            PackageStatus::Avail => style(fl!("pkg-search-avail")).dim(),
            PackageStatus::Installed => style(fl!("pkg-search-installed")).color256(72),
            PackageStatus::Upgrade => style(fl!("pkg-search-upgrade")).color256(214),
        }
        .to_string();

        writeln!(writer, "{}{}", gen_prefix(&prefix, 10), pkg_info_line).ok();

        writeln_inner(&i.desc, "", WRITER.get_max_len().into(), |t, s| {
            match t {
                MessageType::Msg => writeln!(writer, "{}", style(s.trim()).color256(182)),
                MessageType::Prefix => write!(writer, "{}", gen_prefix(s, 10)),
            }
            .ok();
        });
    }

    drop(writer);
    pager.wait_for_exit().map_err(|e| OutputError {
        description: "Failed to wait pager exit".to_string(),
        source: Some(Box::new(e)),
    })?;

    Ok(0)
}
