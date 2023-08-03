use std::{
    borrow::Cow,
    path::{Path, PathBuf},
};

use anyhow::{anyhow, Result};
use dialoguer::{console::style, theme::ColorfulTheme, Select};
use oma_console::{
    error, indicatif::ProgressBar, info, pb::oma_spinner, success, warn, writer::gen_prefix,
};
use oma_contents::QueryMode;
use oma_pm::{
    apt::{AptArgs, AptArgsBuilder, FilterMode, OmaApt, OmaAptArgsBuilder, OmaAptError},
    pkginfo::PkgInfo,
    query::OmaDatabase,
    PackageStatus,
};
use oma_refresh::db::OmaRefresh;
use oma_utils::dpkg_arch;

use crate::{
    fl,
    table::{handle_resolve, oma_display, table_for_install_pending},
    InstallArgs, RemoveArgs, UpgradeArgs,
};

pub fn install(pkgs_unparse: Vec<String>, args: InstallArgs) -> Result<i32> {
    if !args.no_refresh {
        refresh()?;
    }

    let local_debs = pkgs_unparse
        .iter()
        .filter(|x| x.ends_with(".deb"))
        .map(|x| x.to_owned())
        .collect::<Vec<_>>();

    let pkgs_unparse = pkgs_unparse.iter().map(|x| x.as_str()).collect::<Vec<_>>();

    let oma_apt_args = OmaAptArgsBuilder::default()
        .install_recommends(args.install_recommends)
        .install_suggests(args.install_suggests)
        .no_install_recommends(args.no_install_recommends)
        .no_install_suggests(args.no_install_suggests)
        .build()?;

    let apt = OmaApt::new(local_debs, oma_apt_args)?;
    let pkgs = apt.select_pkg(pkgs_unparse, args.install_dbg, true)?;

    apt.install(pkgs, args.reinstall)?;

    let apt_args = AptArgsBuilder::default()
        .yes(args.yes)
        .force_yes(args.force_yes)
        .dpkg_force_all(args.dpkg_force_all)
        .dpkg_force_confnew(args.force_confnew)
        .build()?;

    let op = apt.operation_vec()?;
    let install = op.install;
    let remove = op.remove;
    let disk_size = op.disk_size;

    if install.is_empty() && remove.is_empty() {
        return Ok(0);
    }

    handle_resolve(&apt, args.no_fixbroken)?;
    apt.check_disk_size()?;
    table_for_install_pending(install, remove, disk_size, !args.yes)?;
    apt.commit(None, &apt_args)?;

    Ok(0)
}

pub fn upgrade(pkgs_unparse: Vec<String>, args: UpgradeArgs) -> Result<i32> {
    refresh()?;

    let local_debs = pkgs_unparse
        .iter()
        .filter(|x| x.ends_with(".deb"))
        .map(|x| x.to_owned())
        .collect::<Vec<_>>();

    let pkgs_unparse = pkgs_unparse.iter().map(|x| x.as_str()).collect::<Vec<_>>();
    let mut retry_times = 1;

    let apt_args = AptArgsBuilder::default()
        .dpkg_force_all(args.dpkg_force_all)
        .dpkg_force_confnew(args.force_confnew)
        .force_yes(args.force_yes)
        .yes(args.yes)
        .build()?;

    loop {
        let oma_apt_args = OmaAptArgsBuilder::default().build()?;
        let apt = OmaApt::new(local_debs.clone(), oma_apt_args)?;

        let pkgs = apt.select_pkg(pkgs_unparse.clone(), false, true)?;

        apt.upgrade()?;

        apt.install(pkgs, false)?;

        let op = apt.operation_vec()?;
        let install = op.install;
        let remove = op.remove;
        let disk_size = op.disk_size;

        if install.is_empty() && remove.is_empty() {
            success!("{}", fl!("successfully-refresh"));
            return Ok(0);
        }

        if retry_times == 1 {
            table_for_install_pending(install, remove, disk_size, !args.yes)?;
        }

        handle_resolve(&apt, false)?;
        apt.check_disk_size()?;
        match apt.commit(None, &apt_args) {
            Ok(_) => break,
            Err(e) => match e {
                OmaAptError::RustApt(_) => {
                    if retry_times == 3 {
                        return Err(anyhow!("{e}"));
                    }
                    warn!("{e}, retrying ...");
                    retry_times += 1;
                }
                _ => return Err(anyhow!("{e}")),
            },
        }
    }

    Ok(0)
}

pub fn remove(pkgs: Vec<&str>, args: RemoveArgs) -> Result<i32> {
    let oma_apt_args = OmaAptArgsBuilder::default().build()?;
    let mut apt = OmaApt::new(vec![], oma_apt_args)?;
    let pkgs = apt.select_pkg(pkgs, false, true)?;

    apt.remove(pkgs, !args.keep_config, true, true, args.no_autoremove)?;

    let op = apt.operation_vec()?;
    let install = op.install;
    let remove = op.remove;
    let disk_size = op.disk_size;

    if install.is_empty() && remove.is_empty() {
        return Ok(0);
    }

    let apt_args = AptArgsBuilder::default()
        .yes(args.yes)
        .force_yes(args.force_yes)
        .build()?;

    handle_resolve(&apt, false)?;
    apt.check_disk_size()?;
    table_for_install_pending(install, remove, disk_size, !args.yes)?;
    apt.commit(None, &apt_args)?;

    Ok(0)
}

pub fn download(keyword: Vec<&str>, path: Option<PathBuf>) -> Result<i32> {
    let oma_apt_args = OmaAptArgsBuilder::default().build()?;
    let apt = OmaApt::new(vec![], oma_apt_args)?;
    let pkgs = apt.select_pkg(keyword, false, true)?;

    apt.download(pkgs, None, path.as_deref())?;

    Ok(0)
}

pub fn search(args: &[String]) -> Result<i32> {
    let oma_apt_args = OmaAptArgsBuilder::default().build()?;
    let apt = OmaApt::new(vec![], oma_apt_args)?;
    let db = OmaDatabase::new(&apt.cache)?;
    let s = args.join(" ");
    let res = db.search(&s)?;
    let mut pager = oma_display(false, res.len())?;

    let mut writer = pager.get_writer()?;

    for i in res {
        let mut pkg_info_line = if i.is_base {
            style(&i.name).bold().blue().to_string()
        } else {
            style(&i.name).bold().to_string()
        };

        pkg_info_line.push(' ');

        if i.status == PackageStatus::Upgrade {
            pkg_info_line.push_str(&format!(
                "{} -> {}",
                style(i.old_version.unwrap()).yellow(),
                style(&i.new_version).green()
            ));
        } else {
            pkg_info_line.push_str(&style(&i.new_version).green().to_string());
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
            PackageStatus::Installed => style(fl!("pkg-search-installed")).green(),
            PackageStatus::Upgrade => style(fl!("pkg-search-upgrade")).yellow(),
        }
        .to_string();

        writeln!(writer, "{}{}", gen_prefix(&prefix, 10), pkg_info_line).ok();
        writeln!(writer, "{}{}", gen_prefix("", 10), i.desc).ok();
    }

    drop(writer);
    pager.wait_for_exit()?;

    Ok(0)
}

pub fn command_refresh() -> Result<i32> {
    refresh()?;

    let oma_apt_args = OmaAptArgsBuilder::default().build()?;
    let apt = OmaApt::new(vec![], oma_apt_args)?;
    let (upgradable, removable) = apt.available_action()?;
    let mut s = vec![];

    if upgradable != 0 {
        s.push(fl!("packages-can-be-upgrade", len = upgradable));
    }

    if removable != 0 {
        s.push(fl!("packages-can-be-removed", len = removable));
    }

    if s.is_empty() {
        success!("{}", fl!("successfully-refresh"));
    } else {
        let mut s = s.join(&fl!("comma"));
        s = s + &fl!("full-comma");
        success!("{}", fl!("successfully-refresh-with-tips", s = s));
    }
    Ok(0)
}

fn refresh() -> Result<()> {
    info!("{}", fl!("refreshing-repo-metadata"));
    let refresh = OmaRefresh::scan(None)?;
    let tokio = tokio::runtime::Builder::new_multi_thread()
        .enable_io()
        .enable_time()
        .build()?;

    tokio.block_on(async move { refresh.start().await })?;

    Ok(())
}

pub fn show(all: bool, pkgs_unparse: Vec<&str>) -> Result<i32> {
    let oma_apt_args = OmaAptArgsBuilder::default().build()?;
    let apt = OmaApt::new(vec![], oma_apt_args)?;
    let pkg = apt.select_pkg(pkgs_unparse, false, false)?;
    for (i, c) in pkg.iter().enumerate() {
        if c.is_candidate || all {
            if i != pkg.len() - 1 {
                println!("{c}\n");
            } else {
                println!("{c}");
            }
        }
    }
    if !all {
        let other_version = pkg
            .iter()
            .filter(|x| !x.is_candidate)
            .collect::<Vec<_>>()
            .len();

        if other_version > 0 {
            info!("{}", fl!("additional-version", len = other_version));
        }
    }

    Ok(0)
}

pub fn find(x: &str, is_bin: bool, pkg: String) -> Result<i32> {
    let pb = ProgressBar::new_spinner();
    let (style, inv) = oma_spinner(false)?;
    pb.set_style(style);
    pb.enable_steady_tick(inv);
    pb.set_message(fl!("searching"));

    let query_mode = match x {
        "files" => QueryMode::ListFiles(is_bin),
        "provides" => QueryMode::Provides(is_bin),
        _ => unreachable!(),
    };

    let res = oma_contents::find(
        &pkg,
        query_mode,
        Path::new("/var/lib/apt/lists"),
        &dpkg_arch()?,
        |c| {
            pb.set_message(fl!("search-with-result-count", count = c));
        },
    )?;

    pb.finish_and_clear();
    let mut pager = oma_display(false, res.len())?;
    let mut out = pager.get_writer()?;

    for (_, v) in res {
        writeln!(out, "{v}").ok();
    }

    drop(out);
    pager.wait_for_exit()?;

    Ok(0)
}

pub fn depends(pkgs: Vec<String>) -> Result<i32> {
    let oma_apt_args = OmaAptArgsBuilder::default().build()?;
    let apt = OmaApt::new(vec![], oma_apt_args)?;

    let pkgs = apt.select_pkg(
        pkgs.iter().map(|x| x.as_str()).collect::<Vec<_>>(),
        false,
        true,
    )?;

    for pkg in pkgs {
        println!("{}:", pkg.raw_pkg.name());
        let all_deps = pkg.deps;

        for (k, v) in all_deps {
            for dep in v.inner() {
                for b_dep in dep {
                    let s = if let Some(comp_ver) = b_dep.comp_ver {
                        Cow::Owned(format!("({comp_ver})"))
                    } else {
                        Cow::Borrowed("")
                    };

                    println!("  {k}: {} {}", b_dep.name, s);
                }
            }
        }
    }

    Ok(0)
}

pub fn pick(pkg_str: String, no_refresh: bool) -> Result<i32> {
    if !no_refresh {
        refresh()?;
    }

    let oma_apt_args = OmaAptArgsBuilder::default().build()?;
    let apt = OmaApt::new(vec![], oma_apt_args)?;
    let pkg = apt
        .cache
        .get(&pkg_str)
        .ok_or_else(|| anyhow!(fl!("can-not-get-pkg-from-database", name = pkg_str.clone())))?;

    let versions = pkg.versions().collect::<Vec<_>>();
    let versions_str = versions
        .iter()
        .map(|x| x.version().to_string())
        .collect::<Vec<_>>();

    let mut v = vec![];
    for i in 0..versions.len() {
        for j in 1..versions.len() {
            if i == j {
                continue;
            }

            if versions_str[i] == versions_str[j] {
                v.push((i, j));
            }
        }
    }

    let mut version_str_display = versions_str.clone();
    for (a, b) in v {
        let uri_a = versions[a].uris().next().unwrap();
        version_str_display[a] = format!("{} (from: {})", versions_str[a], uri_a);

        let uri_b = versions[b].uris().next().unwrap();
        version_str_display[b] = format!("{} (from: {})", versions_str[b], uri_b);
    }

    let theme = ColorfulTheme::default();
    let mut dialoguer = Select::with_theme(&theme);
    dialoguer.items(&versions_str);
    dialoguer.with_prompt(fl!("pick-tips", pkgname = pkg.name()));

    let pos = if let Some(installed) = pkg.installed() {
        versions_str
            .iter()
            .position(|x| x == installed.version())
            .unwrap_or(0)
    } else {
        0
    };

    dialoguer.default(pos);
    let sel = dialoguer.interact()?;
    let version = pkg.get_version(&versions_str[sel]).ok_or_else(|| {
        anyhow!(fl!(
            "can-not-get-pkg-version-from-database",
            name = pkg_str,
            version = versions_str[sel].clone()
        ))
    })?;

    let pkginfo = PkgInfo::new(&apt.cache, version.unique(), &pkg);

    apt.install(vec![pkginfo], false)?;

    let op = apt.operation_vec()?;
    let install = op.install;
    let remove = op.remove;
    let disk_size = op.disk_size;

    handle_resolve(&apt, false)?;
    apt.check_disk_size()?;
    table_for_install_pending(install, remove, disk_size, true)?;
    apt.commit(None, &AptArgsBuilder::default().build()?)?;

    Ok(0)
}

pub fn fix_broken() -> Result<i32> {
    let oma_apt_args = OmaAptArgsBuilder::default().build()?;
    let apt = OmaApt::new(vec![], oma_apt_args)?;
    handle_resolve(&apt, false)?;
    apt.commit(None, &AptArgs::default())?;

    Ok(0)
}

pub fn command_not_found(pkg: String) -> Result<i32> {
    let res = oma_contents::find(
        &pkg,
        QueryMode::CommandNotFound,
        Path::new("/var/lib/apt/lists"),
        &dpkg_arch()?,
        |_| {},
    );
    match res {
        Ok(res) if res.is_empty() => {
            error!("{}", fl!("command-not-found", kw = pkg));
        }
        Ok(res) => {
            println!("{}\n", fl!("command-not-found-with-result", kw = pkg));

            let oma_apt_args = OmaAptArgsBuilder::default().build()?;
            let apt = OmaApt::new(vec![], oma_apt_args)?;

            for (k, v) in res {
                let (pkg, bin_path) = v.split_once(':').unwrap();
                let bin_path = bin_path.trim();
                let pkg = apt.cache.get(pkg);

                let desc = pkg
                    .unwrap()
                    .candidate()
                    .and_then(|x| x.description())
                    .unwrap();

                println!("{k} ({bin_path}): {desc}");
            }
        }
        Err(e) => {
            // TODO: match error to translate
            error!("{e}");
            error!("{}", fl!("command-not-found", kw = pkg));
        }
    }

    Ok(127)
}

pub fn list(all: bool, installed: bool, upgradable: bool, pkgs: Vec<String>) -> Result<i32> {
    let oma_apt_args = OmaAptArgsBuilder::default().build()?;
    let apt = OmaApt::new(vec![], oma_apt_args)?;

    let mut filter_mode = vec![];
    if installed {
        filter_mode.push(FilterMode::Installed);
    }
    if upgradable {
        filter_mode.push(FilterMode::Upgradable)
    }

    let filter_pkgs = apt.filter_pkgs(&filter_mode)?;
    let filter_pkgs: Box<dyn Iterator<Item = _>> = if pkgs.is_empty() {
        Box::new(filter_pkgs)
    } else {
        Box::new(filter_pkgs.filter(|x| pkgs.contains(&x.name().to_string())))
    };

    for pkg in filter_pkgs {
        let name = pkg.name();
        let versions = if all {
            pkg.versions().collect()
        } else {
            vec![pkg
                .candidate()
                .ok_or_else(|| anyhow!(fl!("no-candidate-ver", pkg = name)))?]
        };

        for version in &versions {
            let uris = version.uris().collect::<Vec<_>>();
            let mut branches = vec![];
            for uri in uris.iter() {
                let mut branch = uri.split('/');
                let branch = branch.nth_back(3).unwrap_or("");
                branches.push(branch);
            }
            let branches = branches.join(",");
            let version_str = version.version();
            let arch = pkg.arch();
            let installed = pkg.installed().as_ref() == Some(version);
            let upgradable = pkg.is_upgradable();
            let automatic = pkg.is_auto_installed();

            let mut s = vec![];

            if installed {
                s.push("installed");
            }

            if upgradable {
                s.push("upgradable");
            }

            if automatic {
                s.push("automatc");
            }

            let s = if s.is_empty() {
                Cow::Borrowed("")
            } else {
                Cow::Owned(format!("[{}]", s.join(",")))
            };

            println!(
                "{}/{branches} {version_str} {arch} {s}",
                style(name).green().bold()
            );
        }
    }

    Ok(0)
}

pub fn clean() -> Result<i32> {
    let oma_apt_args = OmaAptArgsBuilder::default().build()?;
    let apt = OmaApt::new(vec![], oma_apt_args)?;
    let download_dir = apt.get_archive_dir();
    let dir = std::fs::read_dir(&download_dir)?;

    for i in dir.flatten() {
        if i.path().extension().and_then(|x| x.to_str()) == Some("deb") {
            std::fs::remove_file(i.path()).ok();
        }
    }

    let p = download_dir.join("..");
    std::fs::remove_file(p.join("pkgcache.bin")).ok();
    std::fs::remove_file(p.join("srcpkgcache.bin")).ok();

    success!("{}", fl!("clean-successfully"));

    Ok(0)
}

pub fn pkgnames(keyword: Option<String>) -> Result<i32> {
    let oma_apt_args = OmaAptArgsBuilder::default().build()?;
    let apt = OmaApt::new(vec![], oma_apt_args)?;
    let mut pkgs: Box<dyn Iterator<Item = _>> = Box::new(apt.filter_pkgs(&[FilterMode::Names])?);

    if let Some(keyword) = keyword {
        pkgs = Box::new(pkgs.filter(move |x| x.name().starts_with(&keyword)));
    }

    for pkg in pkgs {
        println!("{}", pkg.name());
    }

    Ok(0)
}
