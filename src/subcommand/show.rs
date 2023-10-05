use oma_console::info;
use oma_pm::apt::{OmaApt, OmaAptArgsBuilder};

use crate::error::OutputError;

use super::utils::handle_no_result;
use crate::fl;

pub fn execute(all: bool, pkgs_unparse: Vec<&str>) -> Result<i32, OutputError> {
    let oma_apt_args = OmaAptArgsBuilder::default().build()?;
    let mut apt = OmaApt::new(vec![], oma_apt_args, false)?;
    let (pkgs, no_result) = apt.select_pkg(&pkgs_unparse, false, false)?;
    handle_no_result(no_result);

    if !all {
        if let Some(pkg) = pkgs.first() {
            pkg.print_info(&apt.cache);
        }

        let other_version = pkgs.len() - 1;

        if other_version > 0 {
            info!("{}", fl!("additional-version", len = other_version));
        }
    } else {
        for (i, c) in pkgs.iter().enumerate() {
            if i != pkgs.len() - 1 {
                c.print_info(&apt.cache);
                println!();
            } else {
                c.print_info(&apt.cache);
            }
        }
    }

    Ok(0)
}
