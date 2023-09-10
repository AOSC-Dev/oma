use dialoguer::console::style;
use oma_console::{indicatif::ProgressBar, pb::oma_spinner, writer::gen_prefix};
use oma_pm::{
    apt::{OmaApt, OmaAptArgsBuilder},
    query::OmaDatabase,
    PackageStatus,
};

use crate::fl;
use crate::{error::OutputError, table::oma_display};

use super::utils::check_unsupport_stmt;

pub fn execute(args: &[String], no_progress: bool) -> Result<i32, OutputError> {
    for arg in args {
        check_unsupport_stmt(arg);
    }

    let oma_apt_args = OmaAptArgsBuilder::default().build()?;
    let apt = OmaApt::new(vec![], oma_apt_args, false)?;
    let db = OmaDatabase::new(&apt.cache)?;
    let s = args.join(" ");

    let (sty, inv) = oma_spinner(false);

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

    let mut pager = oma_display(false, res.len())?;

    let mut writer = pager.get_writer()?;

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

        if i.dbg_package {
            pkg_info_line.push(' ');
            pkg_info_line.push_str(&style(fl!("debug-symbol-available")).dim().to_string());
        }

        if i.full_match {
            pkg_info_line.push(' ');
            pkg_info_line.push_str(
                &style(format!("[{}]", fl!("full-match")))
                    .yellow()
                    .bold()
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
        writeln!(
            writer,
            "{}{}",
            gen_prefix("", 10),
            style(i.desc).color256(182)
        )
        .ok();
    }

    drop(writer);
    pager.wait_for_exit()?;

    Ok(0)
}
