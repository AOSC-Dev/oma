use std::path::Path;

use anyhow::{bail, Context, Ok, Result};
use apt_sources_lists::SourceEntry;
use console::style;
use glob_match::glob_match_with_captures;
use indexmap::IndexMap;
use indicatif::HumanBytes;
use reqwest::Client;
use rust_apt::{
    cache::{Cache, PackageSort, Upgrade},
    new_cache,
    package::Version,
    raw::{progress::AptInstallProgress, util::raw::apt_lock_inner},
    records::RecordField,
    util::{apt_lock, apt_unlock, apt_unlock_inner, unit_str, DiskSpace, Exception, NumSys},
};
use tabled::{
    object::{Columns, Segment},
    Alignment, Modify, Style, Table, Tabled,
};

use std::io::Write;

use crate::{
    db::{
        dpkg_status, get_sources, get_sources_dists_filename, package_list, packages_download,
        update_db, APT_LIST_DISTS,
    },
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

            let (action, len) = apt_handler(&cache, spv.as_deref())?;

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
                display_result(&action, apt_db, disk_size)?;
            }

            // let db_for_update = newest_package_list(apt_db)?;

            packages_download(&list, apt_db, sources, client, None).await?;

            if let Err(e) = cache.resolve(true) {
                let sort = PackageSort::default();
                let mut now_broken = false;
                let mut inst_broken = false;
                for pkg in cache.packages(&sort) {
                    now_broken = pkg.is_now_broken();
                    inst_broken = pkg.is_inst_broken();

                    if !now_broken && !inst_broken {
                        continue;
                    }

                    let ver = if now_broken {
                        pkg.current_version()
                    } else {
                        pkg.version_list()
                    }
                    .context("123")?;

                    while let Some(i) = ver.depends() {
                        dbg!(i.dep_type());
                    }
                }
            }
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
        let cache = new_cache!()?;

        let selected_packages_version = install_handle(list, &cache, &self.db)?;

        let (action, len) = apt_handler(&cache, Some(&selected_packages_version))?;

        if len == 0 {
            success!("No need to do anything.");
            return Ok(());
        }

        let mut list = vec![];
        list.extend(action.install.clone());
        list.extend(action.update.clone());
        list.extend(action.downgrade.clone());

        // if let Err(e) = cache.resolve(true) {
        //     let sort = PackageSort::default();
        //     let mut now_broken = false;
        //     let mut inst_broken = false;
        //     for pkg in cache.packages(&sort) {
        //         now_broken = pkg.is_now_broken();
        //         inst_broken = pkg.is_inst_broken();

        //         if !now_broken && !inst_broken {
        //             continue;
        //         }

        //         let ver = if now_broken {
        //             pkg.current_version()
        //         } else {
        //             pkg.version_list()
        //         }
        //         .context("123")?;

        //         while let Some(i) = ver.depends() {
        //             dbg!(i.dep_type());
        //         }
        //     }
        // }

        autoremove(&cache);

        if let Err(e) = cache.resolve(true) {
            if count != 0 {
                return Err(e.into());
            }

            let sort = PackageSort::default().installed();
            let now = false;
            for pkg in cache.packages(&sort) {
                let now_broken = pkg.is_now_broken();
                let inst_broken = pkg.is_inst_broken();

                if !now_broken && !inst_broken {
                    continue;
                }

                // let ver = if now_broken {
                //     pkg.installed()
                // } else {
                //     pkg.ins
                // }
                // .context("context")?;

                // dbg!(pkg.name());
                // dbg!(now_broken, ins)

                // for i in ver.dependencies().unwrap() {
                //     for j in &i.base_deps {
                //         if j.target_pkg().is_essential() {
                //             continue;
                //         }

                //         dbg!(cache.depcache().is_inst_broken(&j.target_pkg()));

                //         // if now && !cache.depcache().is_now_broken(&j.target_pkg()) {
                //         //     continue;
                //         // }

                //         // if !now && !cache.depcache().is_inst_broken(&j.target_pkg()) {
                //         //     continue;
                //         // }

                //         dbg!(j.target_pkg().name());
                //     }
                // }

                return Err(e.into());
            }
        }

        if count == 0 {
            let disk_size = cache.depcache().disk_size();
            display_result(&action, &self.db, disk_size)?;
        }

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
            pkg.protect();
        }

        cache.resolve(true)?;
        autoremove(&cache);
        cache.resolve(true)?;

        let (action, len) = apt_handler(&cache, None)?;

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

            pkg.mark_install(true, true);
            pkg.protect();
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

            pkg.mark_install(true, true);
            pkg.protect();
        } else {
            let res = apt_db
                .iter()
                .filter(|x| glob_match_with_captures(i, &x["Package"]).is_some());

            for i in res {
                let pkg = cache
                    .get(i["Package"].as_str())
                    .take()
                    .context(format!("Can not get package: {}", i["Package"]))?;

                pkg.mark_install(true, true);
                pkg.protect();
            }
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

            let ver = pkg.get_version(&version).context(format!(
                "Can not get package version in apt database: {}",
                pkg.name()
            ))?;

            let size = ver.installed_size();
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
                name: style(pkg.name()).green().to_string(),
                name_no_color: pkg.name().to_string(),
                version: format!("{old_version} -> {version}"),
                new_version: version.to_string(),
                size,
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

            let old_pkg = pkg
                .installed()
                .context(format!("Can not get installed version: {}", pkg.name()))?;

            let old_version = old_pkg.version();
            let old_size = old_pkg.installed_size() as i64;

            let new_pkg = pkg.get_version(&version).context(format!(
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
