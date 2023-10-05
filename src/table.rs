use std::fmt::Display;
use std::io::Write;
use std::sync::atomic::Ordering;

use crate::console::{style, Color};
use crate::error::OutputError;
use crate::{fl, ALLOWCTRLC};
use oma_console::indicatif::HumanBytes;
use oma_console::pager::Pager;
use oma_console::WRITER;
use oma_pm::operation::{InstallEntry, InstallOperation, RemoveEntry, RemoveTag};
use oma_pm::unmet::{UnmetDep, WhyUnmet};
use tabled::settings::object::{Columns, Segment};
use tabled::settings::{Alignment, Format, Modify, Style};
use tabled::{Table, Tabled};

#[derive(Debug, Tabled)]
struct InstallEntryDisplay {
    name: String,
    version_delta: String,
    size_delta: String,
}

#[derive(Debug, Tabled)]
struct RemoveEntryDisplay {
    name: String,
    size: String,
    detail: String,
}

#[derive(Debug, Tabled)]
pub struct UnmetDepDisplay {
    #[tabled(rename = "Package")]
    package: String,
    #[tabled(rename = "Unmet Dependency")]
    unmet_dependency: String,
    #[tabled(rename = "Specified Dependency")]
    specified_dependency: String,
}

impl From<&UnmetDep> for UnmetDepDisplay {
    fn from(value: &UnmetDep) -> Self {
        Self {
            package: style(&value.package).red().bold().to_string(),
            unmet_dependency: match &value.unmet_dependency {
                WhyUnmet::DepNotExist(s) => format!("{s} does not exist"),
                WhyUnmet::Unmet {
                    dep_name,
                    need_ver,
                    symbol,
                } => format!("{dep_name} {symbol} {need_ver}"),
                WhyUnmet::Breaks {
                    break_type,
                    dep_name,
                    comp_ver,
                } => {
                    let comp_ver = comp_ver.as_deref().unwrap_or("");

                    format!("{break_type} {dep_name} {comp_ver}")
                }
            },
            specified_dependency: value.specified_dependency.clone(),
        }
    }
}

impl From<&RemoveEntry> for RemoveEntryDisplay {
    fn from(value: &RemoveEntry) -> Self {
        let name = style(value.name()).red().bold().to_string();
        let mut detail = vec![];
        for i in value.details() {
            match i {
                RemoveTag::Purge => detail.push(fl!("purge-file")),
                RemoveTag::AutoRemove => detail.push(fl!("removed-as-unneed-dep")),
            }
        }

        let mut detail = detail.join(&fl!("semicolon"));

        // 首字大小写转换
        detail.get_mut(0..1).map(|s| {
            s.make_ascii_uppercase();
            &*s
        });

        let size = format!("-{}", HumanBytes(value.size()));

        Self { name, detail, size }
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

pub fn oma_display_with_normal_output(is_question: bool, len: usize) -> Result<Pager, OutputError> {
    if !is_question {
        ALLOWCTRLC.store(true, Ordering::Relaxed);
    }

    let tips = less_tips(is_question);

    let pager = if len < WRITER.get_height().into() {
        Pager::plain()
    } else {
        Pager::external(tips)?
    };

    Ok(pager)
}

fn less_tips(is_question: bool) -> String {
    let has_x11 = std::env::var("DISPLAY");

    if is_question {
        if has_x11.is_ok() {
            fl!("question-tips-with-x11")
        } else {
            fl!("question-tips")
        }
    } else if has_x11.is_ok() {
        fl!("normal-tips-with-x11")
    } else {
        fl!("normal-tips")
    }
}

pub struct PagerPrinter<W> {
    writer: W,
}

impl<W: Write> PagerPrinter<W> {
    pub fn new(writer: W) -> PagerPrinter<W> {
        PagerPrinter { writer }
    }

    pub fn print<D: Display>(&mut self, d: D) -> std::io::Result<()> {
        writeln!(self.writer, "{d}")
    }

    pub fn print_table<T, I>(&mut self, table: I, header: Option<Vec<&str>>) -> std::io::Result<()>
    where
        I: IntoIterator<Item = T>,
        T: Tabled,
    {
        let mut table = match header {
            Some(h) => {
                let mut t = Table::builder(table);
                t.set_header(h);
                t.build()
            }
            None => Table::new(table),
        };

        table
            .with(Modify::new(Segment::all()).with(Alignment::left()))
            .with(Modify::new(Columns::new(2..3)).with(Alignment::left()))
            .with(Style::psql())
            .with(Modify::new(Segment::all()).with(Format::content(|s| format!(" {s} "))));

        writeln!(self.writer, "{table}")
    }
}

pub fn print_unmet_dep(u: &[UnmetDep]) -> Result<(), OutputError> {
    let tips = less_tips(false);
    let mut pager = Pager::external(tips)?;
    let out = pager.get_writer().unwrap();
    let mut printer = PagerPrinter::new(out);

    printer
        .print(format!("{:<80}\n", style(fl!("dep-error")).on_red().bold()))
        .ok();
    printer.print(format!("{}\n", fl!("dep-error-desc"))).ok();
    printer
        .print(format!("{}\n", fl!("contact-admin-tips")))
        .ok();
    printer
        .print(format!("    {}", style(fl!("how-to-abort")).bold()))
        .ok();
    printer
        .print(format!("    {}\n\n", style(fl!("how-to-op-with-x")).bold()))
        .ok();

    let v = u.iter().map(UnmetDepDisplay::from).collect::<Vec<_>>();

    printer
        .print(format!(
            "{} {}{}\n",
            fl!("unmet-dep-before", count = v.len()),
            style(fl!("unmet-dep")).red().bold(),
            fl!("colon")
        ))
        .ok();

    printer.print_table(v, None).ok();
    printer.print("\n").ok();

    drop(printer);
    pager.wait_for_exit()?;

    Ok(())
}

pub fn table_for_install_pending(
    install: &[InstallEntry],
    remove: &[RemoveEntry],
    disk_size: &(String, u64),
    is_pager: bool,
    dry_run: bool,
) -> Result<(), OutputError> {
    if dry_run {
        return Ok(());
    }

    let tips = less_tips(true);

    let mut pager = if is_pager {
        Pager::external(tips)?
    } else {
        Pager::plain()
    };

    let pager_name = pager.pager_name().to_owned();
    let out = pager.get_writer()?;
    let mut printer = PagerPrinter::new(out);

    if is_pager {
        review_msg(&mut printer, pager_name);
    }

    print_pending_inner(printer, remove, install, disk_size);
    let success = pager.wait_for_exit()?;

    if is_pager && success {
        let pager = Pager::plain();
        let out = pager.get_writer()?;
        let mut printer = PagerPrinter::new(out);
        printer.print("").ok();
        print_pending_inner(printer, remove, install, disk_size);
    }

    Ok(())
}

pub fn table_for_history_pending(
    install: &[InstallEntry],
    remove: &[RemoveEntry],
    disk_size: &(String, u64),
) -> Result<(), OutputError> {
    let tips = less_tips(false);

    let mut pager = Pager::external(tips)?;

    let out = pager.get_writer()?;
    let printer = PagerPrinter::new(out);

    print_pending_inner(printer, remove, install, disk_size);
    pager.wait_for_exit()?;

    Ok(())
}

fn print_pending_inner<W: Write>(
    mut printer: PagerPrinter<W>,
    remove: &[RemoveEntry],
    install: &[InstallEntry],
    disk_size: &(String, u64),
) {
    if !remove.is_empty() {
        printer
            .print(format!(
                "{} {}{}\n",
                fl!("count-pkg-has-desc", count = remove.len()),
                style(fl!("removed")).red().bold(),
                fl!("colon")
            ))
            .ok();

        let remove_display = remove
            .iter()
            .map(RemoveEntryDisplay::from)
            .collect::<Vec<_>>();

        printer
            .print_table(remove_display, Some(vec!["Name", "Size", "Detail"]))
            .ok();
        printer.print("\n").ok();
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

        let install_e_display = install_e.map(InstallEntryDisplay::from).collect::<Vec<_>>();

        if !install_e_display.is_empty() {
            printer
                .print(format!(
                    "{} {}{}\n",
                    fl!("count-pkg-has-desc", count = install_e_display.len()),
                    style(fl!("installed")).green().bold(),
                    fl!("colon")
                ))
                .ok();

            printer
                .print_table(
                    install_e_display,
                    Some(vec!["Name", "Version", "Installed size"]),
                )
                .ok();
            printer.print("\n").ok();
        }

        let update = install
            .iter()
            .filter(|x| x.op() == &InstallOperation::Upgrade);

        let update_display = update.map(InstallEntryDisplay::from).collect::<Vec<_>>();

        if !update_display.is_empty() {
            printer
                .print(format!(
                    "{} {}{}\n",
                    fl!("count-pkg-has-desc", count = update_display.len()),
                    style(fl!("upgrade")).color256(87),
                    fl!("colon")
                ))
                .ok();

            printer
                .print_table(
                    update_display,
                    Some(vec!["Name", "Version", "Installed size"]),
                )
                .ok();
            printer.print("\n").ok();
        }

        let downgrade = install
            .iter()
            .filter(|x| x.op() == &InstallOperation::Downgrade);

        let downgrade_display = downgrade.map(InstallEntryDisplay::from).collect::<Vec<_>>();

        if !downgrade_display.is_empty() {
            printer
                .print(format!(
                    "{} {}{}\n",
                    fl!("count-pkg-has-desc", count = downgrade_display.len()),
                    style(fl!("downgraded")).yellow().bold(),
                    fl!("colon")
                ))
                .ok();

            printer
                .print_table(
                    downgrade_display,
                    Some(vec!["Name", "Version", "Installed size"]),
                )
                .ok();
            printer.print("\n").ok();
        }

        let reinstall = install
            .iter()
            .filter(|x| x.op() == &InstallOperation::ReInstall);

        let reinstall_display = reinstall.map(InstallEntryDisplay::from).collect::<Vec<_>>();

        if !reinstall_display.is_empty() {
            printer
                .print(format!(
                    "{} {}{}\n",
                    fl!("count-pkg-has-desc", count = reinstall_display.len()),
                    style(fl!("reinstall")).blue().bold(),
                    fl!("colon"),
                ))
                .ok();

            printer
                .print_table(
                    reinstall_display,
                    Some(vec!["Name", "Version", "Installed size"]),
                )
                .ok();
            printer.print("\n").ok();
        }
    }

    printer
        .print(format!(
            "{}{}",
            style(fl!("total-download-size")).bold(),
            HumanBytes(total_download_size)
        ))
        .ok();

    let (symbol, abs_install_size_change) = disk_size;

    printer
        .print(format!(
            "{}{}{}",
            style(fl!("change-storage-usage")).bold(),
            symbol,
            HumanBytes(*abs_install_size_change)
        ))
        .ok();
    printer.print("").ok();
}

fn review_msg<W: Write>(printer: &mut PagerPrinter<W>, pager_name: Option<&str>) {
    if pager_name == Some("less") {
        printer
            .print(format!(
                "{:<80}",
                style(fl!("pending-op")).bold().bg(Color::Color256(25))
            ))
            .ok();
    }

    printer.print("").ok();
    printer.print(format!("{}\n", fl!("review-msg"))).ok();
    // let _ = writeln!(out, "{}\n", fl!("review-msg"));
    printer
        .print(format!(
            "{}\n",
            fl!(
                "oma-may",
                a = style(fl!("install")).green().to_string(),
                b = style(fl!("remove")).red().to_string(),
                c = style(fl!("upgrade")).color256(87).to_string(),
                d = style(fl!("downgrade")).yellow().to_string(),
                e = style(fl!("reinstall")).blue().to_string()
            )
        ))
        .ok();

    if pager_name == Some("less") {
        let has_x11 = std::env::var("DISPLAY");

        let line1 = format!("    {}", fl!("end-review"));
        let line2 = format!("    {}", fl!("cc-to-abort"));

        if has_x11.is_ok() {
            let line3 = format!("    {}\n\n", fl!("how-to-op-with-x"));

            printer.print(format!("{}", style(line1).bold())).ok();
            printer.print(format!("{}", style(line2).bold())).ok();
            printer.print(format!("{}", style(line3).bold())).ok();
        } else {
            let line3 = format!("    {}\n\n", fl!("how-to-op"));

            printer.print(format!("{}", style(line1).bold())).ok();
            printer.print(format!("{}", style(line2).bold())).ok();
            printer.print(format!("{}", style(line3).bold())).ok();
        }
    }
}
