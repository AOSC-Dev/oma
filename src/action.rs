use anyhow::{bail, Context, Ok, Result};
use apt_sources_lists::SourceEntry;
use console::{style, Color};
use glob_match::glob_match_with_captures;
use indicatif::HumanBytes;
use reqwest::Client;
use rust_apt::{
    cache::{Cache, PackageSort, Upgrade},
    new_cache,
    raw::{progress::AptInstallProgress, util::raw::apt_lock_inner},
    records::RecordField,
    util::{apt_lock, apt_unlock, apt_unlock_inner, unit_str, DiskSpace, NumSys},
};
use tabled::{
    object::{Columns, Segment},
    Alignment, Modify, Style, Table, Tabled,
};

use std::io::Write;

use crate::{
    db::{get_sources, packages_download, update_db, APT_LIST_DISTS},
    formatter::NoProgress,
    pager::Pager,
    pkgversion::PkgVersion,
    success,
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

// impl RemoveRow {
//     fn new(name: &str, is_purge: bool) -> Result<Self> {
//         let pkg = dpkg
//             .iter()
//             .find(|x| x.get("Package") == Some(&name.to_owned()))
//             .context(format!("Can not get package {name}"))?;

//         let size = pkg["Installed-Size"].to_owned();

//         Ok(Self {
//             name: style(name).red().bold().to_string(),
//             _name_no_color: name.to_owned(),
//             size: unit_str(size.parse::<u64>()?, NumSys::Decimal),
//             detail: if is_purge {
//                 style("Purge configuration files.").red().to_string()
//             } else {
//                 "".to_string()
//             },
//         })
//     }
// }

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

pub struct OmaAction {
    sources: Vec<SourceEntry>,
    client: Client,
}

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
        update_db(&self.sources, &self.client, None).await?;

        async fn update_inner(client: &Client, count: usize, packages: &[String]) -> Result<()> {
            let local_debs = packages
                .iter()
                .filter(|x| x.ends_with(".deb"))
                .collect::<Vec<_>>();

            let cache = new_cache!(&local_debs)?;
            cache.upgrade(&Upgrade::FullUpgrade)?;

            install_handle(packages, &cache)?;

            let (action, len) = apt_handler(&cache)?;

            if len == 0 {
                success!("No need to do anything.");
                return Ok(());
            }

            let mut list = action.update.clone();
            list.extend(action.install.clone());
            list.extend(action.downgrade.clone());

            // autoremove 标记可以删除的包前解析一次依赖，在执行 autoremove 后再解析一次依赖，不知是否必要
            cache.resolve(true)?;
            autoremove(&cache);
            cache.resolve(true)?;

            if count == 0 {
                let disk_size = cache.depcache().disk_size();
                display_result(&action, &cache, disk_size)?;
            }

            // let db_for_update = newest_package_list(apt_db)?;

            packages_download(&list, client, None).await?;

            if let Err(e) = cache.resolve(true) {
                return Err(e.into());
                // let sort = PackageSort::default();
                // let mut now_broken = false;
                // let mut inst_broken = false;
                // for pkg in cache.packages(&sort) {
                //     now_broken = pkg.is_now_broken();
                //     inst_broken = pkg.is_inst_broken();

                //     if !now_broken && !inst_broken {
                //         continue;
                //     }

                //     let ver = if now_broken {
                //         pkg.current_version()
                //     } else {
                //         pkg.version_list()
                //     }
                //     .context("123")?;

                //     while let Some(i) = ver.depends() {
                //         dbg!(i.dep_type());
                //     }
                // }
            }

            apt_install(cache)?;

            Ok(())
        }

        // Retry 3 times
        let mut count = 0;
        while let Err(e) = update_inner(&self.client, count, packages).await {
            // warn!("{e}, retrying ...");
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
            // debug!("{e}, retrying ...");
            if count == 3 {
                return Err(e);
            }

            count += 1;
        }

        Ok(())
    }

    async fn install_inner(&self, list: &[String], count: usize) -> Result<()> {
        let local_debs = list
            .iter()
            .filter(|x| x.ends_with(".deb"))
            .collect::<Vec<_>>();

        let cache = new_cache!(&local_debs)?;
        install_handle(list, &cache)?;

        let (action, len) = apt_handler(&cache)?;

        if len == 0 {
            success!("No need to do anything.");
            return Ok(());
        }

        let mut list = vec![];
        list.extend(action.install.clone());
        list.extend(action.update.clone());
        list.extend(action.downgrade.clone());

        autoremove(&cache);

        if let Err(e) = cache.resolve(true) {
            if count != 0 {
                return Err(e.into());
            }

            // let sort = PackageSort::default().installed();
            // let now = false;
            // for pkg in cache.packages(&sort) {
            //     let now_broken = pkg.is_now_broken();
            //     let inst_broken = pkg.is_inst_broken();

            //     if !now_broken && !inst_broken {
            //         continue;
            //     }

            //     // let ver = if now_broken {
            //     //     pkg.installed()
            //     // } else {
            //     //     pkg.ins
            //     // }
            //     // .context("context")?;

            //     // dbg!(pkg.name());
            //     // dbg!(now_broken, ins)

            //     // for i in ver.dependencies().unwrap() {
            //     //     for j in &i.base_deps {
            //     //         if j.target_pkg().is_essential() {
            //     //             continue;
            //     //         }

            //     //         dbg!(cache.depcache().is_inst_broken(&j.target_pkg()));

            //     //         // if now && !cache.depcache().is_now_broken(&j.target_pkg()) {
            //     //         //     continue;
            //     //         // }

            //     //         // if !now && !cache.depcache().is_inst_broken(&j.target_pkg()) {
            //     //         //     continue;
            //     //         // }

            //     //         dbg!(j.target_pkg().name());
            //     //     }
            //     // }

            //     return Err(e.into());
            // }
        }

        if count == 0 {
            let disk_size = cache.depcache().disk_size();
            display_result(&action, &cache, disk_size)?;
        }

        // TODO: limit 参数（限制下载包并发）目前是写死的，以后将允许用户自定义并发数
        packages_download(&list, &self.client, None).await?;
        apt_install(cache)?;

        // let cache = install_handle_with_local(&local_debs)?;

        // let (action, len) = apt_handler(&cache)?;

        // if len == 0 {
        //     success!("No need to do anything.");
        //     return Ok(());
        // }

        // if count == 0 && !local_debs.is_empty() {
        //     let disk_size = cache.depcache().disk_size();
        //     display_result(&action, &cache, disk_size)?;
        // }

        // apt_install(cache)?;

        Ok(())
    }

    pub fn remove(&self, list: &[String], is_purge: bool) -> Result<()> {
        let cache = new_cache!()?;

        for i in list {
            let pkg = cache.get(i).context(format!("Can not get package {i}"))?;
            pkg.mark_delete(is_purge);
            pkg.protect();
        }

        cache.resolve(true)?;
        autoremove(&cache);
        cache.resolve(true)?;

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
            pkg.protect();
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

fn install_handle(list: &[String], cache: &Cache) -> Result<()> {
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

            pkg.mark_install(true, true);
            pkg.protect();
        } else if i.contains('/') && !i.ends_with(".deb") {
            // Support apt install fish/stable
            let mut split_arg = i.split('/');
            let name = split_arg.next().context(format!("Not Support: {i}"))?;
            let branch = split_arg.next().context(format!("Not Support: {i}"))?;

            let pkg = cache
                .get(name)
                .take()
                .context(format!("Can not get package: {i}"))?;

            let mut sort = vec![];

            for i in pkg.versions() {
                let item = i
                    .get_record(RecordField::Filename)
                    .context(format!("Can not get package {name} filename!"))?;

                if item.split('/').nth(1) == Some(branch) {
                    sort.push(i)
                }
            }

            if sort.is_empty() {
                bail!("Can not get package {} with {} branch.", name, branch);
            }

            sort.sort_by(|x, y| rust_apt::util::cmp_versions(x.version(), y.version()));

            let version = sort.last().unwrap();

            version.set_candidate();

            pkg.mark_install(true, true);
            pkg.protect();
        } else if !i.ends_with(".deb") {
            let sort = PackageSort::default();
            let res = cache
                .packages(&sort)
                .filter(|x| glob_match_with_captures(i, x.name()).is_some());

            for pkg in res {
                pkg.mark_install(true, true);
                pkg.protect();
            }
        } else {
            let mut filename_split = i.split('/').last().unwrap_or(i).split('_');
            let package = filename_split
                .next()
                .context(format!("Can not get pacakge name from file: {i}"))?;

            let pkg = cache
                .get(package)
                .take()
                .context(format!("Can not get package from file: {i}"))?;

            pkg.mark_install(true, true);
            pkg.protect();
        }
    }

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

fn apt_handler(cache: &Cache) -> Result<(Action, usize)> {
    let changes = cache.get_changes(true).collect::<Vec<_>>();
    let len = changes.len();

    let mut update: Vec<InstallRow> = vec![];
    let mut install: Vec<InstallRow> = vec![];
    let mut del: Vec<RemoveRow> = vec![];
    let mut reinstall: Vec<String> = vec![];
    let mut downgrade: Vec<InstallRow> = vec![];

    for pkg in changes {
        if pkg.marked_install() {
            let cand = pkg.candidate().take().context(format!(
                "Can not get package version in apt database: {}",
                pkg.name()
            ))?;

            let size = cand.installed_size();
            let human_size = format!("+{}", unit_str(size, NumSys::Decimal));

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
            let old_size = old_pkg.size() as i64;

            let new_pkg = pkg
                .candidate()
                .context(format!("Can not get candidate version: {}", pkg.name()))?;

            let new_size = new_pkg.installed_size() as i64;

            let size = new_size - old_size;

            let size = if size >= 0 {
                format!("+{}", unit_str(size as u64, NumSys::Decimal))
            } else {
                format!("-{}", unit_str(size.unsigned_abs(), NumSys::Decimal))
            };

            update.push(InstallRow {
                name: style(pkg.name()).cyan().to_string(),
                name_no_color: pkg.name().to_string(),
                version: format!("{old_version} -> {version}"),
                new_version: version.to_string(),
                size,
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
                size: unit_str(size, NumSys::Decimal),
                detail: if is_purge {
                    style("Purge configuration files.").red().to_string()
                } else {
                    "".to_string()
                },
            };

            del.push(remove_row);
        }
        if pkg.marked_reinstall() {
            reinstall.push(pkg.name().to_string());
        }
        if pkg.marked_downgrade() {
            // let id = pkg.unsafe_version_list();

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

            let new_size = new_pkg.size() as i64;

            let size = new_size - old_size;

            let size = if size >= 0 {
                format!("+{}", unit_str(size as u64, NumSys::Decimal))
            } else {
                format!("-{}", unit_str(size.unsigned_abs(), NumSys::Decimal))
            };

            downgrade.push(InstallRow {
                name: style(pkg.name()).yellow().to_string(),
                name_no_color: pkg.name().to_string(),
                version: format!("{old_version} -> {version}"),
                new_version: version.to_string(),
                size,
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

        writeln!(out, "\n{}", reinstall.join(", "))?;
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
    pager.wait_for_exit()?;

    Ok(())
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
