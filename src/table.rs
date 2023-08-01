use crate::console::{style, Color};
use crate::fl;
use anyhow::Result;
use oma_console::indicatif::HumanBytes;
use oma_console::pager::Pager;
use oma_pm::operation::{InstallEntry, InstallOperation, RemoveEntry};
use tabled::settings::object::{Columns, Segment};
use tabled::settings::{Alignment, Format, Modify, Style};
use tabled::{Table, Tabled};

#[derive(Debug, Tabled)]
struct InstallEntryDisplay {
    #[tabled(rename = "Name")]
    name: String,
    #[tabled(rename = "Version")]
    version_delta: String,
    #[tabled(rename = "Installed Size")]
    size_delta: String,
}

#[derive(Debug, Tabled)]
struct RemoveEntryDisplay {
    #[tabled(rename = "Name")]
    name: String,
    #[tabled(rename = "Details")]
    detail: String,
}

impl From<RemoveEntry> for RemoveEntryDisplay {
    fn from(value: RemoveEntry) -> Self {
        let name = style(value.name()).red().bold().to_string();
        let mut detail = vec![];
        for i in value.details() {
            match i {
                oma_pm::operation::RemoveTag::Purge => detail.push(fl!("purge-file")),
                oma_pm::operation::RemoveTag::AutoRemove => {
                    detail.push(fl!("removed-as-unneed-dep"))
                }
            }
        }

        let mut detail = detail.join(&fl!("semicolon"));

        // 首字大小写转换
        detail.get_mut(0..1).map(|s| {
            s.make_ascii_uppercase();
            &*s
        });

        Self { name, detail }
    }
}

impl From<&InstallEntry> for InstallEntryDisplay {
    fn from(value: &InstallEntry) -> Self {
        let name = match value.op() {
            InstallOperation::Install => style(value.name()).green().to_string(),
            InstallOperation::ReInstall => style(value.name()).blue().to_string(),
            InstallOperation::Upgrade => style(value.name()).color256(87).to_string(),
            InstallOperation::Downgrade => style(value.name()).yellow().to_string(),
            InstallOperation::Download => value.name().to_string(),
            InstallOperation::Default => unreachable!(),
        };

        let version_delta = if let Some(old_version) = value.old_version() {
            format!("{old_version} -> {}", value.new_version())
        } else {
            value.new_version().to_string()
        };

        let size_delta = if let Some(old_size) = value.old_size() {
            value.new_size() as i64 - old_size as i64
        } else {
            value.new_size() as i64
        };

        let size_delta = if size_delta >= 0 {
            format!("+{}", HumanBytes(size_delta.unsigned_abs()))
        } else {
            format!("-{}", HumanBytes(size_delta.unsigned_abs()))
        };

        Self {
            name,
            version_delta,
            size_delta,
        }
    }
}

pub fn table_for_install_pending(
    install: Vec<InstallEntry>,
    remove: Vec<RemoveEntry>,
    disk_size: (&str, u64),
    pager: bool
) -> Result<()> {
    let has_x11 = std::env::var("DISPLAY");

    let tips = if has_x11.is_ok() {
        fl!("question-tips-with-x11")
    } else {
        fl!("question-tips")
    };

    let mut pager = Pager::new(!pager, &tips)?;
    let pager_name = pager.pager_name().to_owned();
    let mut out = pager.get_writer()?;

    if pager_name == Some("less") {
        let _ = writeln!(
            out,
            "{:<80}",
            style(fl!("pending-op")).bold().bg(Color::Color256(25))
        );
    }

    let _ = writeln!(out);
    let _ = writeln!(out, "{}\n", fl!("review-msg"));
    let _ = writeln!(
        out,
        "{}\n",
        fl!(
            "oma-may",
            a = style(fl!("install")).green().to_string(),
            b = style(fl!("remove")).red().to_string(),
            c = style(fl!("upgrade")).color256(87).to_string(),
            d = style(fl!("downgrade")).yellow().to_string(),
            e = style(fl!("reinstall")).blue().to_string()
        ),
    );

    let _ = writeln!(out);

    if pager_name == Some("less") {
        let has_x11 = std::env::var("DISPLAY");

        let line1 = format!("    {}", fl!("end-review"));
        let line2 = format!("    {}", fl!("cc-to-abort"));

        if has_x11.is_ok() {
            let line3 = format!("    {}\n\n", fl!("how-to-op-with-x"));

            writeln!(out, "{}", style(line1).bold()).ok();
            writeln!(out, "{}", style(line2).bold()).ok();
            writeln!(out, "{}", style(line3).bold()).ok();
        } else {
            let line3 = format!("    {}\n\n", fl!("how-to-op"));

            writeln!(out, "{}", style(line1).bold()).ok();
            writeln!(out, "{}", style(line2).bold()).ok();
            writeln!(out, "{}", style(line3).bold()).ok();
        }
    }

    if !remove.is_empty() {
        let _ = writeln!(
            out,
            "{} {}{}\n",
            fl!("count-pkg-has-desc", count = remove.len()),
            style(fl!("removed")).red().bold(),
            fl!("colon")
        );

        let remove_display = remove
            .into_iter()
            .map(RemoveEntryDisplay::from)
            .collect::<Vec<_>>();

        let mut table = Table::new(remove_display);

        table
            .with(Modify::new(Segment::all()).with(Alignment::left()))
            .with(Modify::new(Columns::new(2..3)).with(Alignment::left()))
            .with(Style::psql());

        let _ = writeln!(out, "{table}\n\n");
    }

    let total_download_size: u64 = install
        .iter()
        .filter(|x| x.op() == &InstallOperation::Install || x.op() == &InstallOperation::Upgrade)
        .map(|x| x.download_size())
        .sum();

    if !install.is_empty() {
        let install_e = install
            .iter()
            .filter(|x| x.op() == &InstallOperation::Install);

        let install_e_display = install_e
            .map(InstallEntryDisplay::from)
            .collect::<Vec<_>>();

        if !install_e_display.is_empty() {
            let _ = writeln!(
                out,
                "{} {}{}\n",
                fl!("count-pkg-has-desc", count = install_e_display.len()),
                style(fl!("installed")).green().bold(),
                fl!("colon")
            );

            let mut table = Table::new(&install_e_display);

            table
                .with(Modify::new(Segment::all()).with(Alignment::left()))
                // Install Size column should align right
                .with(Modify::new(Columns::new(2..3)).with(Alignment::right()))
                .with(Style::psql())
                .with(Modify::new(Segment::all()).with(Format::content(|s| format!(" {s} "))));

            writeln!(out, "{table}\n\n").ok();
        }

        let update = install
            .iter()
            .filter(|x| x.op() == &InstallOperation::Upgrade);

        let update_display = update
            .map(InstallEntryDisplay::from)
            .collect::<Vec<_>>();

        if !update_display.is_empty() {
            let _ = writeln!(
                out,
                "{} {}{}\n",
                fl!("count-pkg-has-desc", count = update_display.len()),
                style(fl!("upgrade")).color256(87),
                fl!("colon")
            );

            let mut table = Table::new(&update_display);

            table
                .with(Modify::new(Segment::all()).with(Alignment::left()))
                // Install Size column should align right
                .with(Modify::new(Columns::new(2..3)).with(Alignment::right()))
                .with(Style::psql())
                .with(Modify::new(Segment::all()).with(Format::content(|s| format!(" {s} "))));

            writeln!(out, "{table}\n\n").ok();
        }

        let downgrade = install
            .iter()
            .filter(|x| x.op() == &InstallOperation::Downgrade);

        let downgrade_display = downgrade
            .map(InstallEntryDisplay::from)
            .collect::<Vec<_>>();

        if !downgrade_display.is_empty() {
            let _ = writeln!(
                out,
                "{} {}{}\n",
                fl!("count-pkg-has-desc", count = downgrade_display.len()),
                style(fl!("downgraded")).yellow().bold(),
                fl!("colon")
            );

            let mut table = Table::new(downgrade_display);

            table
                .with(Modify::new(Segment::all()).with(Alignment::left()))
                // Install Size column should align right
                .with(Modify::new(Columns::new(1..2)).with(Alignment::right()))
                .with(Style::psql())
                .with(Modify::new(Segment::all()).with(Format::content(|s| format!(" {s} "))));

            writeln!(out, "{table}\n\n").ok();
        }

        let reinstall = install
            .iter()
            .filter(|x| x.op() == &InstallOperation::ReInstall);

        let reinstall_display = reinstall
            .map(InstallEntryDisplay::from)
            .collect::<Vec<_>>();

        if !reinstall_display.is_empty() {
            let _ = writeln!(
                out,
                "{} {}{}\n",
                fl!("count-pkg-has-desc", count = reinstall_display.len()),
                style(fl!("reinstall")).blue().bold(),
                fl!("colon")
            );

            let mut table = Table::new(reinstall_display);

            table
                .with(Modify::new(Segment::all()).with(Alignment::left()))
                // Install Size column should align right
                .with(Modify::new(Columns::new(2..3)).with(Alignment::right()))
                .with(Style::psql())
                .with(Modify::new(Segment::all()).with(Format::content(|s| format!(" {s} "))));

            writeln!(out, "{table}\n\n").ok();
        }
    }

    writeln!(
        out,
        "{}{}",
        style(fl!("total-download-size")).bold(),
        HumanBytes(total_download_size)
    )
    .ok();

    let (symbol, abs_install_size_change) = disk_size;

    writeln!(
        out,
        "{}{}{}",
        style(fl!("change-storage-usage")).bold(),
        symbol,
        HumanBytes(abs_install_size_change)
    )
    .ok();

    drop(out);

    pager.wait_for_exit()?;

    Ok(())
}
