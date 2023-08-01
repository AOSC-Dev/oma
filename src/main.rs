use std::borrow::Cow;
use std::path::{Path, PathBuf};

use std::process::exit;

mod args;
mod lang;
mod table;

use anyhow::{anyhow, Result};

use clap::ArgMatches;
use dialoguer::theme::ColorfulTheme;
use dialoguer::Select;
use nix::sys::signal;
use oma_console::indicatif::ProgressBar;
use oma_console::pb::oma_spinner;
use oma_console::writer::gen_prefix;
use oma_console::{console::style, info};
use oma_console::{debug, due_to, error, success, warn, DEBUG, WRITER};
use oma_contents::QueryMode;
use oma_pm::apt::{AptArgs, FilterMode, OmaApt, OmaAptError, OmaArgs};
use oma_pm::pkginfo::PkgInfo;
use oma_pm::query::OmaDatabase;
use oma_pm::PackageStatus;
use oma_refresh::db::OmaRefresh;
use oma_utils::{dpkg_arch, unlock_oma, OsRelease};

use std::sync::atomic::{AtomicBool, Ordering};

use oma_console::console;

use crate::table::table_for_install_pending;

use oma_console::pager::{Pager, SUBPROCESS};

static ALLOWCTRLC: AtomicBool = AtomicBool::new(false);
static LOCKED: AtomicBool = AtomicBool::new(false);
static AILURUS: AtomicBool = AtomicBool::new(false);


fn main() {
    ctrlc::set_handler(single_handler).expect(
        "Oma could not initialize SIGINT handler.\n\nPlease restart your installation environment.",
    );

    let code = match try_main() {
        Ok(exit_code) => exit_code,
        Err(e) => {
            if !e.to_string().is_empty() {
                error!("{e}");
            }
            e.chain().skip(1).for_each(|cause| {
                due_to!("{cause}");
            });
            1
        }
    };

    unlock_oma().ok();

    exit(code);
}

fn try_main() -> Result<i32> {
    let cmd = args::command_builder();
    let matches = cmd.get_matches();

    // Egg
    if matches.get_count("ailurus") == 3 {
        AILURUS.store(true, Ordering::Relaxed);
    } else if matches.get_count("ailurus") != 0 {
        println!(
            "{} unexpected argument '{}' found\n",
            style("error:").red().bold(),
            style("\x1b[33m--ailurus\x1b[0m").bold()
        );
        println!("{}: oma <COMMAND>\n", style("Usage").bold().underlined());
        println!("For more information, try '{}'.", style("--help").bold());

        return Ok(3);
    }

    // Init dry-run flag
    // let _dry_run = if let Some(Ok(Some(true))) = matches
    //     .subcommand()
    //     .map(|(_, x)| x.try_get_one::<bool>("dry_run"))
    // {
    //     true
    // } else {
    //     false
    // };

    // Init debug flag
    if matches.get_flag("debug") {
        DEBUG.store(true, Ordering::Relaxed);
    }

    debug!(
        "oma version: {}\n OS: {:#?}",
        env!("CARGO_PKG_VERSION"),
        OsRelease::new()
    );

    let pkgs_getter = |args: &ArgMatches| {
        args.get_many::<String>("packages")
            .map(|x| x.map(|x| x.to_owned()).collect::<Vec<_>>())
    };

    let exit_code = match matches.subcommand() {
        Some(("install", args)) => {
            if !args.get_flag("no_refresh") {
                refresh()?;
            }

            let install_dbg = args.get_flag("install_dbg");
            let pkgs_unparse = pkgs_getter(args).unwrap_or_default();

            let local_debs = pkgs_unparse
                .iter()
                .filter(|x| x.ends_with(".deb"))
                .map(|x| x.to_owned())
                .collect::<Vec<_>>();

            let pkgs_unparse = pkgs_unparse.iter().map(|x| x.as_str()).collect::<Vec<_>>();

            let apt = OmaApt::new(local_debs)?;

            let pkgs = apt.select_pkg(pkgs_unparse, install_dbg, true)?;

            let mut oma_args = OmaArgs::new();
            oma_args.no_fix_broken(args.get_flag("no_fix_broken"));

            apt.install(pkgs, args.get_flag("reinstall"))?;

            let mut apt_args = AptArgs::new();
            let yes = args.get_flag("yes");
            apt_args.yes(yes);
            apt_args.force_yes(args.get_flag("force_yes"));
            // apt_args.dpkg_force_confnew(args.get_flag("dpkg_force_confnew"));
            apt_args.dpkg_force_all(args.get_flag("dpkg_force_all"));

            let op = apt.operation_vec()?;
            let install = op.install;
            let remove = op.remove;
            let disk_size = op.disk_size;

            if install.is_empty() && remove.is_empty() {
                return Ok(0);
            }

            table_for_install_pending(install, remove, disk_size, !yes)?;

            // TODO: network thread
            apt.commit(None, &apt_args, &oma_args)?;

            0
        }
        // OmaCommand::Install(
        //     InstallOptions {
        //     packages: pkgs_getter(args),
        //     install_dbg: args.get_flag("install_dbg"),
        //     reinstall: args.get_flag("reinstall"),
        //     no_fixbroken: args.get_flag("no_fix_broken"),
        //     no_refresh: args.get_flag("no_refresh"),
        //     yes: args.get_flag("yes"),
        //     force_yes: args.get_flag("force_yes"),
        //     force_confnew: args.get_flag("force_confnew"),
        //     dry_run: args.get_flag("dry_run"),
        //     dpkg_force_all: args.get_flag("dpkg_force_all"),
        //     install_recommends: args.get_flag("install_recommends"),
        //     install_suggests: args.get_flag("install_suggests"),
        //     no_install_recommends: args.get_flag("no_install_recommends"),
        //     no_install_suggests: args.get_flag("no_install_suggests"),
        // }
        Some(("upgrade", args)) => {
            refresh()?;

            let pkgs_unparse = pkgs_getter(args).unwrap_or_default();

            let local_debs = pkgs_unparse
                .iter()
                .filter(|x| x.ends_with(".deb"))
                .map(|x| x.to_owned())
                .collect::<Vec<_>>();

            let pkgs_unparse = pkgs_unparse.iter().map(|x| x.as_str()).collect::<Vec<_>>();

            let mut retry_times = 1;

            loop {
                let apt = OmaApt::new(local_debs.clone())?;

                let pkgs = apt.select_pkg(pkgs_unparse.clone(), false, true)?;

                apt.upgrade()?;
                apt.install(pkgs, false)?;

                let oma_args = OmaArgs::new();

                let mut apt_args = AptArgs::new();
                let yes = args.get_flag("yes");
                apt_args.yes(yes);
                apt_args.force_yes(args.get_flag("force_yes"));
                apt_args.dpkg_force_all(args.get_flag("dpkg_force_all"));

                let op = apt.operation_vec()?;
                let install = op.install;
                let remove = op.remove;
                let disk_size = op.disk_size;

                if install.is_empty() && remove.is_empty() {
                    success!("{}", fl!("successfully-refresh"));
                    return Ok(0);
                }

                if retry_times == 1 {
                    table_for_install_pending(install, remove, disk_size, !yes)?;
                }

                match apt.commit(None, &apt_args, &oma_args) {
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

            0
        }
        // OmaCommand::Upgrade(UpgradeOptions {
        //     packages: pkgs_getter(args),
        //     yes: args.get_flag("yes"),
        //     force_yes: args.get_flag("force_yes"),
        //     force_confnew: args.get_flag("force_confnew"),
        //     dry_run: args.get_flag("dry_run"),
        //     dpkg_force_all: args.get_flag("dpkg_force_all"),
        //     no_autoremove: args.get_flag("no_autoremove"),
        // }),
        Some(("download", args)) => {
            let apt = OmaApt::new(vec![])?;
            let pkgs = apt.select_pkg(
                pkgs_getter(args)
                    .unwrap_or_default()
                    .iter()
                    .map(|x| x.as_str())
                    .collect::<Vec<_>>(),
                false,
                true,
            )?;

            let path = args
                .get_one::<String>("path")
                .cloned()
                .map(|x| PathBuf::from(&x));

            apt.download(pkgs, None, path.as_deref())?;

            0
        }
        // OmaCommand::Download(Download {
        //     packages: pkgs_getter(args).unwrap(),
        //     path: args.get_one::<String>("path").cloned(),
        //     with_deps: args.get_flag("with_deps"),
        // }),
        Some(("remove", args)) => {
            let mut apt = OmaApt::new(vec![])?;
            let pkgs = apt.select_pkg(
                pkgs_getter(args)
                    .unwrap_or_default()
                    .iter()
                    .map(|x| x.as_str())
                    .collect::<Vec<_>>(),
                false,
                true,
            )?;

            // TODO: protect
            apt.remove(
                pkgs,
                !args.get_flag("keep_config"),
                true,
                true,
                args.get_flag("no_autoremove"),
            )?;

            let oma_args = OmaArgs::new();

            let mut apt_args = AptArgs::new();
            let yes = args.get_flag("yes");
            apt_args.yes(yes);
            apt_args.force_yes(args.get_flag("force_yes"));

            let op = apt.operation_vec()?;
            let install = op.install;
            let remove = op.remove;
            let disk_size = op.disk_size;

            if install.is_empty() && remove.is_empty() {
                return Ok(0);
            }

            table_for_install_pending(install, remove, disk_size, !yes)?;

            apt.commit(None, &apt_args, &oma_args)?;

            0
        }
        // OmaCommand::Remove(RemoveOptions {
        //     packages: pkgs_getter(args).unwrap(),
        //     yes: args.get_flag("yes"),
        //     force_yes: args.get_flag("force_yes"),
        //     keep_config: args.get_flag("keep_config"),
        //     dry_run: args.get_flag("dry_run"),
        //     no_autoremove: args.get_flag("no_autoremove"),
        // }),
        Some(("refresh", _)) => {
            // TODO: limit
            refresh()?;

            let apt = OmaApt::new(vec![])?;
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

            0
        }
        Some(("show", args)) => {
            let pkgs_unparse = pkgs_getter(args).unwrap_or_default();

            let pkgs_unparse = pkgs_unparse.iter().map(|x| x.as_str()).collect::<Vec<_>>();

            let apt = OmaApt::new(vec![])?;
            let all = args.get_flag("all");
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

            0
        }
        Some(("search", args)) => {
            let apt = OmaApt::new(vec![])?;
            let db = OmaDatabase::new(&apt.cache)?;
            let s = args
                .get_many::<String>("pattern")
                .map(|x| x.map(|x| x.to_owned()).collect::<Vec<_>>())
                .unwrap();

            let s = s.join(" ");

            let res = db.search(&s)?;

            let is_pager = res.len() * 2 > WRITER.get_height() as usize;

            let has_x11 = std::env::var("DISPLAY");

            let tips = if has_x11.is_ok() {
                fl!("normal-tips-with-x11")
            } else {
                fl!("normal-tips")
            };

            let mut pager = Pager::new(!is_pager, &tips)?;
            let mut writer = pager.get_writer()?;
            ALLOWCTRLC.store(true, Ordering::Relaxed);

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

            0
        }
        Some((x, args)) if x == "files" || x == "provides" => {
            let arg = if x == "files" { "package" } else { "pattern" };
            let pkg = args.get_one::<String>(arg).unwrap().to_string();
            let is_bin = args.get_flag("bin");

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

            let mut pager = Pager::new(res.len() < WRITER.get_height().into(), "TODO")?;

            ALLOWCTRLC.store(true, Ordering::Relaxed);
            let mut out = pager.get_writer()?;

            for (_, v) in res {
                writeln!(out, "{v}").ok();
            }

            drop(out);
            pager.wait_for_exit()?;

            0
        }
        Some(("fix-broken", _args)) => {
            let apt = OmaApt::new(vec![])?;
            apt.commit(None, &AptArgs::default(), &OmaArgs::default())?;

            0
        }
        // OmaCommand::FixBroken(FixBroken {
        //     dry_run: args.get_flag("dry_run"),
        // }),
        Some(("pick", args)) => {
            if !args.get_flag("no_refresh") {
                refresh()?;
            }

            let apt = OmaApt::new(vec![])?;
            let pkg_str = args.get_one::<String>("package").unwrap().to_string();

            let pkg = apt.cache.get(&pkg_str).ok_or_else(|| {
                anyhow!(fl!("can-not-get-pkg-from-database", name = pkg_str.clone()))
            })?;

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

            table_for_install_pending(install, remove, disk_size, true)?;

            let mut oma_args = OmaArgs::new();
            oma_args.no_fix_broken(args.get_flag("no_fix_broken"));

            apt.commit(None, &AptArgs::new(), &oma_args)?;

            0
        }
        // OmaCommand::Pick(PickOptions {
        //     package: args.get_one::<String>("package").unwrap().to_string(),
        //     no_fixbroken: args.get_flag("no_fix_broken"),
        //     no_refresh: args.get_flag("no_refresh"),
        //     dry_run: args.get_flag("dry_run"),
        // }),
        Some(("mark", _args)) => todo!(),
        // OmaCommand::Mark(Mark {
        //     action: match args.get_one::<String>("action").map(|x| x.as_str()) {
        //         Some("hold") => MarkAction::Hold(MarkActionArgs {
        //             pkgs: pkgs_getter(args).unwrap(),
        //         }),
        //         Some("unhold") => MarkAction::Unhold(MarkActionArgs {
        //             pkgs: pkgs_getter(args).unwrap(),
        //         }),
        //         Some("auto") => MarkAction::Auto(MarkActionArgs {
        //             pkgs: pkgs_getter(args).unwrap(),
        //         }),
        //         Some("manual") => MarkAction::Manual(MarkActionArgs {
        //             pkgs: pkgs_getter(args).unwrap(),
        //         }),
        //         _ => unreachable!(),
        //     },
        //     dry_run: args.get_flag("dry_run"),
        // }),
        Some(("command-not-found", args)) => {
            let pkg = args.get_one::<String>("package").unwrap().to_string();
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

                    let apt = OmaApt::new(vec![])?;

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

            127
        }
        Some(("list", args)) => {
            let apt = OmaApt::new(vec![])?;
            let pkgs = pkgs_getter(args).unwrap_or_default();

            let mut filter_mode = vec![];

            if args.get_flag("installed") {
                filter_mode.push(FilterMode::Installed);
            }

            if args.get_flag("upgradable") {
                filter_mode.push(FilterMode::Upgradable)
            }

            let filter_pkgs = apt.filter_pkgs(&filter_mode)?;

            let filter_pkgs: Box<dyn Iterator<Item = _>> = if pkgs.is_empty() {
                Box::new(filter_pkgs)
            } else {
                Box::new(filter_pkgs.filter(|x| pkgs.contains(&x.name().to_string())))
            };

            let all = args.get_flag("all");

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

            0
        }
        Some(("depends", _args)) => todo!(),
        // OmaCommand::Depends(Dep {
        //     pkgs: pkgs_getter(args).unwrap(),
        // }),
        Some(("rdepends", _args)) => todo!(),
        // OmaCommand::Rdepends(Dep {
        //     pkgs: pkgs_getter(args).unwrap(),
        // }),
        Some(("clean", _)) => {
            let apt = OmaApt::new(vec![])?;
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

            0
        }
        Some(("history", _args)) => todo!(),
        // OmaCommand::History(History {
        //     action: match args.get_one::<String>("action").map(|x| x.as_str()) {
        //         Some("undo") => HistoryAction::Undo(args.get_one::<usize>("index").copied()),
        //         Some("redo") => HistoryAction::Redo(args.get_one::<usize>("index").copied()),
        //         _ => unimplemented!(),
        //     },
        // }),
        #[cfg(feature = "aosc")]
        Some(("topics", _v)) => todo!(),
        // OmaCommand::Topics(Topics {
        //     opt_in: v
        //         .get_many::<String>("opt_in")
        //         .map(|x| x.map(|x| x.to_owned()).collect::<Vec<_>>()),
        //     opt_out: v
        //         .get_many::<String>("opt_out")
        //         .map(|x| x.map(|x| x.to_owned()).collect::<Vec<_>>()),
        // }),
        Some(("pkgnames", args)) => {
            let apt = OmaApt::new(vec![])?;
            let mut pkgs: Box<dyn Iterator<Item = _>> =
                Box::new(apt.filter_pkgs(&[FilterMode::Names])?);
            let keyword = args.get_one::<String>("keyword").map(|x| x.to_owned());

            if let Some(keyword) = keyword {
                pkgs = Box::new(pkgs.filter(move |x| x.name().starts_with(&keyword)));
            }

            for pkg in pkgs {
                println!("{}", pkg.name());
            }

            0
        }
        _ => unreachable!(),
    };

    Ok(exit_code)
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

fn single_handler() {
    // Kill subprocess
    let subprocess_pid = SUBPROCESS.load(Ordering::Relaxed);
    let allow_ctrlc = ALLOWCTRLC.load(Ordering::Relaxed);
    if subprocess_pid > 0 {
        let pid = nix::unistd::Pid::from_raw(subprocess_pid);
        signal::kill(pid, signal::SIGTERM).expect("Failed to kill child process.");
        if !allow_ctrlc {
            info!("{}", fl!("user-aborted-op"));
        }
    }

    // Dealing with lock
    if LOCKED.load(Ordering::Relaxed) {
        unlock_oma().expect("Failed to unlock instance.");
    }

    // Show cursor before exiting.
    // This is not a big deal so we won't panic on this.
    let _ = WRITER.show_cursor();

    std::process::exit(2);
}
