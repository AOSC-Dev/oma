use std::{
    io::BufReader,
    process::{Command, Stdio},
};

use anyhow::{Context, Result};

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
    name: String,
    action: AptAction,
    now_version: Option<String>,
    info: Option<AptPackageInfo>,
}

#[derive(Debug)]
pub struct AptPackageInfo {
    new_version: String,
    branch_and_arch: String,
}

#[derive(Debug, PartialEq, Eq)]
pub enum AptAction {
    Install,
    Remove,
    Configure,
    Purge,
}

pub fn apt_calc(list: &[Package]) -> Result<Vec<AptPackage>> {
    let mut result = vec![];
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

    for i in output.lines() {
        let mut ready = i.split_whitespace();

        let action = match ready.nth(0) {
            Some("Inst") => AptAction::Install,
            Some("Remv") => AptAction::Remove,
            Some("Conf") => AptAction::Configure,
            Some("Purg") => AptAction::Purge,
            _ => continue,
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

                *info = Some(AptPackageInfo {
                    new_version,
                    branch_and_arch,
                })
            }

            Ok(())
        }

        if let Some(same_item) = same_item {
            if same_item.starts_with("[") && same_item.ends_with("]") {
                now_version = same_item
                    .strip_prefix("[")
                    .and_then(|x| x.strip_suffix("]"))
                    .map(|x| x.to_string());
                let ready = ready_split.collect::<Vec<_>>().join(" ");

                if ready.starts_with("(") && ready.ends_with(")") {
                    info_inner(ready, &mut info)?;
                }
            } else {
                info_inner(ready, &mut info)?;
            }
        } else {
            info_inner(ready, &mut info)?;
        }
        // if action == AptAction::Configure {
        //     info = same_item_2.strip_prefix("(").and_then(|x| x.strip_suffix(")")).map(|x| x.to_string());
        // }

        // let same_item_2 = ready.nth(0);

        // if let Some(same_item_2) = same_item_2 {
        //     if same_item_2.starts_with("(") && same_item_2.ends_with(")") {
        //         info = same_item_2.strip_prefix("(").and_then(|x| x.strip_suffix(")")).map(|x| x.to_string());
        //     }
        // }

        result.push(AptPackage {
            name,
            action,
            now_version,
            info,
        });
    }

    Ok(result)
}

#[test]
fn test() {
    let test = Package {
        name: "kodi".to_string(),
        action: Action::Install,
    };

    let test = apt_calc(&[test]).unwrap();
    dbg!(test);
}
