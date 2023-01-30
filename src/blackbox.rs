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
}

#[derive(Debug)]
pub enum AptAction {
    Install,
    Remove,
    Configure,
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
        let action = if i.starts_with("Inst ") {
            AptAction::Install
        } else if i.starts_with("Remv ") {
            AptAction::Remove
        } else if i.starts_with("Conf ") {
            AptAction::Configure
        } else {
            continue;
        };

        let mut ready = i.split_whitespace();
        let name = ready.nth(1).take().context("invalid apt Action!")?.to_string();

        result.push(AptPackage { name, action })
    }

    Ok(result)
}

#[test]
fn test() {
    let test = Package {
        name: "go".to_string(),
        action: Action::Install,
    };

    let test = apt_calc(&[test]).unwrap();
    dbg!(test);
}
