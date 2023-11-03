use oma_history::{connect_or_create_db, list_history, SummaryType};
use oma_pm::{
    apt::{AptArgsBuilder, FilterMode, OmaApt, OmaAptArgsBuilder},
    operation::InstallOperation,
    pkginfo::PkgInfo,
};

use crate::{
    error::OutputError,
    utils::{create_async_runtime, dbus_check, root},
};

use super::utils::{
    dialoguer_select_history, format_summary_log, handle_no_result, normal_commit, NormalCommitArgs,
};

pub fn execute(
    network_thread: usize,
    no_progress: bool,
    sysroot: String,
) -> Result<i32, OutputError> {
    root()?;

    let rt = create_async_runtime()?;
    dbus_check(&rt)?;

    let conn = connect_or_create_db(false, sysroot.clone())?;
    let list = list_history(conn)?;
    let display_list = format_summary_log(&list, true);
    let selected = dialoguer_select_history(&display_list, 0)?;

    let selected = &list[selected].0;
    let op = &selected.op;

    let oma_apt_args = OmaAptArgsBuilder::default()
        .sysroot(sysroot.clone())
        .build()?;
    let mut apt = OmaApt::new(vec![], oma_apt_args, false)?;

    let mut delete = vec![];
    let mut install = vec![];

    if !op.install.is_empty() {
        for i in &op.install {
            match i.op() {
                InstallOperation::Default | InstallOperation::Download => unreachable!(),
                InstallOperation::Install => delete.push(i.name()),
                InstallOperation::ReInstall => continue,
                InstallOperation::Upgrade => install.push((i.name(), i.old_version().unwrap())),
                InstallOperation::Downgrade => install.push((i.name(), i.old_version().unwrap())),
            }
        }
    }

    if !op.remove.is_empty() {
        for i in &op.remove {
            if let Some(ver) = i.version() {
                install.push((i.name(), ver));
            }
        }
    }

    let (delete, no_result) = apt.select_pkg(&delete, false, true, false)?;
    handle_no_result(no_result);

    apt.remove(&delete, false, true, |_| true)?;

    let pkgs = apt.filter_pkgs(&[FilterMode::Default])?.collect::<Vec<_>>();

    let install = install
        .iter()
        .filter_map(|(pkg, ver)| {
            let pkg = pkgs.iter().find(move |y| &y.name() == pkg);

            if let Some(pkg) = pkg {
                Some((pkg, pkg.get_version(ver)?))
            } else {
                None
            }
        })
        .map(|(x, y)| PkgInfo::new(&y, x))
        .collect::<Vec<_>>();

    apt.install(&install, false)?;

    let args = NormalCommitArgs {
        apt,
        dry_run: false,
        typ: SummaryType::Undo,
        apt_args: AptArgsBuilder::default().no_progress(no_progress).build()?,
        no_fixbroken: false,
        network_thread,
        no_progress,
        sysroot,
    };

    normal_commit(args)?;

    Ok(0)
}
