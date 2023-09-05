use oma_console::info;
use oma_pm::apt::{OmaApt, OmaAptArgsBuilder};

use crate::error::OutputError;

use super::utils::handle_no_result;
use crate::fl;

pub fn execute(all: bool, pkgs_unparse: Vec<&str>) -> Result<i32, OutputError> {
    let oma_apt_args = OmaAptArgsBuilder::default().build()?;
    let mut apt = OmaApt::new(vec![], oma_apt_args, false)?;
    let (pkgs, no_result) = apt.select_pkg(pkgs_unparse, false, false)?;
    handle_no_result(no_result);

    for (i, c) in pkgs.iter().enumerate() {
        if c.is_candidate || all {
            if i != pkgs.len() - 1 {
                println!("{c}\n");
            } else {
                println!("{c}");
            }
        }
    }

    if !all {
        let other_version = pkgs
            .iter()
            .filter(|x| !x.is_candidate)
            .collect::<Vec<_>>()
            .len();

        if other_version > 0 {
            info!("{}", fl!("additional-version", len = other_version));
        }
    }

    Ok(0)
}
