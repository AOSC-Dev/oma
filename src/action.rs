use std::path::Path;

use anyhow::{bail, Context, Ok, Result};
use apt_sources_lists::SourceEntry;
use console::style;
use indexmap::IndexMap;
use reqwest::Client;
use rust_apt::{
    cache::{Cache, PackageSort, Upgrade},
    new_cache,
    raw::{progress::AptInstallProgress, util::raw::apt_lock_inner},
    util::{apt_lock, apt_unlock, apt_unlock_inner, unit_str, NumSys},
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
            name: name.to_owned(),
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
struct InstallRow {
    #[tabled(rename = "Name")]
    name: String,
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

impl OmaAction {
    pub async fn new() -> Result<Self> {
        let client = reqwest::ClientBuilder::new().user_agent("aoscpt").build()?;

        let sources = get_sources()?;

        std::fs::create_dir_all(APT_LIST_DISTS)?;

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
    pub async fn update(&self) -> Result<()> {
        update_db(&self.sources, &self.client, None).await?;

        async fn update_inner(
            sources: &[SourceEntry],
            client: &Client,
            db: &[IndexMap<String, String>],
            dpkg: &[IndexMap<String, String>],
        ) -> Result<()> {
            let cache = new_cache!()?;
            cache.upgrade(&Upgrade::FullUpgrade)?;

            let (action, len) = apt_handler(&cache, db, dpkg)?;

            if len == 0 {
                success!("No need to do anything.");
                return Ok(());
            }

            let mut list = action.update.clone();
            list.extend(action.install.clone());

            let names = list.into_iter().map(|x| x.name).collect::<Vec<_>>();

            autoremove(&cache);

            display_result(&action)?;

            let db_for_updates = newest_package_list(db)?;
            packages_download(&names, &db_for_updates, sources, client, None).await?;

            cache.resolve(true)?;
            apt_install(cache)?;

            Ok(())
        }

        // Retry 3 times
        let mut count = 0;
        while let Err(e) = update_inner(&self.sources, &self.client, &self.db, &self.dpkg_db).await
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
        while let Err(e) = self.install_inner(list).await {
            warn!("{e}, retrying ...");
            if count == 3 {
                return Err(e);
            }

            count += 1;
        }

        Ok(())
    }

    async fn install_inner(&self, list: &[String]) -> Result<()> {
        let cache = new_cache!()?;
        for i in list {
            let pkg = cache
                .get(i)
                .take()
                .context(format!("Can not get package: {i}"))?;
            if i.contains('=') {
                // Support apt install fish=3.6.0

                let mut split_arg = i.split('=');
                let name = split_arg.next().context(format!("Not Support: {i}"))?;
                let version = split_arg.nth(1).context(format!("Not Support: {i}"))?;

                if PkgVersion::try_from(version).is_err() {
                    bail!("invalid version: {}", version);
                }

                let version = pkg
                    .get_version(version)
                    .context(format!("Can not get package {name} version: {version}"))?;

                let pkg = version.parent();
                pkg.mark_install(true, true);
                pkg.protect();
            } else if i.contains('/') {
                // Support apt install fish/stable
                let mut split_arg = i.split('/');
                let name = split_arg.next().context(format!("Not Support: {i}"))?;
                let branch = split_arg.nth(1).context(format!("Not Support: {i}"))?;

                let mut res = self
                    .db
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

                let pkg = version.parent();
                pkg.mark_install(true, true);
                pkg.protect();
            }

            pkg.mark_install(true, true);
            pkg.protect();
        }
        let (action, len) = apt_handler(&cache, &self.db, &self.dpkg_db)?;

        if len == 0 {
            success!("No need to do anything.");
            return Ok(());
        }

        let mut list = vec![];
        list.extend(action.install.clone());
        list.extend(action.update.clone());
        autoremove(&cache);
        cache.resolve(true)?;

        display_result(&action)?;

        let names = list.into_iter().map(|x| x.name).collect::<Vec<_>>();
        // TODO: limit 参数（限制下载包并发）目前是写死的，以后将允许用户自定义并发数
        packages_download(&names, &self.db, &self.sources, &self.client, None).await?;
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

        let (action, len) = apt_handler(&cache, &self.db, &self.dpkg_db)?;

        if len == 0 {
            success!("No need to do anything.");
            return Ok(());
        }

        display_result(&action)?;

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
    apt: &[IndexMap<String, String>],
    dpkg: &[IndexMap<String, String>],
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
            let version = pkg
                .candidate()
                .context(format!("Can not get package version {}", pkg.name()))?
                .version()
                .to_owned();

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

            let size = format!(
                "+{}",
                unit_str(
                    apt_pkg["Installed-Size"].to_owned().parse::<u64>()?,
                    NumSys::Decimal,
                )
            );

            install.push(InstallRow {
                name: style(pkg.name()).green().to_string(),
                version: version.to_string(),
                size,
            });

            // If the package is marked install then it will also
            // show up as marked upgrade, downgrade etc.
            // Check this first and continue.
            continue;
        }
        if pkg.marked_upgrade() {
            let version = pkg
                .candidate()
                .context(format!("Can not get package version {}", pkg.name()))?
                .version()
                .to_owned();

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

            let old_pkg = dpkg
                .iter()
                .find(|x| x.get("Package") == Some(&pkg.name().to_string()))
                .context(format!(
                    "Can not get package version from dpkg: {}",
                    pkg.name()
                ))?;

            let old_version = old_pkg["Version"].to_string();
            let old_size = old_pkg["Installed-Size"].parse::<i64>()?;
            let new_size = apt_pkg["Installed-Size"].parse::<i64>()?;
            let size = new_size - old_size;

            let size = if size >= 0 {
                format!("+{}", unit_str(size as u64, NumSys::Decimal))
            } else {
                format!("-{}", unit_str(size.abs() as u64, NumSys::Decimal))
            };

            update.push(InstallRow {
                name: style(pkg.name()).green().to_string(),
                version: format!("{old_version} -> {version}"),
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
            let version = pkg
                .candidate()
                .context(format!("Can not get package version {}", pkg.name()))?
                .version()
                .to_owned();

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

            let old_pkg = dpkg
                .iter()
                .find(|x| x.get("Package") == Some(&pkg.name().to_string()))
                .context(format!(
                    "Can not get package version from dpkg: {}",
                    pkg.name()
                ))?;

            let old_version = old_pkg["Version"].to_string();
            let old_size = old_pkg["Installed-Size"].parse::<i64>()?;
            let new_size = apt_pkg["Installed-Size"].parse::<i64>()?;
            let size = new_size - old_size;

            let size = if size >= 0 {
                format!("+{}", unit_str(size as u64, NumSys::Decimal))
            } else {
                format!("-{}", unit_str(size.abs() as u64, NumSys::Decimal))
            };

            downgrade.push(InstallRow {
                name: style(pkg.name()).green().to_string(),
                version: format!("{old_version} -> {version}"),
                size,
            });
        }
    }

    let action = Action::new(update, install, del, reinstall, downgrade);

    Ok((action, len))
}

fn display_result(action: &Action) -> Result<()> {
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

        let mut table = Table::new(install);

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

        let mut table = Table::new(update);

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

        let mut table = Table::new(downgrade);

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

    drop(out);
    pager.wait_for_exit()?;

    // writeln!(
    //     out,
    //     "{} {}",
    //     style("Total download size:").bold(),
    //     HumanBytes(actions.calculate_download_size())
    // )?;

    Ok(())
}

fn write_review_help_message(w: &mut dyn Write) -> Result<()> {
    writeln!(w, "{}", style("Pending Operations").bold())?;
    writeln!(w)?;
    writeln!(w, "Shown below is an overview of the pending changes Omakase will apply to your system, please review them carefully.")?;
    writeln!(w, "Please note that Omakase may {}, {}, {}, {}, or {} packages in order to fulfill your requested changes.", style("install").green(), style("remove").red(), style("upgrade").green(), style("downgrade").yellow(), style("reinstall").blue())?;
    writeln!(w)?;
    Ok(())
}
