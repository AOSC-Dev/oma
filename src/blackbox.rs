use std::{
    collections::HashMap,
    fmt::format,
    io::Read,
    path::Path,
    process::{Command, Stdio},
};

use anyhow::{bail, Context, Result};
use eight_deep_parser::{parse_back, IndexMap, Item};
use log::debug;

use crate::{
    update::{APT_LIST_DISTS, DOWNLOAD_DIR},
    utils::get_arch_name,
};

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
    let names = list.iter().map(|x| x.name.clone()).collect::<Vec<_>>();

    let command = Command::new("apt-get")
        .arg("install")
        .arg("-s")
        .args(&names)
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .output()
        .context("could not launch `apt-get` process")?;

    let output = std::str::from_utf8(&command.stdout)?;

    let result = parse_apt_simu(output, Some(list))?;

    Ok(result)
}

fn info_inner(ready: String, info: &mut Option<AptPackageInfo>) -> Result<()> {
    let info_str = ready
        .strip_prefix("(")
        .and_then(|x| x.strip_suffix(")"))
        .map(|x| x.to_string());

    if let Some(info_str) = info_str {
        let mut info_str_split = info_str.split_whitespace();
        let new_version = info_str_split
            .nth(0)
            .take()
            .context("Unsupported item!")?
            .to_string();
        let branch_and_arch = info_str_split.collect::<Vec<_>>().join(" ").to_string();

        let mut baa_split = branch_and_arch.split_once(":");
        let (distro, after) = baa_split
            .take()
            .context("Can not get distro branch and arch!")?;

        let distro = distro.to_string();
        let mut after_split = after.split_whitespace();

        let branch = after_split
            .nth(0)
            .context("Can not get distro branch!")?
            .to_string();

        let arch = after_split
            .nth(0)
            .context("Can not get distro branch!")?
            .to_string();

        *info = Some(AptPackageInfo {
            new_version,
            distro,
            branch,
            arch,
        });
    }

    Ok(())
}

fn parse_apt_simu(output: &str, list: Option<&[Package]>) -> Result<Vec<AptPackage>> {
    let mut result = vec![];

    for i in output.lines() {
        let res = parse_simu_inner(i, list);

        if let Ok(res) = res {
            result.push(res);
        } else {
            continue;
        }
    }

    Ok(result)
}

fn parse_simu_inner(i: &str, list: Option<&[Package]>) -> Result<AptPackage> {
    let mut ready = i.split_whitespace();

    let action = match ready.nth(0) {
        Some("Inst") => AptAction::Install,
        Some("Remv") => AptAction::Remove,
        Some("Conf") => AptAction::Configure,
        Some("Purg") => AptAction::Purge,
        Some(x) => {
            debug!("Useless line: {}", x);
            bail!("Useless line: {}", x);
        }
        _ => bail!(""),
    };

    let name = ready
        .nth(0)
        .take()
        .context("invalid apt Action!")?
        .to_string();

    let ready = ready.collect::<Vec<_>>().join(" ");

    let mut ready_split = ready.split_whitespace();
    let same_item = ready_split.nth(0);

    let mut now_version = None;
    let mut info = None;

    if let Some(same_item) = same_item {
        if same_item.starts_with("[") && same_item.ends_with("]") {
            now_version = same_item
                .strip_prefix("[")
                .and_then(|x| x.strip_suffix("]"))
                .map(|x| x.to_string());
            let ready = ready_split.collect::<Vec<_>>().join(" ");

            let ready = if ready.ends_with("[]") {
                ready.strip_suffix(" []").unwrap().to_string()
            } else {
                ready
            };

            if ready.starts_with("(") && ready.ends_with(")") {
                info_inner(ready, &mut info)?;
            }
        } else {
            info_inner(ready, &mut info)?;
        }
    } else {
        info_inner(ready, &mut info)?;
    }

    let is_auto = if let Some(list) = list {
        if list.iter().find(|x| x.name == name).is_none() {
            true
        } else {
            false
        }
    } else {
        false
    };

    let res = AptPackage {
        name,
        action,
        now_version,
        info,
        is_auto,
    };

    Ok(res)
}

fn dpkg_executer(action_list: &[AptPackage], download_dir: Option<&str>) -> Result<()> {
    let mut s = String::new();
    let p = Path::new("/var/lib/apt/extended_states");

    if !p.is_file() {
        std::fs::create_dir_all(APT_LIST_DISTS)?;
        std::fs::File::create(&p)?;
    }

    let mut f = std::fs::File::open("/var/lib/apt/extended_states")?;
    f.read_to_string(&mut s)?;
    let mut extend = eight_deep_parser::parse_multi(&s)?;

    for i in action_list {
        match i.action {
            AptAction::Install => {
                let mut cmd = Command::new("dpkg");

                // Ignore dependency/break and checks. These will be guaranteed by aoscpt
                cmd.args(&[
                    "--force-downgrade",
                    "--force-breaks",
                    "--force-conflicts",
                    "--force-depends",
                ]);
                let name = i.name.as_str();
                let info = i
                    .info
                    .clone()
                    .take()
                    .context(format!("Unexpect error in package: {}", i.name))?;

                let version = info.new_version;

                // Handle 1.0.0-0
                let version_rev_split = version.split_once("-");

                let version = if version_rev_split.is_none() {
                    format!("{}-{}", version, 0)
                } else {
                    let (_, rev) = version_rev_split.unwrap();

                    // FIXME: 6.01+posix2017-a
                    if rev.parse::<u64>().is_err() {
                        format!("{}-{}", version, 0)
                    } else {
                        version.to_owned()
                    }
                };

                let version_epoch_split = version.split_once(":");
                // Handle epoch
                let version = if let Some((_, version)) = version_epoch_split {
                    version.to_string()
                } else {
                    version
                };

                let arch = if info.arch == "[all]" {
                    "noarch"
                } else {
                    &info
                        .arch
                        .strip_prefix("[")
                        .and_then(|x| x.strip_suffix("]"))
                        .take()
                        .context("Can not parse info arch!")?
                };

                let filename = format!("{}_{}_{}.deb", name, version, arch);

                cmd.arg("--unpack");
                cmd.arg(format!(
                    "{}/{}",
                    download_dir.unwrap_or(DOWNLOAD_DIR),
                    filename
                ));

                dpkg_execute_ineer(&mut cmd)?;
            }
            AptAction::Configure => {
                let mut cmd = Command::new("dpkg");

                // Ignore dependency/break and checks. These will be guaranteed by aoscpt
                cmd.args(&[
                    "--force-downgrade",
                    "--force-breaks",
                    "--force-conflicts",
                    "--force-depends",
                ]);
                cmd.arg("--configure");
                cmd.arg(i.name.clone());

                dpkg_execute_ineer(&mut cmd)?;

                if i.is_auto {
                    let mut entry = IndexMap::new();

                    entry.insert("Package".to_string(), Item::OneLine(i.name.to_owned()));
                    entry.insert(
                        "Architecture".to_string(),
                        Item::OneLine(
                            get_arch_name()
                                .take()
                                .context("Can not get architecture")?
                                .to_string(),
                        ),
                    );
                    entry.insert("Auto-Installed".to_string(), Item::OneLine("1".to_string()));

                    extend.push(entry);
                }
            }

            AptAction::Remove => todo!(),
            AptAction::Purge => todo!(),
        }
    }

    let s = parse_back(&extend);
    std::fs::write("/var/lib/apt/extended_states", s)?;

    Ok(())
}

pub fn dpkg_run(list: &[AptPackage]) -> Result<()> {
    let mut count = 0;
    while let Err(e) = dpkg_executer(list, None) {
        if count == 3 {
            return Err(e);
        }

        count += 1;
    }

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

#[test]
fn test() {
    let test = "Inst samba [4.14.2-2] (4.17.2-1 AOSC OS:stable [amd64]) []";

    let test = parse_simu_inner(test, None);
    dbg!(test);
}
