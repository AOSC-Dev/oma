use anyhow::{anyhow, bail, Context, Result};
use apt_sources_lists::SourceEntry;
use console::{style, Color};
use debarchive::Archive;
use dialoguer::{theme::ColorfulTheme, Select};
use indicatif::HumanBytes;
use reqwest::Client;
use rust_apt::{
    cache::{Cache, PackageSort, Upgrade},
    new_cache,
    raw::{progress::AptInstallProgress, util::raw::apt_lock_inner},
    records::RecordField,
    util::{apt_lock, apt_unlock, apt_unlock_inner, DiskSpace, Exception},
};
use sysinfo::{Pid, System, SystemExt};
use tabled::{
    object::{Columns, Segment},
    Alignment, Modify, Style, Table, Tabled,
};

use std::{
    io::{Read, Write},
    path::Path,
    process::{Command, Stdio},
    str::FromStr,
    sync::{atomic::Ordering},
};

use crate::{
    contents::find,
    db::{get_sources, packages_download, update_db, APT_LIST_DISTS},
    formatter::NoProgress,
    info,
    pager::Pager,
    search::{search_pkgs, show_pkgs, OmaPkg},
    success,
    utils::size_checker,
    warn,
};

#[derive(Tabled, Debug, Clone)]
struct RemoveRow {
    #[tabled(rename = "Name")]
    name: String,
    #[tabled(skip)]
    _name_no_color: String,
    #[tabled(rename = "Package Size")]
    size: String,
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
    pub async fn update(&self, packages: &[String]) -> Result<()> {
        lock_oma()?;
        is_root()?;

        update_db(&self.sources, &self.client, None).await?;

        async fn update_inner(
            client: &Client,
            count: usize,
            packages: &[String],
        ) -> InstallResult<()> {
            let cache = install_handle(packages, false, false)?;

            cache
                .upgrade(&Upgrade::FullUpgrade)
                .map_err(|e| anyhow!("{e}"))?;

            let (action, len) = apt_handler(&cache)?;

            if len == 0 {
                success!("No need to do anything.");
                return Ok(());
            }

            let mut list = action.update.clone();
            list.extend(action.install.clone());
            list.extend(action.downgrade.clone());

            if count == 0 {
                let disk_size = cache.depcache().disk_size();
                size_checker(&disk_size, download_size(&list, &cache)?)?;
                display_result(&action, &cache, disk_size)?;
            }

            packages_download(&list, client, None, None).await?;
            apt_install(cache)?;

            Ok(())
        }

        let mut count = 0;
        while let Err(e) = update_inner(&self.client, count, packages).await {
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

        Ok(())
    }

    pub async fn install(&self, list: &[String], install_dbg: bool, reinstall: bool) -> Result<()> {
        lock_oma()?;
        is_root()?;
        update_db(&self.sources, &self.client, None).await?;

        let mut count = 0;
        while let Err(e) = self
            .install_inner(list, count, install_dbg, reinstall)
            .await
        {
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

            count += 1;
        }

        Ok(())
    }

    pub fn list(list: Option<&[String]>, all: bool, installed: bool) -> Result<()> {
        let cache = new_cache!()?;

        let mut sort = PackageSort::default();

        if installed {
            sort = sort.installed();
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
                            if v.version() == version.version() && !pkg.is_upgradable() {
                                s += " [Installed]";
                            } else if v.version() == version.version() && pkg.is_upgradable() {
                                s += &format!(" [Upgrade from {}]", v.version());
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
                            if v.version() == version.version() && !pkg.is_upgradable() {
                                s += " [Installed]";
                            } else if v.version() == version.version() && pkg.is_upgradable() {
                                s += &format!(" [Upgrade from {}]", v.version());
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
        }

        res.sort();

        for i in res {
            println!("{i}");
        }

        let mut res = vec![];

        if let Some(list) = list {
            if !all {
                for i in list {
                    let pkgs = show_pkgs(&cache, i)?;
                    res.extend(pkgs);
                }
            } else {
                for i in list {
                    let pkg = cache.get(i);
                    if let Some(pkg) = pkg {
                        let vers = pkg.versions().collect::<Vec<_>>();
                        for ver in vers {
                            let pkginfo = OmaPkg::new(&cache, pkg.name(), ver.version())?;

                            res.push(pkginfo);
                        }
                    }
                }
            }
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
                if v.version() == i.version && !pkg.is_upgradable() {
                    s += " [Installed]";
                } else if v.version() == i.version && pkg.is_upgradable() {
                    s += &format!(" [Upgrade from {}]", v.version());
                }
            }

            println!("{}", style(s).bold());
        }

        Ok(())
    }

    pub fn show(list: &[String]) -> Result<()> {
        let cache = new_cache!()?;

        let mut s = String::new();

        for (i, c) in list.iter().enumerate() {
            let oma_pkg = show_pkgs(&cache, c)?;
            let len = oma_pkg.len();
            for (i, entry) in oma_pkg.into_iter().enumerate() {
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

        Ok(())
    }

    pub fn search(kw: &str) -> Result<()> {
        let cache = new_cache!()?;
        search_pkgs(&cache, kw)?;

        Ok(())
    }

    pub fn list_files(kw: &str) -> Result<()> {
        let res = find(kw, true, false)?;

        for (_, line) in res {
            println!("{line}");
        }

        Ok(())
    }

    pub fn search_file(kw: &str) -> Result<()> {
        let res = find(kw, false, false)?;

        for (_, line) in res {
            println!("{line}");
        }

        Ok(())
    }

    pub async fn fix_broken(&self) -> Result<()> {
        lock_oma()?;
        is_root()?;

        let cache = new_cache!()?;

        let (action, _) = apt_handler(&cache)?;

        let mut list = action.install.clone();
        list.extend(action.update.clone());
        list.extend(action.downgrade.clone());

        let install_size = cache.depcache().disk_size();
        size_checker(&install_size, download_size(&list, &cache)?)?;

        display_result(&action, &cache, install_size)?;
        packages_download(&list, &self.client, None, None).await?;

        cache.commit(
            &mut NoProgress::new_box(),
            &mut AptInstallProgress::new_box(),
        )?;

        Ok(())
    }

    pub async fn download(&self, list: &[String]) -> Result<()> {
        let cache = new_cache!()?;

        let mut downloads = vec![];
        for i in list {
            let oma_pkg = show_pkgs(&cache, i)?;
            for i in oma_pkg {
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

    async fn install_inner(
        &self,
        list: &[String],
        count: usize,
        install_dbg: bool,
        reinstall: bool,
    ) -> InstallResult<()> {
        let cache = install_handle(list, install_dbg, reinstall)?;

        let (action, len) = apt_handler(&cache)?;

        if len == 0 {
            success!("No need to do anything.");
            return Ok(());
        }

        let mut list = vec![];
        list.extend(action.install.clone());
        list.extend(action.update.clone());
        list.extend(action.downgrade.clone());
        list.extend(action.reinstall.clone());

        if count == 0 {
            let disk_size = cache.depcache().disk_size();
            size_checker(&disk_size, download_size(&list, &cache)?)?;
            display_result(&action, &cache, disk_size)?;
        }

        // TODO: limit 参数（限制下载包并发）目前是写死的，以后将允许用户自定义并发数
        packages_download(&list, &self.client, None, None).await?;
        apt_install(cache)?;

        Ok(())
    }

    pub fn remove(&self, list: &[String], is_purge: bool) -> Result<()> {
        lock_oma()?;
        is_root()?;

        let cache = new_cache!()?;

        for i in list {
            let pkg = cache.get(i).context(format!("Can not get package {i}"))?;
            pkg.mark_delete(is_purge);
            pkg.protect();
        }

        let (action, len) = apt_handler(&cache)?;

        if len == 0 {
            success!("No need to do anything.");
            return Ok(());
        }

        let disk_size = cache.depcache().disk_size();
        display_result(&action, &cache, disk_size)?;

        cache.commit(
            &mut NoProgress::new_box(),
            &mut AptInstallProgress::new_box(),
        )?;

        Ok(())
    }

    pub async fn refresh(&self) -> Result<()> {
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

    pub async fn pick(&self, pkg: &str) -> Result<()> {
        lock_oma()?;
        is_root()?;

        let cache = new_cache!()?;
        let pkg = cache
            .get(pkg)
            .context(format!("Can not get package: {pkg}"))?;

        let versions = pkg
            .versions()
            .map(|x| x.version().to_string())
            .collect::<Vec<_>>();

        let installed = pkg.installed();

        let theme = ColorfulTheme::default();
        let mut dialoguer = Select::with_theme(&theme);

        dialoguer.items(&versions);
        dialoguer.with_prompt(format!("Select {} version:", pkg.name()));

        if let Some(installed) = installed {
            let pos = versions.iter().position(|x| x == installed.version());
            if let Some(pos) = pos {
                dialoguer.default(pos);
            }
        }

        let index = dialoguer.interact()?;

        let version = &versions[index];
        let version = pkg.get_version(version).unwrap();

        if pkg.installed().as_ref() == Some(&version) {
            success!("No need to do anything.");
            return Ok(());
        }

        version.set_candidate();

        pkg.mark_install(true, true);
        pkg.protect();

        let (action, _) = apt_handler(&cache)?;
        let disk_size = cache.depcache().disk_size();

        let mut list = vec![];
        list.extend(action.install.clone());
        list.extend(action.update.clone());
        list.extend(action.downgrade.clone());

        size_checker(&disk_size, download_size(&list, &cache)?)?;
        display_result(&action, &cache, disk_size)?;

        packages_download(&list, &self.client, None, None).await?;

        apt_install(cache)?;

        Ok(())
    }

    pub fn mark(pkg: &str, user_action: &str) -> Result<()> {
        let cache = new_cache!()?;

        let package = cache.get(pkg);

        if package.is_none() {
            bail!("Can not get package {pkg}")
        }

        let package = package.unwrap();

        package
            .installed()
            .context(format!("{pkg} is not installed!"))?;

        match user_action {
            "hold" => {
                let selections = dpkg_selections()?;

                let v = selections
                    .iter()
                    .find(|x| x.0 == pkg)
                    .context("dpkg data is broken!")?;

                if v.1 == "hold" {
                    info!("{pkg} is already hold!");
                    return Ok(());
                }

                dpkg_set_selections(pkg, user_action)?;
                success!("{pkg} is set to hold.");

                return Ok(());
            }
            "unhold" => {
                let selections = dpkg_selections()?;

                let v = selections
                    .iter()
                    .find(|x| x.0 == pkg)
                    .context("dpkg data is broken!")?;

                if v.1 == "install" {
                    info!("{pkg} is already unhold!");
                    return Ok(());
                }

                dpkg_set_selections(pkg, user_action)?;
                success!("{pkg} is set to unhold.");

                return Ok(());
            }
            "manual" => {
                if !package.is_auto_installed() {
                    info!("{pkg} is already manual installed status.");
                    return Ok(());
                }
                info!("Setting {pkg} to manual installed status ...");
                package.mark_auto(false);
            }
            "auto" => {
                if package.is_auto_installed() {
                    info!("{pkg} is already auto installed status.");
                    return Ok(());
                }
                info!("Setting {pkg} to auto installed status ...");
                package.mark_auto(true);
            }
            _ => {
                bail!("Unsupport action: {user_action}");
            }
        }

        cache.commit(
            &mut NoProgress::new_box(),
            &mut AptInstallProgress::new_box(),
        )?;

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
                let pkginfo = OmaPkg::new(&cache, &pkg, version.version())?;
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
}

fn dpkg_set_selections(pkg: &str, user_action: &str) -> Result<()> {
    let cmd = Command::new("dpkg")
        .arg("--set-selections")
        .stdin(Stdio::piped())
        .spawn()?;

    let user_action = if user_action == "unhold" {
        "install"
    } else {
        user_action
    };

    cmd.stdin
        .unwrap()
        .write_all(format!("{pkg} {user_action}").as_bytes())?;

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

fn is_root() -> Result<()> {
    if !nix::unistd::geteuid().is_root() {
        bail!("Please run me as root!");
    }

    Ok(())
}

fn autoremove(cache: &Cache) {
    let sort = PackageSort::default();

    for pkg in cache.packages(&sort) {
        if pkg.is_auto_removable() {
            pkg.mark_delete(true);
            pkg.protect();
        }
    }
}

fn apt_install(cache: Cache) -> std::result::Result<(), Exception> {
    apt_lock()?;
    cache.get_archives(&mut NoProgress::new_box())?;
    apt_unlock_inner();

    if let Err(e) = cache.do_install(&mut AptInstallProgress::new_box()) {
        apt_lock_inner()?;
        apt_unlock();
        return Err(e);
    }

    apt_unlock();

    Ok(())
}

fn install_handle(list: &[String], install_dbg: bool, reinstall: bool) -> Result<Cache> {
    // Get local packages
    let local_debs = list
        .iter()
        .filter(|x| x.ends_with(".deb"))
        .collect::<Vec<_>>();

    let mut local = vec![];

    for i in &local_debs {
        let archive = Archive::new(Path::new(i)).context("Can not read file: {i}")?;
        let control = archive.control_map()?;
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
        pkgs.extend(show_pkgs(&cache, i)?);
    }

    // install local packages
    for (pkg, path) in local {
        let pkg = cache.get(&pkg).unwrap();
        let pkgname = pkg.name();
        let versions = pkg.versions().collect::<Vec<_>>();

        let version = versions
            .iter()
            .find(|x| x.uris().any(|x| x == path.to_owned()));

        let version = version.unwrap();

        mark_install(&cache, pkgname, version.version(), reinstall, true)?;
    }

    // install another package
    for pkginfo in pkgs {
        mark_install(&cache, &pkginfo.package, &pkginfo.version, reinstall, false)?;

        if install_dbg && pkginfo.has_dbg {
            mark_install(
                &cache,
                &format!("{}-dbg", pkginfo.package),
                &pkginfo.version,
                reinstall,
                false,
            )?;
        } else if install_dbg && !pkginfo.has_dbg {
            warn!("{} has no debug symbol package!", pkginfo.package);
        }
    }

    Ok(cache)
}

fn mark_install(
    cache: &Cache,
    pkg: &str,
    version: &str,
    reinstall: bool,
    is_local: bool,
) -> Result<()> {
    let pkg = cache.get(pkg).unwrap();
    let ver = pkg.get_version(version).unwrap();

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

#[derive(Debug, Clone)]
struct Action {
    update: Vec<InstallRow>,
    install: Vec<InstallRow>,
    del: Vec<RemoveRow>,
    reinstall: Vec<InstallRow>,
    downgrade: Vec<InstallRow>,
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

fn apt_handler(cache: &Cache) -> Result<(Action, usize)> {
    cache.resolve(true)?;
    autoremove(cache);
    cache.resolve(true)?;

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
                name: style(pkg.name()).cyan().to_string(),
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

            let old_pkg = pkg
                .installed()
                .context(format!("Can not get installed version: {}", pkg.name()))?;

            let size = old_pkg.installed_size();
            let is_purge = true;

            let remove_row = RemoveRow {
                name: style(name).red().bold().to_string(),
                _name_no_color: name.to_owned(),
                size: HumanBytes(size).to_string(),
                detail: if is_purge {
                    style("Purge configuration files.").red().to_string()
                } else {
                    "".to_string()
                },
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

    let action = Action::new(update, install, del, reinstall, downgrade);

    Ok((action, len))
}

fn display_result(action: &Action, cache: &Cache, disk_size: DiskSpace) -> Result<()> {
    let update = action.update.clone();
    let install = action.install.clone();
    let del = action.del.clone();
    let reinstall = action.reinstall.clone();
    let downgrade = action.downgrade.clone();

    let mut pager = Pager::new(false)?;
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
            // Install Size column should align right
            .with(Modify::new(Columns::new(1..2)).with(Alignment::right()))
            .with(Modify::new(Segment::all()).with(|s: &str| format!(" {s} ")))
            .with(Style::psql());

        writeln!(out, "{table}")?;
    }

    if !install.is_empty() {
        writeln!(
            out,
            "{} packages will be {}:\n",
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
            style("upgraded").green().bold()
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
            "{} packages will be {}:\n",
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

    let (symbol, abs_install_size_change) = match disk_size {
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
        style("upgrade").cyan(),
        style("downgrade").yellow(),
        style("reinstall").blue()
    )?;
    writeln!(w)?;

    Ok(())
}

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

pub fn unlock_oma() -> Result<()> {
    if Path::new(LOCK).exists() {
        std::fs::remove_file(LOCK)?;
    }

    Ok(())
}
