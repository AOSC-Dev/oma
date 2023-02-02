use std::{path::Path, process::Command};

use anyhow::{bail, Context, Result};
use indexmap::IndexMap;
use log::debug;
use rust_apt::{cache::Upgrade, config, new_cache};

use crate::update::{debcontrol_from_file, APT_LIST_DISTS, DOWNLOAD_DIR};

#[derive(Debug)]
pub struct Package {
    pub name: String,
    pub action: Action,
}

#[derive(Debug)]
pub enum Action {
    Install,
    Remove,
}

#[derive(Debug)]
pub struct AptPackage {
    pub name: String,
    pub action: AptAction,
    pub now_version: Option<String>,
    pub info: Option<AptPackageInfo>,
    pub is_auto: bool,
}

#[derive(Debug, Clone)]
pub struct AptPackageInfo {
    pub new_version: String,
    pub distro: String,
    pub branch: String,
    pub arch: String,
}

#[derive(Debug, PartialEq, Eq)]
pub enum AptAction {
    Install,
    Remove,
    Configure,
    Purge,
}

/// Use apt -s to calculate dependencies and action order (Install after Configure or Purge after install or ...?)
pub fn apt_install_calc(list: &[Package]) -> Result<Vec<AptPackage>> {
    config::init_config_system();

    let names = list.iter().map(|x| x.name.clone()).collect::<Vec<_>>();
    let cache = new_cache!()?;

    for i in names {
        let pkg = cache.get(&i).unwrap();

        pkg.protect();
        pkg.mark_install(true, true);
    }

    cache.resolve(false).unwrap();
    let action = cache.get_changes(true);

    // let mut res = vec![];

    for pkg in action {
        if pkg.marked_install() {
            println!("{} is marked install", pkg.name());
            // If the package is marked install then it will also
            // show up as marked upgrade, downgrade etc.
            // Check this first and continue.
            continue;
        }
        if pkg.marked_upgrade() {
            println!("{} is marked upgrade", pkg.name())
        }
        if pkg.marked_delete() {
            println!("{} is marked remove", pkg.name())
        }
        if pkg.marked_reinstall() {
            println!("{} is marked reinstall", pkg.name())
        }
        if pkg.marked_downgrade() {
            println!("{} is marked downgrade", pkg.name())
        }
    }

    todo!()
}

// fn dpkg_executer(action_list: &[AptPackage], download_dir: Option<&str>) -> Result<()> {
//     let extend_file = Path::new("/var/lib/apt/extended_states");

//     if !extend_file.is_file() {
//         std::fs::create_dir_all(APT_LIST_DISTS)?;
//         std::fs::File::create(&extend_file)?;
//     }

//     let mut extend = debcontrol_from_file(extend_file)?;

//     for pkg in action_list {
//         if pkg.marked_install() {
//             println!("{} is marked install", pkg.name());
//             // If the package is marked install then it will also
//             // show up as marked upgrade, downgrade etc.
//             // Check this first and continue.
//             continue;
//         }
//         if pkg.marked_upgrade() {
//             println!("{} is marked upgrade", pkg.name())
//         }
//         if pkg.marked_delete() {
//             println!("{} is marked remove", pkg.name())
//         }
//         if pkg.marked_reinstall() {
//             println!("{} is marked reinstall", pkg.name())
//         }
//         if pkg.marked_downgrade() {
//             println!("{} is marked downgrade", pkg.name())
//         }
        // match i.action {
        //     AptAction::Install => {
        //         let mut cmd = dpkg_cmd();

        //         let name = i.name.as_str();
        //         let info = i
        //             .info
        //             .clone()
        //             .take()
        //             .context(format!("Unexpect error in package: {}", i.name))?;

        //         let version = info.new_version;

        //         // Handle 1.0.0-0
        //         let version_rev_split = version.split_once("-");

        //         let version = if version_rev_split.is_none() {
        //             format!("{}-{}", version, 0)
        //         } else {
        //             let (_, rev) = version_rev_split.unwrap();

        //             // FIXME: 6.01+posix2017-a
        //             if rev.parse::<u64>().is_err() {
        //                 format!("{}-{}", version, 0)
        //             } else {
        //                 version.to_owned()
        //             }
        //         };

        //         let version_epoch_split = version.split_once(":");
        //         // Handle epoch
        //         let version = if let Some((_, version)) = version_epoch_split {
        //             version.to_string()
        //         } else {
        //             version
        //         };

        //         let arch = if info.arch == "[all]" {
        //             "noarch"
        //         } else {
        //             &info
        //                 .arch
        //                 .strip_prefix("[")
        //                 .and_then(|x| x.strip_suffix("]"))
        //                 .take()
        //                 .context("Can not parse info arch!")?
        //         };

        //         let filename = format!("{}_{}_{}.deb", name, version, arch);

        //         cmd.arg("--unpack");
        //         cmd.arg(format!(
        //             "{}/{}",
        //             download_dir.unwrap_or(DOWNLOAD_DIR),
        //             filename
        //         ));

        //         dpkg_execute_ineer(&mut cmd)?;
        //     }
        //     AptAction::Configure => {
        //         let mut cmd = dpkg_cmd();
        //         cmd.arg("--configure");
        //         cmd.arg(i.name.clone());

        //         dpkg_execute_ineer(&mut cmd)?;

        //         if i.is_auto {
        //             let mut item = IndexMap::new();
        //             item.insert("Package".to_owned(), i.name.to_string());
        //             item.insert("Architecture".to_owned(), i.name.to_string());
        //             item.insert("Auto-Installed".to_owned(), "1".to_owned());

        //             extend.push(item);
        //         }
        //     }

        //     AptAction::Remove => {
        //         let mut cmd = dpkg_cmd();
        //         cmd.arg("--remove");
        //         cmd.arg(i.name.clone());

        //         dpkg_execute_ineer(&mut cmd)?;

        //         let index = extend
        //             .iter()
        //             .position(|x| x.get("Package") == Some(&i.name));

        //         if let Some(index) = index {
        //             extend.remove(index);
        //         }
        //     }
        //     AptAction::Purge => {
        //         let mut cmd = dpkg_cmd();
        //         cmd.arg("--purge");
        //         cmd.arg(i.name.clone());

        //         dpkg_execute_ineer(&mut cmd)?;

        //         let index = extend
        //             .iter()
        //             .position(|x| x.get("Package") == Some(&i.name));

        //         if let Some(index) = index {
        //             extend.remove(index);
        //         }
        //     }
        // }
//     }

//     let s = parse_back(&extend);
//     std::fs::write("/var/lib/apt/extended_states", s)?;

//     Ok(())
// }

fn parse_back(list: &[IndexMap<String, String>]) -> String {
    let mut s = String::new();
    for i in list {
        for (k, v) in i {
            s += &format!("{}: ", k);
            s += &format!("{}\n", v);
        }

        s += "\n";
    }

    s
}

fn dpkg_cmd() -> Command {
    let mut cmd = Command::new("dpkg");

    // Ignore dependency/break and checks. These will be guaranteed by aoscpt
    cmd.args(&[
        "--force-downgrade",
        "--force-breaks",
        "--force-conflicts",
        "--force-depends",
    ]);

    cmd
}

pub fn dpkg_run(list: &[AptPackage]) -> Result<()> {
    let mut count = 0;

    // If have errpr, retry 3 times
    // while let Err(e) = dpkg_executer(list, None) {
    //     if count == 3 {
    //         return Err(e);
    //     }

    //     count += 1;
    // }

    Ok(())
}

fn dpkg_execute_ineer(cmd: &mut Command) -> Result<()> {
    let res = cmd.status().context("Failed to execute dpkg command(s).")?;

    if !res.success() {
        match res.code() {
            Some(code) => bail!("dpkg exited with non-zero return code: {}.", code),
            None => bail!("dpkg process was terminated by signal."),
        }
    }

    Ok(())
}
