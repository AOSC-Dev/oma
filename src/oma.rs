use anyhow::{anyhow, bail, Context, Result};
use console::style;
use dialoguer::{theme::ColorfulTheme, Select};
use either::Either;
use indicatif::{HumanBytes, ProgressBar};
use reqwest::Client;
use rust_apt::{
    cache::{Cache, PackageSort, Upgrade},
    config::Config,
    new_cache,
    package::{Package, Version},
    raw::{progress::AptInstallProgress, util::raw::apt_lock_inner},
    records::RecordField,
    util::{apt_lock, apt_unlock, apt_unlock_inner},
};
use std::{fmt::Write as FmtWrite, io::BufRead};
use tabled::Tabled;
use time::OffsetDateTime;
use tokio::runtime::Runtime;

use std::{
    fmt::Debug,
    io::Write,
    path::Path,
    process::{Command, Stdio},
    sync::atomic::Ordering,
};

use crate::{
    cli::{
        Download, FixBroken, InstallOptions, ListOptions, Mark, MarkAction, PickOptions,
        RemoveOptions, UpgradeOptions,
    },
    contents::find,
    db::{get_sources, update_db_runner, DOWNLOAD_DIR},
    download::{oma_spinner, packages_download_runner},
    error,
    formatter::{
        display_result, download_size, find_unmet_deps, find_unmet_deps_with_markinstall,
        NoProgress, OmaAptInstallProgress,
    },
    info,
    pager::Pager,
    pkg::{mark_delete, mark_install, query_pkgs, search_pkgs, PkgInfo},
    success,
    utils::{lock_oma, log_to_file, needs_root, size_checker},
    warn, ALLOWCTRLC, DRYRUN, MB, TIME_OFFSET, WRITER,
};

#[cfg(feature = "aosc")]
use crate::topics;

#[cfg(feature = "aosc")]
use crate::cli::Topics;

#[derive(Tabled, Debug, Clone)]
pub struct RemoveRow {
    #[tabled(rename = "Name")]
    name: String,
    #[tabled(skip)]
    _name_no_color: String,
    // #[tabled(skip)]
    // size: String,
    // Show details to this specific removal. Eg: if this is an essential package
    #[tabled(rename = "Details")]
    detail: String,
}

#[derive(Tabled, Debug, Clone)]
pub struct InstallRow {
    #[tabled(rename = "Name")]
    name: String,
    #[tabled(skip)]
    pub name_no_color: String,
    #[tabled(skip)]
    pub new_version: String,
    #[tabled(rename = "Version")]
    version: String,
    #[tabled(rename = "Installed Size")]
    pub size: String,
    #[tabled(skip)]
    pub pkg_urls: Vec<String>,
    #[tabled(skip)]
    pub checksum: Option<String>,
    #[tabled(skip)]
    pub pure_download_size: u64,
}

pub struct Oma {
    client: Client,
    runtime: Runtime,
}

#[derive(thiserror::Error, Debug)]
enum InstallError {
    #[error(transparent)]
    Anyhow(#[from] anyhow::Error),
    #[error(transparent)]
    RustApt(#[from] rust_apt::util::Exception),
}

type InstallResult<T> = std::result::Result<T, InstallError>;

impl Oma {
    pub fn build_async_runtime() -> Result<Self> {
        let client = reqwest::ClientBuilder::new().user_agent("oma").build()?;

        let runtime = tokio::runtime::Builder::new_multi_thread()
            .enable_all()
            .build()
            .expect("Can not init tokio runtime!");

        Ok(Self { client, runtime })
    }

    /// Update mirror database and Get all update, like apt update && apt full-upgrade
    pub fn update(&self, u: UpgradeOptions) -> Result<i32> {
        needs_root()?;
        lock_oma()?;

        if u.yes {
            yes_warn();
        }

        if u.dry_run {
            DRYRUN.store(true, Ordering::Relaxed);
        }

        update_db_runner(&self.runtime, &get_sources()?, &self.client, None)?;

        let start_time = OffsetDateTime::now_utc()
            .to_offset(*TIME_OFFSET)
            .to_string();

        fn update_inner(
            runtime: &Runtime,
            client: &Client,
            count: usize,
            u: &UpgradeOptions,
        ) -> InstallResult<Action> {
            let pkgs = u.packages.clone().unwrap_or_default();
            let cache = install_handle(&pkgs, false, false, false, false)?;

            // 检查一遍是否有依赖不存在的升级
            {
                tracing::debug!("Finding unmet upgrade");
                let pb = MB.add(ProgressBar::new_spinner());
                oma_spinner(&pb);
                pb.set_message("Quering upgradable packages ...");
                let sort = PackageSort::default().upgradable();
                let upgrable_pkgs = cache.packages(&sort);

                for pkg in upgrable_pkgs {
                    find_unmet_deps_with_markinstall(&cache, &pkg.candidate().unwrap(), false)?;
                }
                pb.finish_and_clear();
            }

            cache
                .upgrade(&Upgrade::FullUpgrade)
                .map_err(|e| anyhow!("{e}"))?;

            let (action, len, need_fix_system) =
                apt_handler(&cache, false, u.force_yes, false, u.no_autoremove)?;

            if len == 0 && !need_fix_system {
                success!("No need to do anything.");
                return Ok(action);
            }

            let mut list = action.update.clone();
            list.extend(action.install.clone());
            list.extend(action.downgrade.clone());

            if count == 1 {
                let disk_size = cache.depcache().disk_size();
                size_checker(&disk_size, download_size(&list, &cache)?)?;
                if len != 0 {
                    display_result(&action, &cache, u.yes)?;
                }
            }

            packages_download_runner(runtime, &list, client, None, None)?;
            apt_install(cache, u.yes, u.force_yes, u.force_confnew, u.dpkg_force_all)?;

            Ok(action)
        }

        let mut count = 1;

        loop {
            match update_inner(&self.runtime, &self.client, count, &u) {
                Err(e) => {
                    match e {
                        InstallError::Anyhow(e) => return Err(e),
                        InstallError::RustApt(e) => {
                            warn!("apt has retrn non-zero code, retrying {count} times");
                            // Retry 3 times, if Error is rust_apt return
                            if count == 3 {
                                return Err(e.into());
                            }
                            count += 1;
                        }
                    }
                }
                Ok(v) => {
                    let cache = new_cache!()?;
                    let end_time = OffsetDateTime::now_utc()
                        .to_offset(*TIME_OFFSET)
                        .to_string();

                    log_to_file(&v, &start_time, &end_time)?;

                    if u.dpkg_force_all && cache.depcache().broken_count() != 0 {
                        let e = anyhow!("Your system has broken dependencies");

                        return Err(
                            e.context(
                                format!("Try to use {} to fix broken dependencies\nIf this does not work, please contact upstream: https://github.com/aosc-dev/aosc-os-abbs",
                                style("oma fix-broken").green().bold()
                            ))
                        );
                    }

                    return Ok(0);
                }
            }
        }
    }

    pub fn install(&self, opt: InstallOptions) -> Result<i32> {
        needs_root()?;
        lock_oma()?;

        if opt.dry_run {
            DRYRUN.store(true, Ordering::Relaxed);
        }

        let start_time = OffsetDateTime::now_utc()
            .to_offset(*TIME_OFFSET)
            .to_string();

        if opt.yes {
            yes_warn();
        }

        if !opt.no_refresh {
            update_db_runner(&self.runtime, &get_sources()?, &self.client, None)?;
        }

        let mut count = 1;
        loop {
            match self.install_inner(&opt, count) {
                Err(e) => {
                    match e {
                        InstallError::Anyhow(e) => return Err(e),
                        InstallError::RustApt(e) => {
                            // Retry 3 times, if Error is rust_apt return
                            if count == 3 {
                                return Err(e.into());
                            }
                            count += 1;
                        }
                    }
                }
                Ok(v) => {
                    let end_time = OffsetDateTime::now_utc()
                        .to_offset(*TIME_OFFSET)
                        .to_string();

                    log_to_file(&v, &start_time, &end_time)?;

                    return Ok(0);
                }
            }
        }
    }

    pub fn list(opt: &ListOptions) -> Result<i32> {
        let cache = new_cache!()?;

        let mut sort = PackageSort::default();

        if opt.installed {
            sort = sort.installed();
        }

        if opt.upgradable {
            sort = sort.upgradable();
        }

        let pkgs = match opt.packages {
            Some(ref v) => Either::Left(
                cache
                    .packages(&sort)
                    .filter(|x| v.contains(&x.name().to_string())),
            ),
            None => Either::Right(cache.packages(&sort)),
        };

        let mut query_pkgs = vec![];

        for pkg in pkgs {
            if opt.all {
                for ver in pkg.versions() {
                    query_pkgs.push((ver.unique(), pkg.unique()));
                }
            } else if let Some(cand) = pkg.candidate() {
                query_pkgs.push((cand.unique(), pkg.unique()));
            }
        }

        let result_len = query_pkgs.len();

        for (ver, pkg) in query_pkgs {
            let pkg = Package::new(&cache, pkg);
            let ver = Version::new(ver, &pkg);

            let mut stdout = std::io::stdout();

            let mut mirrors = vec![];

            for url in ver.uris() {
                let mut branch = url
                    .split('/')
                    .nth_back(3)
                    .unwrap_or_default()
                    .trim()
                    .to_owned();

                if branch.is_empty() {
                    branch = "unknown".to_owned();
                }
                mirrors.push(branch);
            }

            let mut status = vec![];

            if pkg.is_installed() {
                status.push("installed".to_owned());
            }

            if pkg.is_auto_installed() {
                status.push("automatic".to_owned());
            }

            if pkg.is_upgradable() {
                let cand = pkg.candidate().unwrap();
                status.push(format!("upgrade from {}", cand.version()));
            }

            let status = if status.is_empty() {
                "".to_string()
            } else {
                format!("[{}]", status.join(","))
            };

            writeln!(
                &mut stdout,
                "{}/{} {} {} {}",
                style(pkg.name()).green().bold(),
                mirrors.join(","),
                ver.version(),
                pkg.arch(),
                status
            )?;

            let len = pkg.versions().collect::<Vec<_>>().len();

            if !opt.all && len > 1 && result_len == 1 {
                info!(
                    "There is {} additional version. Please use the '-a' switch to see it",
                    len - 1
                );
            }
        }

        Ok(0)
    }

    pub fn show(list: &[String], is_all: bool) -> Result<i32> {
        let cache = new_cache!()?;

        let mut s = String::new();

        let mut len = 0;

        for (i, c) in list.iter().enumerate() {
            let oma_pkg = query_pkgs(&cache, c)?;
            len = oma_pkg.len();

            if len == 0 {
                warn!("Could not find any package for keyword {c}");
            }

            for (i, (entry, is_cand)) in oma_pkg.into_iter().enumerate() {
                if !is_all && !is_cand {
                    continue;
                }
                s += &format!("Package: {}\n", entry.package);
                s += &format!("Version: {}\n", entry.version);
                if let Some(section) = entry.section {
                    s += &format!("Section: {section}\n");
                }
                s += &format!("Maintainer: {}\n", entry.maintainer);
                s += &format!("Installed-Size: {}\n", entry.installed_size);
                for (k, v) in entry.dep_map {
                    s += &format!("{k}: {v}\n");
                }
                s += &format!("Download-Size: {}\n", entry.download_size);
                s += &format!("APT-Sources: {:?}\n", entry.apt_sources);
                if let Some(desc) = entry.description {
                    s += &format!("Description: {desc}\n");
                }

                if i < len - 1 {
                    s += "\n";
                }
            }

            if i < list.len() - 1 {
                s += "\n";
            }
        }

        print!("{s}");

        if !is_all && len > 1 {
            info!(
                "There are {} additional records. Please use the '-a' switch to see them.",
                len - 1
            );
        }

        if s.is_empty() {
            return Ok(1);
        }

        Ok(0)
    }

    pub fn search(kw: &str) -> Result<i32> {
        let cache = new_cache!()?;
        search_pkgs(&cache, kw)?;

        Ok(0)
    }

    pub fn list_files(kw: &str) -> Result<i32> {
        let res = find(kw, true, false)?;

        let height = WRITER.get_height();

        let mut pager = if res.len() <= height.into() {
            Pager::new(true, false)?
        } else {
            Pager::new(false, false)?
        };

        // let pager_name = pager.pager_name().to_owned();
        let mut out = pager.get_writer()?;

        ALLOWCTRLC.store(true, Ordering::Relaxed);

        for (_, line) in res {
            writeln!(out, "{line}").ok();
        }

        drop(out);
        pager.wait_for_exit().ok();

        Ok(0)
    }

    pub fn search_file(kw: &str) -> Result<i32> {
        let res = find(kw, false, false)?;

        let height = WRITER.get_height();

        let mut pager = if res.len() <= height.into() {
            Pager::new(true, false)?
        } else {
            Pager::new(false, false)?
        };

        // let pager_name = pager.pager_name().to_owned();
        let mut out = pager.get_writer()?;

        ALLOWCTRLC.store(true, Ordering::Relaxed);

        for (_, line) in res {
            writeln!(out, "{line}").ok();
        }

        drop(out);
        pager.wait_for_exit().ok();

        Ok(0)
    }

    pub fn fix_broken(&self, v: FixBroken) -> Result<i32> {
        needs_root()?;
        lock_oma()?;

        if v.dry_run {
            DRYRUN.store(true, Ordering::Relaxed);
        }

        let start_time = OffsetDateTime::now_utc()
            .to_offset(*TIME_OFFSET)
            .to_string();

        let cache = new_cache!()?;

        let (action, len, need_fixsystem) = apt_handler(&cache, false, false, false, false)?;

        if len == 0 && !need_fixsystem {
            info!("No need to do anything");
            return Ok(0);
        }

        let mut list = action.install.clone();
        list.extend(action.update.clone());
        list.extend(action.downgrade.clone());

        let install_size = cache.depcache().disk_size();
        size_checker(&install_size, download_size(&list, &cache)?)?;
        if len != 0 {
            display_result(&action, &cache, false)?;
        }

        packages_download_runner(&self.runtime, &list, &self.client, None, None)?;

        if !DRYRUN.load(Ordering::Relaxed) {
            cache.commit(
                &mut NoProgress::new_box(),
                &mut AptInstallProgress::new_box(),
            )?;
        }

        let end_time = OffsetDateTime::now_utc()
            .to_offset(*TIME_OFFSET)
            .to_string();

        log_to_file(&action, &start_time, &end_time)?;

        Ok(0)
    }

    pub fn dep(list: &[String], rdep: bool) -> Result<i32> {
        let cache = new_cache!()?;
        let mut res = vec![];
        for c in list {
            let oma_pkg = query_pkgs(&cache, c)?;
            if oma_pkg.is_empty() {
                bail!("Could not find any package for: {}", c);
            }

            res.extend(oma_pkg);
        }

        for (pkginfo, is_cand) in res {
            if !is_cand {
                continue;
            }

            let deps = if rdep { pkginfo.rdeps } else { pkginfo.deps };

            println!("{}:", pkginfo.package);

            if rdep {
                println!("Reverse Depends:")
            }

            for (k, v) in deps {
                for i in v {
                    let mut s = String::new();

                    let k = if rdep {
                        let k = k.strip_suffix('s').unwrap();
                        if k.ends_with('e') {
                            format!("{k}d by")
                        } else if k == "Break" {
                            "Broken by".to_string()
                        } else if k == "PreDepend" {
                            "Pre-depended by".to_string()
                        } else {
                            format!("{k}ed by")
                        }
                    } else {
                        k.clone()
                    };

                    if i.len() == 1 {
                        let entry = i.first().unwrap();

                        s.push_str(&format!("  {k}: {}", entry.name));
                        if let Some(ref comp) = entry.comp_ver {
                            s.push_str(&format!(" ({comp})"));
                        }
                    } else {
                        let mut or_str = String::new();
                        let total = i.len() - 1;
                        for (num, c) in i.iter().enumerate() {
                            or_str.push_str(&c.name);
                            if let Some(comp) = &c.comp_ver {
                                let _ = write!(or_str, " ({comp})");
                            }
                            if num != total {
                                or_str.push_str(" | ");
                            } else {
                                or_str.push_str(", ");
                            }
                        }
                        s = format!("{k}: {or_str}");
                    }

                    println!("{s}");
                }
            }
        }

        Ok(0)
    }

    pub fn download(&self, v: Download) -> Result<i32> {
        if DRYRUN.load(Ordering::Relaxed) {
            return Ok(0);
        }

        let cache = new_cache!()?;

        let mut downloads = vec![];
        for i in &v.packages {
            let oma_pkg = query_pkgs(&cache, i)?;
            for (i, is_cand) in oma_pkg {
                if !is_cand {
                    continue;
                }
                let pkg = i.package;
                let version = i.version;
                let pkg = cache.get(&pkg).unwrap();
                let version = pkg.get_version(&version).unwrap();
                let urls = version.uris();

                downloads.push(InstallRow {
                    name: pkg.name().to_string(),
                    name_no_color: pkg.name().to_string(),
                    new_version: version.version().to_string(),
                    version: version.version().to_string(),
                    size: version.installed_size().to_string(),
                    pkg_urls: urls.collect(),
                    checksum: version.sha256(),
                    pure_download_size: version.size(),
                })
            }
        }

        let path = v.path.unwrap_or(".".to_owned());
        let path = Path::new(&path);

        packages_download_runner(&self.runtime, &downloads, &self.client, None, Some(path))?;

        success!(
            "Successfully downloaded some packages to path: {}",
            path.canonicalize().unwrap_or(path.to_path_buf()).display()
        );

        Ok(0)
    }

    fn install_inner(&self, opt: &InstallOptions, count: usize) -> InstallResult<Action> {
        let pkgs = opt.packages.clone().unwrap_or_default();
        let cache = install_handle(
            &pkgs,
            opt.install_dbg,
            opt.reinstall,
            opt.install_recommends,
            opt.install_suggests,
        )?;

        let (action, len, need_fixsystem) =
            apt_handler(&cache, opt.no_fixbroken, opt.force_yes, false, true)?;

        if len == 0 && !need_fixsystem {
            success!("No need to do anything.");
            return Ok(action);
        }

        let mut list = vec![];
        list.extend(action.install.clone());
        list.extend(action.update.clone());
        list.extend(action.downgrade.clone());
        list.extend(action.reinstall.clone());

        if count == 1 {
            let disk_size = cache.depcache().disk_size();
            size_checker(&disk_size, download_size(&list, &cache)?)?;
            if len != 0 {
                display_result(&action, &cache, opt.yes)?;
            }
        }

        // TODO: limit 参数（限制下载包并发）目前是写死的，以后将允许用户自定义并发数
        packages_download_runner(&self.runtime, &list, &self.client, None, None)?;
        apt_install(
            cache,
            opt.yes,
            opt.force_yes,
            opt.force_confnew,
            opt.dpkg_force_all,
        )?;

        Ok(action)
    }

    pub fn remove(r: RemoveOptions) -> Result<i32> {
        needs_root()?;
        lock_oma()?;

        let start_time = OffsetDateTime::now_utc()
            .to_offset(*TIME_OFFSET)
            .to_string();

        if r.dry_run {
            DRYRUN.store(true, Ordering::Relaxed);
        }

        if r.yes {
            yes_warn();
        }

        let cache = new_cache!()?;

        for i in &r.packages {
            let pkg = cache.get(i).context(format!("Can not get package {i}"))?;
            if !pkg.is_installed() {
                info!("Package {i} is not installed, so no need to remove.");
                continue;
            }
            mark_delete(&pkg, !r.keep_config)?;
        }

        let (action, len, need_fixsystem) =
            apt_handler(&cache, false, r.force_yes, !r.keep_config, r.no_autoremove)?;

        if len == 0 && !need_fixsystem {
            success!("No need to do anything.");
            return Ok(0);
        }

        display_result(&action, &cache, r.yes)?;

        let mut progress = OmaAptInstallProgress::new_box(r.yes, r.force_yes, false, false);

        if !DRYRUN.load(Ordering::Relaxed) {
            cache.commit(&mut NoProgress::new_box(), &mut progress)?;
        }

        let end_time = OffsetDateTime::now_utc()
            .to_offset(*TIME_OFFSET)
            .to_string();

        log_to_file(&action, &start_time, &end_time)?;

        Ok(0)
    }

    pub fn refresh(&self) -> Result<i32> {
        needs_root()?;
        lock_oma()?;

        update_db_runner(&self.runtime, &get_sources()?, &self.client, None)?;

        let cache = new_cache!()?;

        let pb = MB.add(ProgressBar::new_spinner());
        oma_spinner(&pb);
        pb.set_message("Quering status information ...");

        let upgradable = PackageSort::default().upgradable();
        let autoremove = PackageSort::default().auto_removable();
        let upgradable = cache.packages(&upgradable).collect::<Vec<_>>();
        let autoremove = cache.packages(&autoremove).collect::<Vec<_>>();

        pb.finish_and_clear();

        let mut output = vec![];
        if !upgradable.is_empty() {
            output.push(format!("{} package(s) can be upgraded", upgradable.len()));
        }

        if !autoremove.is_empty() {
            output.push(format!("{} package(s) can be removed", autoremove.len()));
        }

        if !output.is_empty() {
            output.push("Run 'oma upgrade' to see it".to_string());
            let s = output.join(", ") + ".";
            success!("Successfully refreshed package database. {s}");
        } else {
            success!("Successfully refreshed package database. No update available.");
        }

        Ok(0)
    }

    pub fn pick(&self, p: PickOptions) -> Result<i32> {
        needs_root()?;
        lock_oma()?;

        if p.dry_run {
            DRYRUN.store(true, Ordering::Relaxed);
        }

        if !p.no_refresh {
            update_db_runner(&self.runtime, &get_sources()?, &self.client, None)?;
        }

        let start_time = OffsetDateTime::now_utc()
            .to_offset(*TIME_OFFSET)
            .to_string();

        let cache = new_cache!()?;
        let pkg = cache
            .get(&p.package)
            .context(format!("Can not get package: {}", p.package))?;

        let versions = pkg.versions().collect::<Vec<_>>();

        let versions_str = versions
            .iter()
            .map(|x| x.version().to_string())
            .collect::<Vec<_>>();

        let mut versions_str_display = versions_str.clone();

        let mut v = vec![];

        for i in 0..versions_str.len() {
            for j in 1..versions_str.len() {
                if i == j {
                    continue;
                }

                if versions_str[i] == versions_str[j] {
                    v.push((i, j));
                }
            }
        }

        for (a, b) in v {
            let uri_a = versions[a].uris().next().unwrap();
            versions_str_display[a] = format!("{} (from: {})", versions_str[a], uri_a);

            let uri_b = versions[b].uris().next().unwrap();
            versions_str_display[b] = format!("{} (from: {})", versions_str[b], uri_b);
        }

        let installed = pkg.installed();

        let index = if DRYRUN.load(Ordering::Relaxed) {
            info!("Pkg: {} can select: {versions_str_display:?}", pkg.name());
            info!("In Dry-run mode, select first: {}", versions_str_display[0]);

            Ok(0usize)
        } else {
            let theme = ColorfulTheme::default();
            let mut dialoguer = Select::with_theme(&theme);

            dialoguer.items(&versions_str_display);
            dialoguer.with_prompt(format!("Select {} version:", pkg.name()));

            let pos = if let Some(installed) = installed {
                versions_str
                    .iter()
                    .position(|x| x == installed.version())
                    .unwrap_or(0)
            } else {
                0
            };

            dialoguer.default(pos);

            dialoguer.interact()
        };

        if let Ok(index) = index {
            let version = &versions[index];

            let installed = pkg.installed();

            if installed.as_ref() == Some(version)
                && installed.map(|x| x.sha256()) == Some(version.sha256())
            {
                success!("No need to do anything.");
                return Ok(0);
            }

            version.set_candidate();

            pkg.mark_install(true, true);
            pkg.protect();

            let (action, _, _) = apt_handler(&cache, p.no_fixbroken, false, false, true)?;
            let disk_size = cache.depcache().disk_size();

            let mut list = vec![];
            list.extend(action.install.clone());
            list.extend(action.update.clone());
            list.extend(action.downgrade.clone());

            size_checker(&disk_size, download_size(&list, &cache)?)?;
            display_result(&action, &cache, false)?;

            packages_download_runner(&self.runtime, &list, &self.client, None, None)?;

            apt_install(cache, false, false, false, false)?;

            let end_time = OffsetDateTime::now_utc().to_offset(*TIME_OFFSET);

            let end_time = end_time.to_string();

            log_to_file(&action, &start_time, &end_time)?;
        }

        Ok(0)
    }

    pub fn mark(opt: Mark) -> Result<i32> {
        needs_root()?;

        if opt.dry_run {
            DRYRUN.store(true, Ordering::Relaxed);
        }

        fn check(cache: &Cache, pkg: &str) -> Result<()> {
            let package = cache.get(pkg);
            if package.is_none() {
                bail!("Can not get package {pkg}")
            }
            let package = package.unwrap();
            package
                .installed()
                .context(format!("{pkg} is not installed!"))?;

            Ok(())
        }

        let cache = new_cache!()?;

        match opt.action {
            MarkAction::Hold(args) => {
                for i in &args.pkgs {
                    check(&cache, i)?;
                }

                let selections = dpkg_selections()?;

                for i in args.pkgs {
                    let v = selections
                        .iter()
                        .find(|x| x.0 == i)
                        .context("dpkg data is broken!")?;

                    if v.1 == "hold" {
                        info!("{} is already hold!", i);
                        continue;
                    }

                    dpkg_set_selections(&i, "hold")?;
                    success!("{} is set to hold.", i);
                }
            }
            MarkAction::Unhold(args) => {
                for i in &args.pkgs {
                    check(&cache, i)?;
                }
                let selections = dpkg_selections()?;

                for i in args.pkgs {
                    let v = selections
                        .iter()
                        .find(|x| x.0 == i)
                        .context("dpkg data is broken!")?;

                    if v.1 == "install" {
                        info!("{} is already unhold!", i);
                        continue;
                    }

                    dpkg_set_selections(&i, "unhold")?;
                    success!("{} is set to unhold.", i);
                }
            }
            MarkAction::Manual(args) => {
                for i in &args.pkgs {
                    check(&cache, i)?;
                }

                for i in args.pkgs {
                    let pkg = cache.get(&i).unwrap();
                    if !pkg.is_auto_installed() {
                        info!("{i} is already manual installed status.");
                        continue;
                    }
                    info!("Setting {i} to manual installed status ...");
                    pkg.mark_auto(false);
                }

                if DRYRUN.load(Ordering::Relaxed) {
                    return Ok(0);
                }

                cache.commit(
                    &mut NoProgress::new_box(),
                    &mut AptInstallProgress::new_box(),
                )?;
            }
            MarkAction::Auto(args) => {
                for i in &args.pkgs {
                    check(&cache, i)?;
                }

                for i in args.pkgs {
                    let pkg = cache.get(&i).unwrap();
                    if pkg.is_auto_installed() {
                        info!("{i} is already auto installed status.");
                        continue;
                    }
                    info!("Setting {i} to auto installed status ...");
                    pkg.mark_auto(true);
                }

                if DRYRUN.load(Ordering::Relaxed) {
                    return Ok(0);
                }

                cache.commit(
                    &mut NoProgress::new_box(),
                    &mut AptInstallProgress::new_box(),
                )?;
            }
        }

        Ok(0)
    }

    pub fn command_not_found(kw: &str) -> Result<i32> {
        let cache = new_cache!()?;
        let f = find(&format!("usr/bin/{kw}"), false, true);

        let mut res = vec![];

        if let Ok(f) = f {
            for (pkg, pkg_str) in f {
                let p = cache.get(&pkg);
                if p.is_none() {
                    continue;
                }
                let p = p.unwrap();
                let version = p.candidate().unwrap();
                let pkg_str = pkg_str.replace(": ", " (") + ")";
                let s = format!(
                    "{pkg_str}: {}",
                    version.description().unwrap_or("".to_string())
                );
                if !res.contains(&s) {
                    res.push(s);
                }
            }
        }
        if !res.is_empty() {
            println!(
                "{}",
                style(format!(
                    "Command not found: {kw}. This command may be found from the following package(s):\n"
                ))
                .bold()
            );

            for i in res {
                println!("{i}");
            }
        } else {
            error!("Command not found: {kw}");
        }

        Ok(127)
    }

    pub fn clean() -> Result<i32> {
        if DRYRUN.load(Ordering::Relaxed) {
            return Ok(0);
        }

        needs_root()?;

        let dir = std::fs::read_dir(&*DOWNLOAD_DIR)?;

        for i in dir.flatten() {
            if i.path().extension().and_then(|x| x.to_str()) == Some("deb") {
                std::fs::remove_file(i.path()).ok();
            }
        }

        let p = DOWNLOAD_DIR.join("..");

        std::fs::remove_file(p.join("pkgcache.bin")).ok();
        std::fs::remove_file(p.join("srcpkgcache.bin")).ok();

        success!("Clean successfully.");

        Ok(0)
    }

    pub fn log() -> Result<i32> {
        let f = std::fs::File::open("/var/log/oma/history")?;
        let r = std::io::BufReader::new(f);
        let mut pager = Pager::new(false, false)?;
        let mut out = pager.get_writer()?;

        ALLOWCTRLC.store(true, Ordering::Relaxed);

        for i in r.lines().flatten() {
            writeln!(out, "{}", i).ok();
        }

        drop(out);
        pager.wait_for_exit().ok();

        Ok(0)
    }

    #[cfg(feature = "aosc")]
    pub fn topics(&self, opt: Topics) -> Result<i32> {
        if DRYRUN.load(Ordering::Relaxed) {
            return Ok(0);
        }

        needs_root()?;

        let mut tm = topics::TopicManager::new()?;

        let (opt_in, opt_out) = if opt.opt_in.is_none() && opt.opt_out.is_none() {
            topics::dialoguer(&mut tm, &self.runtime, &self.client)?
        } else {
            let opt_in = opt.opt_in.unwrap_or_default();
            let opt_out = opt.opt_out.unwrap_or_default();

            (opt_in, opt_out)
        };

        for i in opt_in {
            tm.opt_in(&self.client, &self.runtime, &i)?;
        }

        let mut downgrade_pkgs = vec![];

        for i in opt_out {
            let pkgs = tm.opt_out(&i)?;
            downgrade_pkgs.extend(pkgs);
        }

        let cache = new_cache!()?;

        let mut pkgs = vec![];

        let enabled_pkgs = tm
            .enabled
            .iter()
            .flat_map(|x| &x.packages)
            .collect::<Vec<_>>();

        for i in &downgrade_pkgs {
            let pkg = cache.get(i);

            if let Some(pkg) = pkg {
                if enabled_pkgs.contains(&&pkg.name().to_string()) {
                    continue;
                }

                if pkg.is_installed() {
                    pkgs.push(format!("{i}/stable"))
                }
            }
        }

        tm.write_enabled()?;

        self.update(UpgradeOptions {
            packages: Some(pkgs),
            yes: false,
            force_yes: false,
            force_confnew: false,
            dry_run: false,
            dpkg_force_all: false,
            no_autoremove: true,
        })?;

        Ok(0)
    }

    pub fn pkgnames(s: Option<String>) -> Result<i32> {
        let cache = new_cache!()?;
        let sort = PackageSort::default().names();

        let pkgs = match s {
            Some(ref s) => {
                Either::Left(cache.packages(&sort).filter(|x| x.name().starts_with(&**s)))
            }
            None => Either::Right(cache.packages(&sort)),
        };

        for pkg in pkgs {
            println!("{}", pkg.name());
        }

        Ok(0)
    }
}

fn needs_fix_system(cache: &Cache) -> (bool, Vec<String>) {
    let sort = PackageSort::default().installed();
    let pkgs = cache.packages(&sort);

    let mut reinstall = vec![];

    let mut need = false;

    for pkg in pkgs {
        // current_state 的定义来自 apt 的源码:
        //    enum PkgCurrentState {NotInstalled=0,UnPacked=1,HalfConfigured=2,
        //    HalfInstalled=4,ConfigFiles=5,Installed=6,
        //    TriggersAwaited=7,TriggersPending=8};
        if pkg.current_state() != 6 {
            tracing::info!(
                "pkg {} current state is {}",
                pkg.name(),
                pkg.current_state()
            );
            need = true;
            match pkg.current_state() {
                2 | 4 => {
                    pkg.mark_reinstall(true);
                    reinstall.push(pkg.name().to_string());
                }
                _ => continue,
            }
        }
    }

    tracing::info!("Needs reinstall package: {reinstall:?}");

    (need, reinstall)
}

fn dpkg_set_selections(pkg: &str, user_action: &str) -> Result<()> {
    let mut cmd = Command::new("dpkg")
        .arg("--set-selections")
        .stdin(Stdio::piped())
        .spawn()?;

    let user_action = if user_action == "unhold" {
        "install"
    } else {
        user_action
    };

    if DRYRUN.load(Ordering::Relaxed) {
        return Ok(());
    }

    cmd.stdin
        .as_mut()
        .unwrap()
        .write_all(format!("{pkg} {user_action}").as_bytes())?;

    cmd.wait()?;

    Ok(())
}

fn dpkg_selections() -> Result<Vec<(String, String)>> {
    let mut cmd = Command::new("dpkg");

    cmd.arg("--get-selections");
    let output = cmd.output()?;

    if !output.status.success() {
        bail!(
            "dpkg --get-selections return non-zero code.\n{}",
            String::from_utf8_lossy(&output.stderr)
        )
    }

    let mut seclections = std::str::from_utf8(&output.stdout)?.split('\n');
    seclections.nth_back(0);

    let mut list = vec![];

    for i in seclections {
        let mut split = i.split_whitespace();
        let name = split.next().context("line is broken: {i}")?;
        let status = split.next().context("line is broken: {i}")?;

        list.push((name.to_string(), status.to_string()));
    }

    Ok(list)
}

/// apt autoremove
fn autoremove(cache: &Cache, is_purge: bool, no_autoremove: bool) -> Result<Vec<String>> {
    if no_autoremove {
        return Ok(vec![]);
    }
    let sort = PackageSort::default();

    let mut pkgs = vec![];

    for pkg in cache.packages(&sort) {
        if pkg.marked_delete() {
            continue;
        }

        if pkg.is_auto_removable() {
            mark_delete(&pkg, is_purge)?;
            pkgs.push(pkg.name().to_string());
        }
    }

    Ok(pkgs)
}

/// Install packages
fn apt_install(
    cache: Cache,
    yes: bool,
    force_yes: bool,
    force_confnew: bool,
    dpkg_force_all: bool,
) -> InstallResult<()> {
    if DRYRUN.load(Ordering::Relaxed) {
        return Ok(());
    }

    if let Err(e) = apt_lock() {
        let e = e.to_string();
        if e.contains("dpkg --configure -a") {
            info!(
                "dpkg was interrupted, running {} ...",
                style("dpkg --configure -a").green().bold()
            );
            let cmd = Command::new("dpkg")
                .arg("--configure")
                .arg("-a")
                .output()
                .map_err(|e| anyhow!("{e}"))?;

            if !cmd.status.success() {
                InstallError::Anyhow(anyhow!(
                    "Running `dpkg --configure -a` return non-zero code: {}",
                    cmd.status.code().unwrap()
                ));
            }

            apt_lock()?;
        }
    }

    let pb = MB.add(ProgressBar::new_spinner());
    pb.set_message("Verifying the integrity of packages ...");
    oma_spinner(&pb);

    cache.get_archives(&mut NoProgress::new_box())?;

    pb.finish_and_clear();

    apt_unlock_inner();

    let mut progress =
        OmaAptInstallProgress::new_box(yes, force_yes, force_confnew, dpkg_force_all);

    if let Err(e) = cache.do_install(&mut progress) {
        apt_lock_inner().map_err(|e| anyhow!("{e}"))?;
        apt_unlock();
        return Err(e.into());
    }

    apt_unlock();

    Ok(())
}

fn yes_warn() {
    warn!("Now you are using automatic mode, if this is not your intention, press Ctrl + C to stop the operation!!!!!");
}

/// Handle user input, find pkgs
fn install_handle(
    list: &[String],
    install_dbg: bool,
    reinstall: bool,
    install_recommends: bool,
    install_suggest: bool,
) -> Result<Cache> {
    tracing::debug!("Querying the packages database ...");
    let pb = MB.add(ProgressBar::new_spinner());
    pb.set_message("Querying packages database ...");
    oma_spinner(&pb);

    // Get local packages
    let local_debs = list
        .iter()
        .filter(|x| x.ends_with(".deb"))
        .collect::<Vec<_>>();

    let cache = new_cache!(&local_debs)?;
    let mut pkgs = vec![];
    // Query pkgs
    for i in list {
        let i_res = query_pkgs(&cache, i)?;
        if i_res.is_empty() {
            bail!("Package {i} does not exist.");
        }
        pkgs.extend(i_res);
        tracing::info!("Select pkg: {i}");
    }

    let other_install = if install_recommends {
        install_other(&pkgs, &cache, "recommend")?
    } else {
        vec![]
    };

    pkgs.extend(other_install);

    let other_install = if install_suggest {
        install_other(&pkgs, &cache, "suggest")?
    } else {
        vec![]
    };

    pkgs.extend(other_install);

    for (pkginfo, is_cand) in pkgs {
        if !is_cand {
            continue;
        }

        let pkg = Package::new(&cache, pkginfo.raw_pkg);
        let version = Version::new(pkginfo.version_raw, &pkg);

        mark_install(
            &cache,
            &pkginfo.package,
            version.unique(),
            reinstall,
            false,
            Some(&pb),
        )?;

        if install_dbg && pkginfo.has_dbg {
            let pkginfo = query_pkgs(&cache, &format!("{}-dbg", pkginfo.package))?;
            let pkginfo = pkginfo.into_iter().filter(|x| x.1).collect::<Vec<_>>();
            let dbg_pkgname = &pkginfo[0].0;
            let version = dbg_pkgname.version_raw.unique();

            mark_install(
                &cache,
                &format!("{}-dbg", dbg_pkgname.package),
                version,
                reinstall,
                false,
                Some(&pb),
            )?;
        } else if install_dbg && !pkginfo.has_dbg {
            warn!("{} has no debug symbol package!", &pkginfo.package);
        }
    }

    pb.finish_and_clear();

    Ok(cache)
}

fn install_other(
    pkgs: &[(PkgInfo, bool)],
    cache: &Cache,
    other: &str,
) -> Result<Vec<(PkgInfo, bool)>> {
    let mut other_install = vec![];

    for (pkginfo, is_cand) in pkgs {
        if !is_cand {
            continue;
        }
        let other = match other {
            "recommend" => &pkginfo.recommend,
            "suggest" => &pkginfo.suggest,
            _ => unreachable!(),
        };

        for i in other {
            for j in i {
                let pkg = cache.get(&j.name);

                if let Some(pkg) = pkg {
                    let version = if let Some(v) = &j.ver {
                        pkg.get_version(v)
                    } else {
                        pkg.candidate()
                    };

                    let version = version
                        .context(format!("Can not get version of package: {}", pkg.name()))?;

                    let pkginfo = PkgInfo::new(cache, version.unique(), &pkg)?;
                    tracing::info!("Select pkg: {}", j.name);
                    other_install.push((pkginfo, true));
                }
            }
        }
    }

    Ok(other_install)
}

#[derive(Clone)]
pub struct Action {
    pub update: Vec<InstallRow>,
    pub install: Vec<InstallRow>,
    pub del: Vec<RemoveRow>,
    pub reinstall: Vec<InstallRow>,
    pub downgrade: Vec<InstallRow>,
}

impl Action {
    fn new(
        update: Vec<InstallRow>,
        install: Vec<InstallRow>,
        del: Vec<RemoveRow>,
        reinstall: Vec<InstallRow>,
        downgrade: Vec<InstallRow>,
    ) -> Self {
        Self {
            update,
            install,
            del,
            reinstall,
            downgrade,
        }
    }
}

impl Debug for Action {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if !self.update.is_empty() {
            write!(f, "Update: ")?;
            for (i, c) in self.update.iter().enumerate() {
                write!(f, "{} ({})", c.name_no_color, c.version)?;
                if i < self.update.len() - 1 {
                    write!(f, ", ")?;
                }
            }

            writeln!(f)?;
        }

        if !self.install.is_empty() {
            write!(f, "Install: ")?;
            for (i, c) in self.install.iter().enumerate() {
                write!(f, "{} ({})", c.name_no_color, c.version)?;
                if i < self.install.len() - 1 {
                    write!(f, ", ")?;
                }
            }

            writeln!(f)?;
        }

        if !self.del.is_empty() {
            write!(f, "Remove: ")?;
            for (i, c) in self.del.iter().enumerate() {
                write!(f, "{}", c._name_no_color)?;
                if i < self.del.len() - 1 {
                    write!(f, ", ")?;
                }
            }

            writeln!(f)?;
        }

        if !self.downgrade.is_empty() {
            write!(f, "Downgrade: ")?;
            for (i, c) in self.downgrade.iter().enumerate() {
                write!(f, "{} ({})", c.name_no_color, c.version)?;
                if i < self.downgrade.len() - 1 {
                    write!(f, ", ")?;
                }
            }

            writeln!(f)?;
        }

        Ok(())
    }
}

/// Handle apt resolve result to display results
fn apt_handler(
    cache: &Cache,
    no_fixbroken: bool,
    force_yes: bool,
    is_purge: bool,
    no_autoremove: bool,
) -> Result<(Action, usize, bool)> {
    if force_yes {
        let config = Config::new_clear();
        config.set("APT::Get::force-yes", "true");
        tracing::debug!("dpkg force-yes mode is enabled");
    }

    let fix_broken = !no_fixbroken;
    if fix_broken {
        cache.fix_broken();
        tracing::debug!("oma will fix broken system dependencies status");
    }

    if let Err(e) = cache.resolve(fix_broken) {
        let finded = find_unmet_deps(cache)?;
        if finded {
            bail!("")
        }

        return Err(e.into());
    }

    let autoremove_list = autoremove(cache, is_purge, no_autoremove)?;

    if let Err(e) = cache.resolve(fix_broken) {
        let finded = find_unmet_deps(cache)?;
        if finded {
            bail!("")
        }

        return Err(e.into());
    }

    let (need_fix_system, _) = needs_fix_system(cache);

    let changes = cache.get_changes(true).collect::<Vec<_>>();
    let len = changes.len();

    let mut update: Vec<InstallRow> = vec![];
    let mut install: Vec<InstallRow> = vec![];
    let mut del: Vec<RemoveRow> = vec![];
    let mut reinstall: Vec<InstallRow> = vec![];
    let mut downgrade: Vec<InstallRow> = vec![];

    for pkg in changes {
        if pkg.marked_install() {
            let cand = pkg.candidate().take().context(format!(
                "Can not get package version in apt database: {}",
                pkg.name()
            ))?;

            let size = cand.installed_size();
            let human_size = format!("+{}", HumanBytes(size));

            install.push(InstallRow {
                name: style(pkg.name()).green().to_string(),
                name_no_color: pkg.name().to_string(),
                version: cand.version().to_string(),
                new_version: cand.version().to_string(),
                size: human_size,
                pkg_urls: cand.uris().collect(),
                checksum: cand.get_record(RecordField::SHA256),
                pure_download_size: cand.size(),
            });

            tracing::info!("Pkg {} {} is marked as install", pkg.name(), cand.version());

            // If the package is marked install then it will also
            // show up as marked upgrade, downgrade etc.
            // Check this first and continue.
            continue;
        }

        if pkg.marked_upgrade() {
            let cand = pkg.candidate().take().context(format!(
                "Can not get package version in apt database: {}",
                pkg.name()
            ))?;

            let version = cand.version();

            let old_pkg = pkg
                .installed()
                .context(format!("Can not get installed version: {}", pkg.name()))?;

            let old_version = old_pkg.version();

            let new_pkg = pkg
                .candidate()
                .context(format!("Can not get candidate version: {}", pkg.name()))?;

            let new_size = new_pkg.installed_size() as i64;
            let old_size = old_pkg.installed_size() as i64;

            let size = new_size - old_size;

            let human_size = if size >= 0 {
                format!("+{}", HumanBytes(size as u64))
            } else {
                format!("-{}", HumanBytes(size.unsigned_abs()))
            };

            update.push(InstallRow {
                name: style(pkg.name()).color256(87).to_string(),
                name_no_color: pkg.name().to_string(),
                version: format!("{old_version} -> {version}"),
                new_version: version.to_string(),
                size: human_size,
                pkg_urls: cand.uris().collect(),
                checksum: cand.get_record(RecordField::SHA256),
                pure_download_size: cand.size(),
            });

            tracing::info!(
                "Pkg {} is marked as upgrade: {old_version} -> {}",
                pkg.name(),
                cand.version()
            );
        }

        if pkg.marked_delete() {
            let name = pkg.name();

            let is_purge = pkg.marked_purge();

            let mut v = Vec::new();

            if autoremove_list.contains(&pkg.name().to_string()) {
                v.push("removed as unneeded dependency");
            }

            if is_purge {
                v.push("purge configuration files");
            }

            let s = if !v.is_empty() {
                let mut v = v.join("; ");
                v.get_mut(0..1).map(|s| {
                    s.make_ascii_uppercase();
                    &*s
                });

                v
            } else {
                "".to_string()
            };

            let remove_row = RemoveRow {
                name: style(name).red().bold().to_string(),
                _name_no_color: name.to_owned(),
                // size: HumanBytes(size).to_string(),
                detail: style(s).red().to_string(),
            };

            del.push(remove_row);

            tracing::info!("Pkg {} is marked as delete", pkg.name());
        }

        if pkg.marked_reinstall() {
            let version = pkg.installed().unwrap();
            reinstall.push(InstallRow {
                name: style(pkg.name()).blue().to_string(),
                name_no_color: pkg.name().to_string(),
                version: pkg.installed().unwrap().version().to_owned(),
                new_version: pkg.installed().unwrap().version().to_owned(),
                size: HumanBytes(0).to_string(),
                pkg_urls: version.uris().collect(),
                checksum: version.get_record(RecordField::SHA256),
                pure_download_size: version.size(),
            });

            tracing::info!("Pkg {} is marked as reinstall", pkg.name());
        }

        if pkg.marked_downgrade() {
            let cand = pkg.candidate().take().context(format!(
                "Can not get package version in apt database: {}",
                pkg.name()
            ))?;

            let version = cand.version();

            let old_pkg = pkg
                .installed()
                .context(format!("Can not get installed version: {}", pkg.name()))?;

            let old_version = old_pkg.version();
            let old_size = old_pkg.installed_size() as i64;

            let new_pkg = pkg.get_version(version).context(format!(
                "Can not get package version in apt database: {}",
                pkg.name()
            ))?;

            let new_size = new_pkg.installed_size() as i64;
            let size = new_size - old_size;

            let human_size = if size >= 0 {
                format!("+{}", HumanBytes(size as u64))
            } else {
                format!("-{}", HumanBytes(size.unsigned_abs()))
            };

            downgrade.push(InstallRow {
                name: style(pkg.name()).yellow().to_string(),
                name_no_color: pkg.name().to_string(),
                version: format!("{old_version} -> {version}"),
                new_version: version.to_string(),
                size: human_size,
                pkg_urls: cand.uris().collect(),
                checksum: cand.get_record(RecordField::SHA256),
                pure_download_size: cand.size(),
            });

            tracing::info!(
                "Pkg {} is marked as downgrade: {old_version} -> {}",
                pkg.name(),
                cand.version()
            );
        }
    }

    let action = Action::new(update, install, del, reinstall, downgrade);

    Ok((action, len, need_fix_system))
}
