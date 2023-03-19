use anyhow::{anyhow, bail, Context, Result};
use apt_sources_lists::SourceEntry;
use clap::{Parser, Subcommand};
use console::{style, Color};
use debarchive::Archive;
use dialoguer::{theme::ColorfulTheme, Select};
use indicatif::HumanBytes;
use reqwest::Client;
use rust_apt::{
    cache::{Cache, PackageSort, Upgrade},
    config::Config,
    new_cache,
    package::Version,
    raw::{package::RawVersion, progress::AptInstallProgress, util::raw::apt_lock_inner},
    records::RecordField,
    util::{apt_lock, apt_unlock, apt_unlock_inner, DiskSpace, Exception},
};
use std::fmt::Write as FmtWrite;
use sysinfo::{Pid, System, SystemExt};
use tabled::{
    object::{Columns, Segment},
    Alignment, Modify, Style, Table, Tabled,
};

use std::{
    fmt::Debug,
    io::{Read, Write},
    path::Path,
    process::{Command, Stdio},
    str::FromStr,
    sync::atomic::Ordering,
};

use crate::{
    contents::find,
    db::{get_sources, update_db, APT_LIST_DISTS, DOWNLOAD_DIR},
    download::packages_download,
    formatter::{NoProgress, OmaAptInstallProgress},
    info,
    pager::Pager,
    pkg::{query_pkgs, search_pkgs, PkgInfo},
    success,
    utils::size_checker,
    warn, InstallOptions, PickOptions, RemoveOptions, UpgradeOptions, ALLOWCTRLC, WRITER,
};

#[derive(Tabled, Debug, Clone)]
struct RemoveRow {
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

#[derive(Subcommand, Debug, Clone)]
pub enum MarkAction {
    /// Hold package version
    Hold(MarkActionArgs),
    /// Unhold package version
    Unhold(MarkActionArgs),
    /// Set package status to manual install
    Manual(MarkActionArgs),
    /// Set package status to auto install
    Auto(MarkActionArgs),
}

#[derive(Parser, Debug, Clone)]
pub struct MarkActionArgs {
    pub pkgs: Vec<String>,
}

const LOCK: &str = "/run/lock/oma.lock";

pub struct OmaAction {
    sources: Vec<SourceEntry>,
    client: Client,
}

#[derive(thiserror::Error, Debug)]
enum InstallError {
    #[error(transparent)]
    Anyhow(#[from] anyhow::Error),
    #[error(transparent)]
    RustApt(#[from] rust_apt::util::Exception),
}

type InstallResult<T> = std::result::Result<T, InstallError>;

impl OmaAction {
    pub async fn new() -> Result<Self> {
        let client = reqwest::ClientBuilder::new().user_agent("oma").build()?;

        let sources = get_sources()?;

        std::fs::create_dir_all(APT_LIST_DISTS)?;
        std::fs::create_dir_all("/var/cache/apt/archives")?;

        Ok(Self { sources, client })
    }

    /// Update mirror database and Get all update, like apt update && apt full-upgrade
    pub async fn update(&self, u: UpgradeOptions) -> Result<()> {
        is_root()?;
        lock_oma()?;

        if u.yes {
            yes_warn();
        }

        update_db(&self.sources, &self.client, None).await?;

        async fn update_inner(
            client: &Client,
            count: usize,
            u: &UpgradeOptions,
        ) -> InstallResult<Action> {
            let cache = install_handle(&u.packages, false, false)?;

            cache
                .upgrade(&Upgrade::FullUpgrade)
                .map_err(|e| anyhow!("{e}"))?;

            let (action, len) = apt_handler(
                &cache,
                false,
                u.force_yes,
                false,
                LogAction::Upgrade(u.clone()),
            )?;

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

            packages_download(&list, client, None, None).await?;
            apt_install(cache, u.yes, u.force_yes, u.force_confnew)?;

            Ok(action)
        }

        let mut count = 0;
        loop {
            match update_inner(&self.client, count, &u).await {
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
                Ok(v) => log(&v)?,
            }
        }
    }

    pub async fn install(&self, opt: InstallOptions) -> Result<()> {
        is_root()?;
        lock_oma()?;

        if opt.yes {
            yes_warn();
        }

        if !opt.no_upgrade {
            update_db(&self.sources, &self.client, None).await?;
        }

        let mut count = 0;
        loop {
            match self.install_inner(&opt, count).await {
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
                Ok(v) => log(&v)?,
            }
        }
    }

    pub fn list(
        list: Option<&[String]>,
        all: bool,
        installed: bool,
        upgradable: bool,
    ) -> Result<()> {
        let cache = new_cache!()?;

        let mut sort = PackageSort::default();

        if installed {
            sort = sort.installed();
        }

        if upgradable {
            sort = sort.upgradable();
        }

        let packages = cache.packages(&sort);

        let mut res = vec![];

        if list.is_none() {
            if !all {
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

            if let Some(list) = list {
                if !all {
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
                    list.unwrap_or_default().join(" ")
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

                if !all && versions_len > 1 {
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

    pub async fn fix_broken(&self) -> Result<()> {
        is_root()?;
        lock_oma()?;

        let cache = new_cache!()?;

        let (action, _) = apt_handler(&cache, false, false, false, LogAction::FixBroken)?;

        let mut list = action.install.clone();
        list.extend(action.update.clone());
        list.extend(action.downgrade.clone());

        let install_size = cache.depcache().disk_size();
        size_checker(&install_size, download_size(&list, &cache)?)?;

        display_result(&action, &cache)?;
        packages_download(&list, &self.client, None, None).await?;

        cache.commit(
            &mut NoProgress::new_box(),
            &mut AptInstallProgress::new_box(),
        )?;

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
                        s = or_str;
                    }

                    println!("{s}");
                }
            }
        }

        Ok(())
    }

    pub async fn download(&self, list: &[String]) -> Result<()> {
        let cache = new_cache!()?;

        let mut downloads = vec![];
        for i in list {
            let oma_pkg = query_pkgs(&cache, i)?;
            for (i, _) in oma_pkg {
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

        packages_download(&downloads, &self.client, None, Some(Path::new("."))).await?;

        Ok(())
    }

    async fn install_inner(&self, opt: &InstallOptions, count: usize) -> InstallResult<Action> {
        let cache = install_handle(&opt.packages, opt.install_dbg, opt.reinstall)?;

        let (action, len) = apt_handler(
            &cache,
            opt.no_fixbroken,
            opt.force_yes,
            false,
            LogAction::Install(opt.clone()),
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
        packages_download(&list, &self.client, None, None).await?;
        apt_install(cache, opt.yes, opt.force_yes, opt.force_confnew)?;

        Ok(action)
    }

    pub fn remove(r: RemoveOptions) -> Result<()> {
        is_root()?;
        lock_oma()?;

        if r.yes {
            yes_warn();
        }

        let cache = new_cache!()?;

        for i in &r.packages {
            let pkg = cache.get(&i).context(format!("Can not get package {i}"))?;
            if !pkg.is_installed() {
                info!("Package {i} is not installed, so no need to remove.");
                continue;
            }
            pkg.mark_delete(!r.keep_config);
            pkg.protect();
        }

        let (action, len) = apt_handler(
            &cache,
            false,
            r.force_yes,
            !r.keep_config,
            LogAction::Remove(r.clone()),
        )?;

        if len == 0 {
            success!("No need to do anything.");
            return Ok(());
        }

        if !r.yes {
            display_result(&action, &cache)?;
        }

        let mut progress = OmaAptInstallProgress::new_box(r.yes, r.force_yes, false);

        cache.commit(&mut NoProgress::new_box(), &mut progress)?;

        log(&action)?;

        Ok(())
    }

    pub async fn refresh(&self) -> Result<()> {
        is_root()?;
        lock_oma()?;

        update_db(&self.sources, &self.client, None).await?;

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

    pub async fn pick(&self, p: PickOptions) -> Result<()> {
        is_root()?;
        lock_oma()?;

        if !p.no_upgrade {
            update_db(&self.sources, &self.client, None).await?;
        }

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

        let theme = ColorfulTheme::default();
        let mut dialoguer = Select::with_theme(&theme);

        dialoguer.items(&versions_str_display);
        dialoguer.with_prompt(format!("Select {} version:", pkg.name()));

        if let Some(installed) = installed {
            let pos = versions_str.iter().position(|x| x == installed.version());
            if let Some(pos) = pos {
                dialoguer.default(pos);
            }
        }

        let index = dialoguer.interact();

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

            let (action, _) = apt_handler(
                &cache,
                p.no_fixbroken,
                false,
                false,
                LogAction::Pick(p.clone()),
            )?;
            let disk_size = cache.depcache().disk_size();

            let mut list = vec![];
            list.extend(action.install.clone());
            list.extend(action.update.clone());
            list.extend(action.downgrade.clone());

            size_checker(&disk_size, download_size(&list, &cache)?)?;
            display_result(&action, &cache)?;

            packages_download(&list, &self.client, None, None).await?;

            apt_install(cache, false, false, false)?;
        }

        Ok(())
    }

    pub fn mark(user_action: MarkAction) -> Result<()> {
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

        match user_action {
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
        is_root()?;

        let dir = std::fs::read_dir(DOWNLOAD_DIR)?;

        for i in dir.flatten() {
            if i.path().extension().and_then(|x| x.to_str()) == Some("deb") {
                std::fs::remove_file(i.path()).ok();
            }
        }

        let p = Path::new(DOWNLOAD_DIR).join("..");

        std::fs::remove_file(p.join("pkgcache.bin")).ok();
        std::fs::remove_file(p.join("srcpkgcache.bin")).ok();

        success!("Clean successfully.");

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

    cmd.stdin
        .as_mut()
        .unwrap()
        .write_all(format!("{pkg} {user_action}").as_bytes())?;

    cmd.wait()?;

    Ok(())
}

fn log(action: &Action) -> Result<()> {
    std::fs::create_dir_all("/var/log/oma")?;

    let mut f = std::fs::OpenOptions::new()
        .append(true)
        .create(true)
        .open("/var/log/oma/history")?;

    let _ = match &action.op {
        LogAction::Install(v) => {
            f.write_all(format!("Action: {v:?}\nResolver: {action:?}\n\n").as_bytes())
        }
        LogAction::Upgrade(v) => {
            f.write_all(format!("Action: {v:?}\nResolver: {action:?}\n\n").as_bytes())
        }
        LogAction::Remove(v) => {
            f.write_all(format!("Action: {v:?}\nResolver: {action:?}\n\n").as_bytes())
        }
        LogAction::FixBroken => {
            f.write_all(format!("Action: oma fix-broken\nResolver: {action:?}\n\n").as_bytes())
        }
        LogAction::Pick(v) => {
            f.write_all(format!("Action: {v:?}\nResolver: {action:?}\n\n").as_bytes())
        }
    };

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

/// Check user is root
fn is_root() -> Result<()> {
    if !nix::unistd::geteuid().is_root() {
        bail!("Please run me as root!");
    }

    Ok(())
}

/// apt autoremove
fn autoremove(cache: &Cache, is_purge: bool) {
    let sort = PackageSort::default();

    for pkg in cache.packages(&sort) {
        if pkg.is_auto_removable() {
            pkg.mark_delete(is_purge);
            pkg.protect();
        }
    }
}

/// Install packages
fn apt_install(
    cache: Cache,
    yes: bool,
    force_yes: bool,
    force_confnew: bool,
) -> std::result::Result<(), Exception> {
    apt_lock()?;
    cache.get_archives(&mut NoProgress::new_box())?;
    apt_unlock_inner();

    let mut progress = OmaAptInstallProgress::new_box(yes, force_yes, force_confnew);

    if let Err(e) = cache.do_install(&mut progress) {
        apt_lock_inner()?;
        apt_unlock();
        return Err(e);
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

    let mut local = vec![];

    for i in &local_debs {
        let archive = Archive::new(Path::new(i)).context(format!("Can not read file: {i}"))?;
        let control = archive
            .control_map()
            .map_err(|e| anyhow!("Can not get archive {i} control file, Why: {e}"))?;

        local.push((
            control
                .get("Package")
                .context("Can not get package name from file: {i}")?
                .clone(),
            format!(
                "file:{}",
                Path::new(i)
                    .canonicalize()?
                    .to_str()
                    .context(format!("Can not convert path {i} to str"))?
            ),
        ));
    }

    let another = list.iter().filter(|x| !local_debs.contains(x));
    let cache = new_cache!(&local_debs)?;
    let mut pkgs = vec![];

    for i in another {
        let i_res = query_pkgs(&cache, i)?;
        if i_res.is_empty() {
            bail!("Package {i} does not exist.");
        }
        pkgs.extend(i_res);
    }

    // install local packages
    for (pkg, path) in local {
        let pkg = cache.get(&pkg).unwrap();
        let pkgname = pkg.name();
        let versions = pkg.versions().collect::<Vec<_>>();
        let version = versions.iter().find(|x| x.uris().any(|x| x == path));
        let version = version.unwrap();

        mark_install(&cache, pkgname, version.unique(), reinstall, true)?;
    }

    // install another package
    for (pkginfo, is_cand) in pkgs {
        if !is_cand {
            continue;
        }

        let pkg = cache.get(&pkginfo.package).unwrap();
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

/// Mark package as install status
fn mark_install(
    cache: &Cache,
    pkg: &str,
    ver: RawVersion,
    reinstall: bool,
    is_local: bool,
) -> Result<()> {
    let pkg = cache.get(pkg).unwrap();
    let ver = Version::new(ver, &pkg);
    ver.set_candidate();

    let version = ver.version();

    if pkg.installed().as_ref() == Some(&ver) && !reinstall {
        info!("{} {version} is already installed.", pkg.name());
        return Ok(());
    } else if pkg.installed().as_ref() == Some(&ver) && reinstall {
        pkg.mark_reinstall(true);
    } else {
        pkg.mark_install(true, true);
        if !pkg.marked_install() && !pkg.marked_downgrade() && !pkg.marked_upgrade() {
            // apt 会先就地检查这个包的表面依赖是否满足要求，如果不满足则直接返回错误，而不是先交给 resolver
            bail!(
                "{} can't marked installed! maybe dependency issue?",
                if is_local {
                    ver.uris().next().unwrap_or(pkg.name().to_string())
                } else {
                    pkg.name().to_string()
                }
            );
        }
    }

    pkg.protect();

    Ok(())
}

#[derive(Clone)]
struct Action {
    op: LogAction,
    update: Vec<InstallRow>,
    install: Vec<InstallRow>,
    del: Vec<RemoveRow>,
    reinstall: Vec<InstallRow>,
    downgrade: Vec<InstallRow>,
}

#[derive(Clone, Debug)]
enum LogAction {
    Install(InstallOptions),
    Upgrade(UpgradeOptions),
    Remove(RemoveOptions),
    FixBroken,
    Pick(PickOptions),
}

// impl Debug for LogAction {
//     fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
//         match self {
//             Self::Install(arg0) => {
//                 write!(f, "oma install ")?;

//                 let mut v = vec![];
//                 if arg0.yes {
//                     v.push("--yes".to_owned())
//                 }
//                 if arg0.force_confnew {
//                     v.push("--force-confnew".to_owned())
//                 }
//                 if arg0.no_fixbroken {
//                     v.push("--no-fixbroken".to_owned())
//                 }
//                 if arg0.install_dbg {
//                     v.push("--install-dbg".to_owned())
//                 }
//                 if arg0.no_upgrade {
//                     v.push("--no-upgrade".to_owned())
//                 }
//                 if arg0.reinstall {
//                     v.push("--reinstall".to_owned())
//                 }
//                 v.extend(arg0.packages.clone());

//                 write!(f, "{}", v.join(" "))?;
//             }
//             Self::Upgrade(arg0) => {
//                 write!(f, "oma upgrade ")?;

//                 let mut v = vec![];
//                 if arg0.force_confnew {
//                     v.push("--force-confnew".to_owned())
//                 }
//                 if arg0.force_yes {
//                     v.push("--force-yes".to_owned())
//                 }
//                 v.extend(arg0.packages.clone());
//                 write!(f, "{}", v.join(" "))?;
//             }
//             Self::Remove(arg0) => {
//                 write!(f, "oma remove ")?;

//                 let mut v = vec![];
//                 if arg0.keep_config {
//                     v.push("--keep-config".to_owned())
//                 }
//                 if arg0.force_yes {
//                     v.push("--force-yes".to_owned())
//                 }
//                 v.extend(arg0.packages.clone());
//                 write!(f, "{}", v.join(" "))?;
//             }
//             Self::FixBroken => {
//                 write!(f, "{}", "oma fix-broken")?;
//             }
//             Self::Pick(arg0) => {
//                 write!(f, "oma pick ")?;

//                 let mut v = vec![];
//                 if arg0.no_fixbroken {
//                     v.push("--no-fixbroken")
//                 }
//                 if arg0.no_upgrade {
//                     v.push("--no-upgrade")
//                 }
//                 v.push(&arg0.package);
//                 write!(f, "{}", v.join(" "))?;
//             }
//         };

//         Ok(())
//     }
// }

impl Action {
    fn new(
        op: LogAction,
        update: Vec<InstallRow>,
        install: Vec<InstallRow>,
        del: Vec<RemoveRow>,
        reinstall: Vec<InstallRow>,
        downgrade: Vec<InstallRow>,
    ) -> Self {
        Self {
            op,
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
        write!(f, "Install: [")?;
        for i in &self.install {
            write!(f, "{}({}), ", i.name_no_color, i.version)?;
        }
        write!(f, "]")?;

        write!(f, ", Remove:[")?;
        for i in &self.del {
            write!(f, "{}, ", i._name_no_color)?;
        }
        write!(f, "]")?;

        write!(f, ", Upgrade: [")?;
        for i in &self.update {
            write!(f, "{}({}), ", i.name_no_color, i.version)?;
        }
        write!(f, "]")?;

        write!(f, ", Downgrade: [")?;
        for i in &self.downgrade {
            write!(f, "{}({}), ", i.name_no_color, i.version)?;
        }
        write!(f, "]")?;

        Ok(())
    }
}

/// Handle apt resolve result to display results
fn apt_handler(
    cache: &Cache,
    no_fixbroken: bool,
    force_yes: bool,
    is_purge: bool,
    op: LogAction,
) -> Result<(Action, usize)> {
    if force_yes {
        let config = Config::new_clear();
        config.set("APT::Get::force-yes", "true");
    }

    let fix_broken = !no_fixbroken;
    if fix_broken {
        cache.fix_broken();
    }
    cache.resolve(fix_broken)?;
    autoremove(cache, is_purge);
    cache.resolve(fix_broken)?;

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
        }

        if pkg.marked_delete() {
            let name = pkg.name();

            let is_purge = pkg.marked_purge();

            let mut v = Vec::new();

            if pkg.is_auto_removable() {
                v.push((
                    "removed as unneeded dependency",
                    "Removed as unneeded dependency",
                ));
            }

            if is_purge {
                v.push(("purge configuration files", "Purge configuration files"));
            }

            let s = if !v.is_empty() {
                let mut va = vec![];
                for (i, c) in v.iter().enumerate() {
                    if i == 0 {
                        va.push(c.1);
                    } else {
                        va.push(c.0);
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
        }
    }

    let action = Action::new(op, update, install, del, reinstall, downgrade);

    Ok((action, len))
}

/// Display apt resolver results
fn display_result(action: &Action, cache: &Cache) -> Result<()> {
    let update = action.update.clone();
    let install = action.install.clone();
    let del = action.del.clone();
    let reinstall = action.reinstall.clone();
    let downgrade = action.downgrade.clone();

    let mut pager = Pager::new(false, true)?;
    let pager_name = pager.pager_name().to_owned();
    let mut out = pager.get_writer()?;

    write_review_help_message(&mut out)?;

    if pager_name == Some("less") {
        let has_x11 = std::env::var("DISPLAY");

        if has_x11.is_ok() {
            let line1 = "    Press [q] to end review";
            let line2 = "    Press [Ctrl-c] to abort";
            let line3 = "    Press [PgUp/Dn], arrow keys, or use the mouse wheel to scroll.\n";

            writeln!(out, "{}", style(line1).bold())?;
            writeln!(out, "{}", style(line2).bold())?;
            writeln!(out, "{}", style(line3).bold())?;
        } else {
            let line1 = "    Press [q] to end review";
            let line2 = "    Press [Ctrl-c] to abort";
            let line3 = "    Press [PgUp/Dn] or arrow keys to scroll.\n";

            writeln!(out, "{}", style(line1).bold())?;
            writeln!(out, "{}", style(line2).bold())?;
            writeln!(out, "{}", style(line3).bold())?;
        }
    }

    if !del.is_empty() {
        writeln!(
            out,
            "{} packages will be {}:\n",
            del.len(),
            style("REMOVED").red().bold()
        )?;

        let mut table = Table::new(del);

        table
            .with(Modify::new(Segment::all()).with(Alignment::left()))
            .with(Modify::new(Columns::new(1..2)).with(Alignment::left()))
            .with(Modify::new(Segment::all()).with(|s: &str| format!(" {s} ")))
            .with(Style::psql());

        writeln!(out, "{table}")?;
    }

    if !install.is_empty() {
        writeln!(
            out,
            "\n{} packages will be {}:\n",
            install.len(),
            style("installed").green().bold()
        )?;

        let mut table = Table::new(&install);

        table
            .with(Modify::new(Segment::all()).with(Alignment::left()))
            // Install Size column should align right
            .with(Modify::new(Columns::new(2..3)).with(Alignment::right()))
            .with(Modify::new(Segment::all()).with(|s: &str| format!(" {s} ")))
            .with(Style::psql());

        writeln!(out, "{table}")?;
    }

    if !update.is_empty() {
        writeln!(
            out,
            "\n{} packages will be {}:\n",
            update.len(),
            style("upgraded").color256(87)
        )?;

        let mut table = Table::new(&update);

        table
            .with(Modify::new(Segment::all()).with(Alignment::left()))
            // Install Size column should align right
            .with(Modify::new(Columns::new(2..3)).with(Alignment::right()))
            .with(Modify::new(Segment::all()).with(|s: &str| format!(" {s} ")))
            .with(Style::psql());

        writeln!(out, "{table}")?;
    }

    if !downgrade.is_empty() {
        writeln!(
            out,
            "\n{} packages will be {}:\n",
            downgrade.len(),
            style("downgraded").yellow().bold()
        )?;

        let mut table = Table::new(&downgrade);

        table
            .with(Modify::new(Segment::all()).with(Alignment::left()))
            // Install Size column should align right
            .with(Modify::new(Columns::new(1..2)).with(Alignment::right()))
            .with(Modify::new(Segment::all()).with(|s: &str| format!(" {s} ")))
            .with(Style::psql());

        writeln!(out, "{table}")?;
    }

    if !reinstall.is_empty() {
        writeln!(
            out,
            "\n{} packages will be {}:\n",
            reinstall.len(),
            style("reinstall").blue().bold()
        )?;

        let mut table = Table::new(&reinstall);

        table
            .with(Modify::new(Segment::all()).with(Alignment::left()))
            // Install Size column should align right
            .with(Modify::new(Columns::new(2..3)).with(Alignment::right()))
            .with(Modify::new(Segment::all()).with(|s: &str| format!(" {s} ")))
            .with(Style::psql());

        writeln!(out, "{table}")?;
    }

    let mut list = vec![];
    list.extend(install);
    list.extend(update);
    list.extend(downgrade);

    writeln!(
        out,
        "\n{} {}",
        style("Total download size:").bold(),
        HumanBytes(download_size(&list, cache)?)
    )?;

    let (symbol, abs_install_size_change) = match cache.depcache().disk_size() {
        DiskSpace::Require(n) => ("+", n),
        DiskSpace::Free(n) => ("-", n),
    };

    writeln!(
        out,
        "{} {}{}",
        style("Estimated change in storage usage:").bold(),
        symbol,
        HumanBytes(abs_install_size_change)
    )?;

    drop(out);
    let success = pager.wait_for_exit()?;

    if success {
        Ok(())
    } else {
        // User aborted the operation
        bail!("")
    }
}

/// Get download size
fn download_size(install_and_update: &[InstallRow], cache: &Cache) -> Result<u64> {
    let mut result = 0;

    for i in install_and_update {
        let pkg = cache
            .get(&i.name_no_color)
            .context(format!("Can not get package {}", i.name_no_color))?;

        let ver = pkg.get_version(&i.new_version).context(format!(
            "Can not get package {} version {}",
            i.name_no_color, i.new_version
        ))?;

        let size = ver.size();

        result += size;
    }

    Ok(result)
}

fn write_review_help_message(w: &mut dyn Write) -> Result<()> {
    writeln!(
        w,
        "{:<80}",
        style("Pending Operations").bold().bg(Color::Color256(25))
    )?;
    writeln!(w)?;
    writeln!(w, "Shown below is an overview of the pending changes Omakase will apply to your\nsystem, please review them carefully\n")?;
    writeln!(
        w,
        "Omakase may {}, {}, {}, {}, or {}\npackages in order to fulfill your requested changes.",
        style("install").green(),
        style("remove").red(),
        style("upgrade").color256(87),
        style("downgrade").yellow(),
        style("reinstall").blue()
    )?;
    writeln!(w)?;

    Ok(())
}

/// Lock oma
fn lock_oma() -> Result<()> {
    let lock = Path::new(LOCK);
    if lock.is_file() {
        let mut lock_file = std::fs::File::open(lock)?;
        let mut old_pid = String::new();
        lock_file.read_to_string(&mut old_pid)?;

        let s = System::new_all();
        let old_pid = Pid::from_str(&old_pid)?;

        if s.process(old_pid).is_some() {
            bail!(
                "Another instance of oma (pid: {}) is still running!",
                old_pid
            );
        } else {
            unlock_oma()?;
        }
    }
    let mut lock_file = std::fs::File::create(lock)?;
    let pid = std::process::id().to_string();

    // Set global lock parameter
    crate::LOCKED.store(true, Ordering::Relaxed);

    lock_file.write_all(pid.as_bytes())?;

    Ok(())
}

/// Unlock oma
pub fn unlock_oma() -> Result<()> {
    if Path::new(LOCK).exists() {
        std::fs::remove_file(LOCK)?;
    }

    Ok(())
}
