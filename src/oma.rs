use anyhow::{anyhow, bail, Context, Result};
use console::style;
use dialoguer::{theme::ColorfulTheme, Select};
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
use std::{
    fmt::Write as FmtWrite,
    io::BufRead,
    sync::{atomic::AtomicBool, Arc},
};
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
    formatter::{
        display_result, download_size, find_unmet_deps, NoProgress, OmaAptInstallProgress,
    },
    info,
    pager::Pager,
    pkg::{mark_delete, mark_install, query_pkgs, search_pkgs, PkgInfo},
    success,
    utils::{lock_oma, log_to_file, needs_root, size_checker},
    warn, ALLOWCTRLC, DRYRUN, TIME_OFFSET, WRITER,
};

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
    pub fn update(&self, u: UpgradeOptions) -> Result<()> {
        let mut u = u;
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
            let cache = install_handle(&pkgs, false, false)?;

            cache
                .upgrade(&Upgrade::FullUpgrade)
                .map_err(|e| anyhow!("{e}"))?;

            let (action, len) = apt_handler(&cache, false, u.force_yes, false, u.no_autoremove)?;

            if len == 0 {
                success!("No need to do anything.");
            }

            let mut list = action.update.clone();
            list.extend(action.install.clone());
            list.extend(action.downgrade.clone());

            if count == 0 {
                let disk_size = cache.depcache().disk_size();
                size_checker(&disk_size, download_size(&list, &cache)?)?;
                if len != 0 && !u.yes {
                    display_result(&action, &cache)?;
                }
            }

            packages_download_runner(runtime, &list, client, None, None)?;
            apt_install(cache, u.yes, u.force_yes, u.force_confnew, u.dpkg_force_all)?;

            Ok(action)
        }

        let mut count = 0;
        let mut is_force_all = u.dpkg_force_all;

        loop {
            match update_inner(&self.runtime, &self.client, count, &u) {
                Err(e) => {
                    match e {
                        InstallError::Anyhow(e) => return Err(e),
                        InstallError::RustApt(e) => {
                            warn!("apt has retrn non-zero code, retrying {count} times");
                            // 若重试两次不凑效则启动 dpkg-force-all 模式
                            if count == 2 && !is_force_all {
                                warn!("Try use dpkg-force-all mode to fix broken dependencies");
                                is_force_all = true;
                                u.dpkg_force_all = true;
                                count = 0;
                            }
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
                        bail!("Your system has broken dependencies, try to use {} to fix broken dependencies\nIf this does not work, please contact upstream: https://github.com/aosc-dev/aosc-os-abbs", style("oma fix-broken").green().bold())
                    }

                    return Ok(());
                }
            }
        }
    }

    pub fn install(&self, opt: InstallOptions) -> Result<()> {
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

        if !opt.no_upgrade {
            update_db_runner(&self.runtime, &get_sources()?, &self.client, None)?;
        }

        let mut count = 0;
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

                    return log_to_file(&v, &start_time, &end_time);
                }
            }
        }
    }

    pub fn list(opt: &ListOptions) -> Result<()> {
        let cache = new_cache!()?;

        let mut sort = PackageSort::default();

        if opt.installed {
            sort = sort.installed();
        }

        if opt.upgradable {
            sort = sort.upgradable();
        }

        let packages = cache.packages(&sort);

        let mut res = vec![];

        if opt.packages.is_none() {
            if !opt.all {
                for pkg in packages {
                    let mut mirrors = vec![];
                    let version = pkg.candidate();

                    if let Some(version) = version {
                        let uris = version.uris();
                        for i in uris {
                            let mirror = i.split('/').nth_back(3).unwrap_or("unknown").to_owned();
                            if !mirrors.contains(&mirror) {
                                mirrors.push(mirror);
                            }
                        }

                        let mut s = format!(
                            "{}/{} {} {}",
                            style(pkg.name()).green(),
                            mirrors.join(","),
                            version.version(),
                            pkg.arch()
                        );

                        if let Some(v) = pkg.installed() {
                            let mut is_set = false;
                            if v.version() == version.version() && !pkg.is_upgradable() {
                                s += " [Installed";
                                is_set = true;
                            } else if v.version() == version.version() && pkg.is_upgradable() {
                                s += &format!(" [Upgrade from: {}", v.version());
                                is_set = true;
                            }

                            if pkg.is_auto_installed() && is_set {
                                s += ",automatic]"
                            } else if is_set {
                                s += "]"
                            }
                        }

                        res.push(format!("{}", style(s).bold()));
                    }
                }
            } else {
                for pkg in packages {
                    let mut mirrors = vec![];
                    let versions = pkg.versions().collect::<Vec<_>>();

                    for (i, version) in versions.iter().enumerate() {
                        let uris = version.uris();
                        for i in uris {
                            let mirror = i.split('/').nth_back(3).unwrap_or("unknown").to_owned();
                            if !mirrors.contains(&mirror) {
                                mirrors.push(mirror);
                            }
                        }

                        let mut s = format!(
                            "{}/{} {} {}",
                            style(pkg.name()).green(),
                            mirrors.join(","),
                            version.version(),
                            pkg.arch()
                        );

                        if let Some(v) = pkg.installed() {
                            let mut is_set = false;
                            if v.version() == version.version() && !pkg.is_upgradable() {
                                s += " [Installed";
                                is_set = true;
                            } else if v.version() == version.version() && pkg.is_upgradable() {
                                s += &format!(" [Upgrade from: {}", v.version());
                                is_set = true;
                            }

                            if pkg.is_auto_installed() && is_set {
                                s += ",automatic]"
                            } else if is_set {
                                s += "]"
                            }
                        }

                        if i == versions.len() - 1 {
                            res.push(format!("{}\n", style(s).bold()));
                        } else {
                            res.push(format!("{}", style(s).bold()));
                        }
                    }
                }
            }

            res.sort();

            if res.is_empty() {
                bail!("")
            }

            for i in res {
                println!("{i}");
            }
        } else {
            let mut res = vec![];

            let mut versions_len = 0;

            if let Some(list) = &opt.packages {
                if !opt.all {
                    for i in list {
                        let pkg = cache.get(i);
                        if let Some(pkg) = pkg {
                            versions_len = pkg.versions().collect::<Vec<_>>().len();
                            if let Some(cand) = pkg.candidate() {
                                let pkginfo = PkgInfo::new(&cache, cand.unique(), &pkg)?;

                                res.push(pkginfo);
                            }
                        }
                    }
                } else {
                    for i in list {
                        let pkg = cache.get(i);
                        if let Some(pkg) = pkg {
                            let vers = pkg.versions().collect::<Vec<_>>();
                            for ver in vers {
                                let pkginfo = PkgInfo::new(&cache, ver.unique(), &pkg)?;

                                res.push(pkginfo);
                            }
                        }
                    }
                }
            }

            if res.is_empty() {
                bail!(
                    "Could not find any result for keywords: {}",
                    opt.packages.clone().unwrap_or_default().join(" ")
                );
            }

            for i in res {
                let mut mirror = vec![];
                for j in &i.apt_sources {
                    let branch = j.split('/').nth_back(3).unwrap_or("unknown");
                    if !mirror.contains(&branch) {
                        mirror.push(branch);
                    }
                }

                let pkg = cache.get(&i.package).unwrap();
                let mut s = format!(
                    "{}/{} {} {}",
                    style(&i.package).green(),
                    mirror.join(","),
                    i.version,
                    pkg.arch()
                );
                if let Some(v) = pkg.installed() {
                    let mut is_set = false;
                    if v.version() == i.version && !pkg.is_upgradable() {
                        s += " [Installed";
                        is_set = true;
                    } else if v.version() == i.version && pkg.is_upgradable() {
                        s += &format!(" [Upgrade from: {}", v.version());
                        is_set = true;
                    }

                    if pkg.is_auto_installed() && is_set {
                        s += ",automatic]"
                    } else if is_set {
                        s += "]"
                    }
                }

                println!("{}", style(s).bold());

                if !opt.all && versions_len > 1 {
                    info!(
                        "There is {} additional version. Please use the '-a' switch to see it",
                        versions_len - 1
                    );
                }
            }
        }

        Ok(())
    }

    pub fn show(list: &[String], is_all: bool) -> Result<()> {
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

        Ok(())
    }

    pub fn search(kw: &str) -> Result<()> {
        let cache = new_cache!()?;
        search_pkgs(&cache, kw)?;

        Ok(())
    }

    pub fn list_files(kw: &str) -> Result<()> {
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

        Ok(())
    }

    pub fn search_file(kw: &str) -> Result<()> {
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

        Ok(())
    }

    pub fn fix_broken(&self, v: FixBroken) -> Result<()> {
        needs_root()?;
        lock_oma()?;

        if v.dry_run {
            DRYRUN.store(true, Ordering::Relaxed);
        }

        let start_time = OffsetDateTime::now_utc()
            .to_offset(*TIME_OFFSET)
            .to_string();

        let cache = new_cache!()?;

        let (action, len) = apt_handler(&cache, false, false, false, false)?;

        if len == 0 {
            info!("No need to do anything");
        }

        let mut list = action.install.clone();
        list.extend(action.update.clone());
        list.extend(action.downgrade.clone());

        let install_size = cache.depcache().disk_size();
        size_checker(&install_size, download_size(&list, &cache)?)?;
        if len != 0 {
            display_result(&action, &cache)?;
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

        Ok(())
    }

    pub fn dep(list: &[String], rdep: bool) -> Result<()> {
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

                    let k = {
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

        Ok(())
    }

    pub fn download(&self, v: Download) -> Result<()> {
        if DRYRUN.load(Ordering::Relaxed) {
            return Ok(());
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
            "Successfully downloaded packages: {:?} to path: {}",
            downloads
                .iter()
                .map(|x| x.name_no_color.to_string())
                .collect::<Vec<_>>(),
            path.canonicalize()?.display()
        );

        Ok(())
    }

    fn install_inner(&self, opt: &InstallOptions, count: usize) -> InstallResult<Action> {
        let pkgs = opt.packages.clone().unwrap_or_default();
        let cache = install_handle(&pkgs, opt.install_dbg, opt.reinstall)?;

        let (action, len) = apt_handler(
            &cache,
            opt.no_fixbroken,
            opt.force_yes,
            false,
            opt.no_autoremove,
        )?;

        if len == 0 {
            success!("No need to do anything.");
        }

        let mut list = vec![];
        list.extend(action.install.clone());
        list.extend(action.update.clone());
        list.extend(action.downgrade.clone());
        list.extend(action.reinstall.clone());

        if count == 0 {
            let disk_size = cache.depcache().disk_size();
            size_checker(&disk_size, download_size(&list, &cache)?)?;
            if len != 0 && !opt.yes {
                display_result(&action, &cache)?;
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

    pub fn remove(r: RemoveOptions) -> Result<()> {
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

        let (action, len) =
            apt_handler(&cache, false, r.force_yes, !r.keep_config, r.no_autoremove)?;

        if len == 0 {
            success!("No need to do anything.");
            return Ok(());
        }

        if !r.yes {
            display_result(&action, &cache)?;
        }

        let mut progress = OmaAptInstallProgress::new_box(r.yes, r.force_yes, false, false);

        if !DRYRUN.load(Ordering::Relaxed) {
            cache.commit(&mut NoProgress::new_box(), &mut progress)?;
        }

        let end_time = OffsetDateTime::now_utc()
            .to_offset(*TIME_OFFSET)
            .to_string();

        log_to_file(&action, &start_time, &end_time)?;

        Ok(())
    }

    pub fn refresh(&self) -> Result<()> {
        needs_root()?;
        lock_oma()?;

        update_db_runner(&self.runtime, &get_sources()?, &self.client, None)?;

        let cache = new_cache!()?;
        let upgradable = PackageSort::default().upgradable();
        let autoremove = PackageSort::default().auto_removable();
        let upgradable = cache.packages(&upgradable).collect::<Vec<_>>();
        let autoremove = cache.packages(&autoremove).collect::<Vec<_>>();
        let mut s = String::new();
        if !upgradable.is_empty() {
            s += &format!("{} package can be upgraded", upgradable.len());
        }

        if upgradable.is_empty() && !autoremove.is_empty() {
            s += &format!("{} can be removed.", autoremove.len());
        } else if !upgradable.is_empty() && !autoremove.is_empty() {
            s += &format!(", {} package can be removed.", autoremove.len());
        } else if !upgradable.is_empty() && autoremove.is_empty() {
            s += ".";
        }

        if !upgradable.is_empty() || !autoremove.is_empty() {
            s += " Run 'oma upgrade' to see it."
        }

        if !s.is_empty() {
            info!("{s}");
        }

        Ok(())
    }

    pub fn pick(&self, p: PickOptions) -> Result<()> {
        needs_root()?;
        lock_oma()?;

        if p.dry_run {
            DRYRUN.store(true, Ordering::Relaxed);
        }

        if !p.no_upgrade {
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
                return Ok(());
            }

            version.set_candidate();

            pkg.mark_install(true, true);
            pkg.protect();

            let (action, _) = apt_handler(&cache, p.no_fixbroken, false, false, p.no_autoremove)?;
            let disk_size = cache.depcache().disk_size();

            let mut list = vec![];
            list.extend(action.install.clone());
            list.extend(action.update.clone());
            list.extend(action.downgrade.clone());

            size_checker(&disk_size, download_size(&list, &cache)?)?;
            display_result(&action, &cache)?;

            packages_download_runner(&self.runtime, &list, &self.client, None, None)?;

            apt_install(cache, false, false, false, false)?;

            let end_time = OffsetDateTime::now_utc().to_offset(*TIME_OFFSET);

            let end_time = end_time.to_string();

            log_to_file(&action, &start_time, &end_time)?;
        }

        Ok(())
    }

    pub fn mark(opt: Mark) -> Result<()> {
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

                return Ok(());
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

                return Ok(());
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
                    return Ok(());
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
                    return Ok(());
                }

                cache.commit(
                    &mut NoProgress::new_box(),
                    &mut AptInstallProgress::new_box(),
                )?;
            }
        }

        Ok(())
    }

    pub fn command_not_found(kw: &str) -> Result<()> {
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
                let pkginfo = PkgInfo::new(&cache, version.unique(), &p)?;
                let pkg_str = pkg_str.replace(": ", " (") + ")";
                let s = format!(
                    "{pkg_str}: {}",
                    pkginfo.description.unwrap_or("".to_string())
                );
                if !res.contains(&s) {
                    res.push(s);
                }
            }
        }

        let start = if !res.is_empty() {
            style(format!(
                "Command not found: {kw}, But find some result from package mirror:\n"
            ))
            .bold()
        } else {
            style(format!("Can not find result for command: {kw}")).bold()
        };

        println!("{start}");

        for i in res {
            println!("{i}");
        }

        Ok(())
    }

    pub fn clean() -> Result<()> {
        if DRYRUN.load(Ordering::Relaxed) {
            return Ok(());
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

        Ok(())
    }

    pub fn log() -> Result<()> {
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

        Ok(())
    }
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

    let get_archives_done = Arc::new(AtomicBool::new(false));
    let get_archives_done_clone = get_archives_done.clone();

    let checker = std::thread::spawn(move || {
        let pb = ProgressBar::new_spinner();
        pb.set_message("Verifying the integrity of packages ...");
        oma_spinner(&pb);

        while !get_archives_done_clone.load(Ordering::Relaxed) {}
        pb.finish_and_clear();
    });

    cache.get_archives(&mut NoProgress::new_box())?;
    get_archives_done.store(true, Ordering::Relaxed);

    checker
        .join()
        .expect("Can not wait get archives done, Check your environment?");

    apt_unlock_inner();

    let mut progress =
        OmaAptInstallProgress::new_box(yes, force_yes, force_confnew, dpkg_force_all);

    if let Err(e) = cache.do_install(&mut progress) {
        apt_lock_inner()?;
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
fn install_handle(list: &[String], install_dbg: bool, reinstall: bool) -> Result<Cache> {
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

    for (pkginfo, is_cand) in pkgs {
        if !is_cand {
            continue;
        }

        let pkg = Package::new(&cache, pkginfo.raw_pkg);
        let version = Version::new(pkginfo.version_raw, &pkg);

        mark_install(&cache, &pkginfo.package, version.unique(), reinstall, false)?;

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
            )?;
        } else if install_dbg && !pkginfo.has_dbg {
            warn!("{} has no debug symbol package!", &pkginfo.package);
        }
    }

    Ok(cache)
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
) -> Result<(Action, usize)> {
    if force_yes {
        let config = Config::new_clear();
        config.set("APT::Get::force-yes", "true");
    }

    let fix_broken = !no_fixbroken;
    if fix_broken {
        cache.fix_broken();
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
                let mut va = vec![];
                for (i, c) in v.iter().enumerate() {
                    if i == 0 {
                        let mut v = c.to_string();
                        v.get_mut(0..1).map(|s| {
                            s.make_ascii_uppercase();
                            &*s
                        });
                        va.push(v);
                    } else {
                        va.push(c.to_string());
                    }
                }

                if va.len() == 1 {
                    va[0].to_owned()
                } else {
                    va.join("; ")
                }
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

    Ok((action, len))
}
