use std::path::Path;

use anyhow::{bail, Context, Ok, Result};
use apt_sources_lists::SourceEntry;
use console::style;
use indexmap::IndexMap;
use indicatif::HumanBytes;
use reqwest::Client;
use rust_apt::{
    cache::{Cache, PackageSort, Upgrade},
    new_cache,
    raw::{progress::AptInstallProgress, util::raw::apt_lock_inner},
    util::{apt_lock, apt_unlock, apt_unlock_inner, unit_str, DiskSpace, NumSys},
};
use tabled::{
    object::{Columns, Segment},
    Alignment, Modify, Style, Table, Tabled,
};

use std::io::Write;

use crate::{
    db::{
        dpkg_status, get_sources, get_sources_dists_filename, newest_package_list, package_list,
        packages_download, update_db, APT_LIST_DISTS,
    },
    formatter::NoProgress,
    pager::Pager,
    pkgversion::PkgVersion,
    success, warn,
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

impl RemoveRow {
    fn new(dpkg: &[IndexMap<String, String>], name: &str, is_purge: bool) -> Result<Self> {
        let pkg = dpkg
            .iter()
            .find(|x| x.get("Package") == Some(&name.to_owned()))
            .context(format!("Can not get package {name}"))?;

        let size = pkg["Installed-Size"].to_owned();

        Ok(Self {
            name: style(name).red().bold().to_string(),
            _name_no_color: name.to_owned(),
            size,
            detail: if is_purge {
                style("Purge configuration files.").red().to_string()
            } else {
                "".to_string()
            },
        })
    }
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
    size: String,
}

pub struct OmaAction {
    sources: Vec<SourceEntry>,
    db: Vec<IndexMap<String, String>>,
    dpkg_db: Vec<IndexMap<String, String>>,
    client: Client,
}

struct PackageVersion {
    name: String,
    version: String,
}

impl OmaAction {
    pub async fn new() -> Result<Self> {
        let client = reqwest::ClientBuilder::new().user_agent("oma").build()?;

        let sources = get_sources()?;

        std::fs::create_dir_all(APT_LIST_DISTS)?;
        std::fs::create_dir_all("/var/cache/apt/archives")?;

        let db_paths = get_sources_dists_filename(&sources, &client).await?;

        let db = package_list(
            db_paths
                .iter()
                .map(|x| Path::new(APT_LIST_DISTS).join(x))
                .collect(),
            &sources,
            &client,
        )
        .await?;

        let dpkg_db = dpkg_status()?;

        Ok(Self {
            sources,
            db,
            client,
            dpkg_db,
        })
    }

    /// Update mirror database and Get all update, like apt update && apt full-upgrade
    pub async fn update(&self, packages: &[String]) -> Result<()> {
        update_db(&self.sources, &self.client, None).await?;

        async fn update_inner(
            sources: &[SourceEntry],
            client: &Client,
            apt_db: &[IndexMap<String, String>],
            dpkg: &[IndexMap<String, String>],
            count: usize,
            packages: &[String],
        ) -> Result<()> {
            let cache = new_cache!()?;
            cache.upgrade(&Upgrade::FullUpgrade)?;

            let spv = if !packages.is_empty() {
                install_handle(packages, &cache, apt_db).ok()
            } else {
                None
            };

            let (action, len) = apt_handler(&cache, dpkg, apt_db, spv.as_deref())?;

            if len == 0 {
                success!("No need to do anything.");
                return Ok(());
            }

            let mut list = action.update.clone();
            list.extend(action.install.clone());
            list.extend(action.downgrade.clone());

            autoremove(&cache);

            if count == 0 {
                let disk_size = cache.depcache().disk_size();
                display_result(&action, apt_db, disk_size)?;
            }

            let db_for_update = newest_package_list(apt_db)?;

            packages_download(&list, &db_for_update, sources, client, None).await?;

            cache.resolve(true)?;
            apt_install(cache)?;

            Ok(())
        }

        // Retry 3 times
        let mut count = 0;
        while let Err(e) = update_inner(
            &self.sources,
            &self.client,
            &self.db,
            &self.dpkg_db,
            count,
            packages,
        )
        .await
        {
            warn!("{e}, retrying ...");
            if count == 3 {
                return Err(e);
            }
            count += 1;
        }

        Ok(())
    }

    pub async fn install(&self, list: &[String]) -> Result<()> {
        update_db(&self.sources, &self.client, None).await?;

        let mut count = 0;
        while let Err(e) = self.install_inner(list, count).await {
            warn!("{e}, retrying ...");
            if count == 3 {
                return Err(e);
            }

            count += 1;
        }

        Ok(())
    }

    async fn install_inner(&self, list: &[String], count: usize) -> Result<()> {
        let cache = new_cache!()?;

        let selected_packages_version = install_handle(list, &cache, &self.db)?;

        let (action, len) = apt_handler(
            &cache,
            &self.dpkg_db,
            &self.db,
            Some(&selected_packages_version),
        )?;

        if len == 0 {
            success!("No need to do anything.");
            return Ok(());
        }

        let mut list = vec![];
        list.extend(action.install.clone());
        list.extend(action.update.clone());
        list.extend(action.downgrade.clone());
        autoremove(&cache);
        cache.resolve(true)?;

        if count == 0 {
            let disk_size = cache.depcache().disk_size();
            display_result(&action, &self.db, disk_size)?;
        }

        // let names = list
        //     .into_iter()
        //     .map(|x| x.name_no_color)
        //     .collect::<Vec<_>>();
        // TODO: limit 参数（限制下载包并发）目前是写死的，以后将允许用户自定义并发数
        packages_download(&list, &self.db, &self.sources, &self.client, None).await?;
        apt_install(cache)?;

        Ok(())
    }

    pub fn remove(&self, list: &[String], is_purge: bool) -> Result<()> {
        let cache = new_cache!()?;

        for i in list {
            let pkg = cache.get(i).context(format!("Can not get package {i}"))?;
            pkg.mark_delete(is_purge);
        }

        autoremove(&cache);

        let (action, len) = apt_handler(&cache, &self.dpkg_db, &self.db, None)?;

        if len == 0 {
            success!("No need to do anything.");
            return Ok(());
        }

        let disk_size = cache.depcache().disk_size();
        display_result(&action, &self.db, disk_size)?;

        cache.commit(
            &mut NoProgress::new_box(),
            &mut AptInstallProgress::new_box(),
        )?;

        Ok(())
    }

    pub async fn refresh(&self) -> Result<()> {
        update_db(&self.sources, &self.client, None).await
    }
}

fn autoremove(cache: &Cache) {
    let sort = PackageSort::default();
    let mut autoremove = vec![];

    for pkg in cache.packages(&sort) {
        if pkg.is_auto_removable() {
            autoremove.push(pkg.name().to_string());
            pkg.mark_delete(true);
        }
    }
}

fn apt_install(cache: Cache) -> Result<()> {
    apt_lock()?;
    cache.get_archives(&mut NoProgress::new_box())?;
    apt_unlock_inner();

    if let Err(e) = cache.do_install(&mut AptInstallProgress::new_box()) {
        apt_lock_inner()?;
        apt_unlock();
        return Err(e.into());
    }

    apt_unlock();

    Ok(())
}

fn install_handle(
    list: &[String],
    cache: &Cache,
    apt_db: &[IndexMap<String, String>],
) -> Result<Vec<PackageVersion>> {
    let mut selected_packages_version = vec![];

    for i in list {
        if i.contains('=') {
            // Support apt install fish=3.6.0

            let mut split_arg = i.split('=');
            let name = split_arg.next().context(format!("Not Support: {i}"))?;
            let version_str = split_arg.next().context(format!("Not Support: {i}"))?;

            let pkg = cache
                .get(name)
                .take()
                .context(format!("Can not get package: {i}"))?;

            if PkgVersion::try_from(version_str).is_err() {
                bail!("invalid version: {}", version_str);
            }

            let version = pkg
                .get_version(version_str)
                .context(format!("Can not get package {name} version: {version_str}"))?;

            version.set_candidate();

            selected_packages_version.push(PackageVersion {
                name: name.to_string(),
                version: version_str.to_string(),
            });

            pkg.protect();
            pkg.mark_install(true, true);
        } else if i.contains('/') {
            // Support apt install fish/stable
            let mut split_arg = i.split('/');
            let name = split_arg.next().context(format!("Not Support: {i}"))?;
            let branch = split_arg.next().context(format!("Not Support: {i}"))?;

            let pkg = cache
                .get(name)
                .take()
                .context(format!("Can not get package: {i}"))?;

            let mut res = apt_db
                .iter()
                .filter(|x| x.get("Package") == Some(&name.to_string()))
                .filter(|x| x["Filename"].split('/').nth(1) == Some(branch))
                .collect::<Vec<_>>();

            if res.is_empty() {
                bail!("Can not get package {} with {} branch.", name, branch);
            }

            res.sort_by(|x, y| {
                PkgVersion::try_from(x["Version"].as_str())
                    .unwrap()
                    .cmp(&PkgVersion::try_from(y["Version"].as_str()).unwrap())
            });

            let res = res.last().unwrap()["Version"].to_string();
            let version = pkg.get_version(&res).unwrap();

            version.set_candidate();

            selected_packages_version.push(PackageVersion {
                name: name.to_string(),
                version: res.to_string(),
            });

            // let pkg = version.parent();
            // pkg.protect();
            pkg.mark_install(true, true);
        } else {
            let pkg = cache
                .get(i)
                .take()
                .context(format!("Can not get package: {i}"))?;

            // let version = pkg
            //     .candidate()
            //     .context(format!("Can not get package candidate: {}", pkg.name()))?;

            // let pkg = version.parent();

            pkg.protect();
            pkg.mark_install(true, true);
        };
    }

    Ok(selected_packages_version)
}

#[derive(Debug, Clone)]
struct Action {
    update: Vec<InstallRow>,
    install: Vec<InstallRow>,
    del: Vec<RemoveRow>,
    reinstall: Vec<String>,
    downgrade: Vec<InstallRow>,
}

impl Action {
    fn new(
        update: Vec<InstallRow>,
        install: Vec<InstallRow>,
        del: Vec<RemoveRow>,
        reinstall: Vec<String>,
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

fn apt_handler(
    cache: &Cache,
    dpkg: &[IndexMap<String, String>],
    apt: &[IndexMap<String, String>],
    select_packages_version: Option<&[PackageVersion]>,
) -> Result<(Action, usize)> {
    let changes = cache.get_changes(true).collect::<Vec<_>>();
    let len = changes.len();

    let mut update: Vec<InstallRow> = vec![];
    let mut install: Vec<InstallRow> = vec![];
    let mut del: Vec<RemoveRow> = vec![];
    let mut reinstall: Vec<String> = vec![];
    let mut downgrade: Vec<InstallRow> = vec![];

    for pkg in changes {
        if pkg.marked_install() {
            let version = if let Some(spv) = select_packages_version {
                if let Some(v) = spv
                    .iter()
                    .find(|x| x.name == pkg.name())
                    .map(|x| x.version.clone())
                {
                    v
                } else {
                    let cand = pkg.candidate().take().context(format!(
                        "Can not get package version in apt database: {}",
                        pkg.name()
                    ))?;

                    cand.version().to_string()
                }
            } else {
                let cand = pkg.candidate().take().context(format!(
                    "Can not get package version in apt database: {}",
                    pkg.name()
                ))?;

                cand.version().to_string()
            };

            let apt_pkg = apt
                .iter()
                .find(|x| {
                    x.get("Package") == Some(&pkg.name().to_string())
                        && x.get("Version") == Some(&version.to_string())
                })
                .context(format!(
                    "Can not get package version in apt database: {}",
                    pkg.name()
                ))?;

            let size = apt_pkg["Installed-Size"].parse::<u64>()?;

            let size = format!("+{}", unit_str(size, NumSys::Decimal,));

            install.push(InstallRow {
                name: style(pkg.name()).green().to_string(),
                name_no_color: pkg.name().to_string(),
                version: version.to_string(),
                new_version: version.to_string(),
                size,
            });

            // If the package is marked install then it will also
            // show up as marked upgrade, downgrade etc.
            // Check this first and continue.
            continue;
        }

        if pkg.marked_upgrade() {
            // let id = pkg.id();
            let pkg_ver = pkg.version_list().unwrap();

            let version = pkg_ver.version();

            let old_pkg = dpkg
                .iter()
                .find(|x| x.get("Package") == Some(&pkg.name().to_string()))
                .context(format!(
                    "Can not get package version from dpkg: {}",
                    pkg.name()
                ))?;

            let old_version = old_pkg["Version"].to_string();
            let old_size = old_pkg["Installed-Size"].parse::<i64>()?;

            let apt_pkg = apt
                .iter()
                .find(|x| {
                    x.get("Package") == Some(&pkg.name().to_string())
                        && x.get("Version") == Some(&version.to_string())
                })
                .context(format!(
                    "Can not get package version in apt database: {}",
                    pkg.name()
                ))?;

            let new_size = apt_pkg["Installed-Size"].parse::<i64>()?;

            let size = new_size - old_size;

            let size = if size >= 0 {
                format!("+{}", unit_str(size as u64, NumSys::Decimal))
            } else {
                format!("-{}", unit_str(size.abs() as u64, NumSys::Decimal))
            };

            update.push(InstallRow {
                name: style(pkg.name()).green().to_string(),
                name_no_color: pkg.name().to_string(),
                version: format!("{old_version} -> {version}"),
                new_version: version.to_string(),
                size,
            });
        }

        if pkg.marked_delete() {
            let remove_row = RemoveRow::new(dpkg, pkg.name(), true)?;
            del.push(remove_row);
        }
        if pkg.marked_reinstall() {
            reinstall.push(pkg.name().to_string());
        }
        if pkg.marked_downgrade() {
            // let id = pkg.unsafe_version_list();
            let version = if let Some(spv) = select_packages_version {
                if let Some(v) = spv
                    .iter()
                    .find(|x| x.name == pkg.name())
                    .map(|x| x.version.clone())
                {
                    v
                } else {
                    let cand = pkg.candidate().take().context(format!(
                        "Can not get package version in apt database: {}",
                        pkg.name()
                    ))?;

                    cand.version().to_string()
                }
            } else {
                let cand = pkg.candidate().take().context(format!(
                    "Can not get package version in apt database: {}",
                    pkg.name()
                ))?;

                cand.version().to_string()
            };

            let old_pkg = dpkg
                .iter()
                .find(|x| x.get("Package") == Some(&pkg.name().to_string()))
                .context(format!(
                    "Can not get package version from dpkg: {}",
                    pkg.name()
                ))?;

            let old_version = old_pkg["Version"].to_string();
            let old_size = old_pkg["Installed-Size"].parse::<i64>()?;

            let apt_pkg = apt
                .iter()
                .find(|x| {
                    x.get("Package") == Some(&pkg.name().to_string())
                        && x.get("Version") == Some(&version.to_string())
                })
                .context(format!(
                    "Can not get package version in apt database: {}",
                    pkg.name()
                ))?;

            let new_size = apt_pkg["Installed-Size"].parse::<i64>()?;

            let size = new_size - old_size;

            let size = if size >= 0 {
                format!("+{}", unit_str(size as u64, NumSys::Decimal))
            } else {
                format!("-{}", unit_str(size.abs() as u64, NumSys::Decimal))
            };

            downgrade.push(InstallRow {
                name: style(pkg.name()).green().to_string(),
                name_no_color: pkg.name().to_string(),
                version: format!("{old_version} -> {version}"),
                new_version: version.to_string(),
                size,
            });
        }
    }

    let action = Action::new(update, install, del, reinstall, downgrade);

    Ok((action, len))
}

fn display_result(
    action: &Action,
    apt: &[IndexMap<String, String>],
    disk_size: DiskSpace,
) -> Result<()> {
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
        writeln!(out, "{}", style("Press [q] to finish review.\n").bold())?;
    }

    if !del.is_empty() {
        writeln!(
            out,
            "The following packages will be {}:\n",
            style("REMOVED").red().bold()
        )?;

        // let remove_rows = remove_rows.to_vec();

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
            "The following packages will be {}:\n",
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
            "\nThe following packages will be {}:\n",
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
            "\nThe following packages will be {}:\n",
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
            "The following packages will be {}:\n",
            style("reinstall").blue().bold()
        )?;

        writeln!(out, "\n{}", reinstall.join(", "))?;
    }

    let mut list = vec![];
    list.extend(install);
    list.extend(update);
    list.extend(downgrade);

    writeln!(
        out,
        "{} {}",
        style("\nTotal download size:").bold(),
        HumanBytes(download_size(&list, apt)?)
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
    pager.wait_for_exit()?;

    Ok(())
}

fn download_size(
    install_and_update: &[InstallRow],
    apt: &[IndexMap<String, String>],
) -> Result<u64> {
    let mut result = 0;

    for i in install_and_update {
        let item = apt
            .iter()
            .find(|x| {
                x.get("Package") == Some(&i.name_no_color)
                    && x.get("Version") == Some(&i.new_version)
            })
            .context(format!(
                "Can not find pacakge {} from apt package database",
                i.name
            ))?;

        let size = item["Size"].parse::<u64>()?;
        result += size;
    }

    Ok(result)
}

fn write_review_help_message(w: &mut dyn Write) -> Result<()> {
    writeln!(w, "{}", style("Pending Operations").bold())?;
    writeln!(w)?;
    writeln!(w, "Shown below is an overview of the pending changes Omakase will apply to your system, please review them carefully.")?;
    writeln!(w, "Please note that Omakase may {}, {}, {}, {}, or {} packages in order to fulfill your requested changes.", style("install").green(), style("remove").red(), style("upgrade").green(), style("downgrade").yellow(), style("reinstall").blue())?;
    writeln!(w)?;
    Ok(())
}
