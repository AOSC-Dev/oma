use std::path::Path;

use oma_pm::apt::{OmaApt, OmaAptArgsBuilder, OmaAptError};

fn main() -> Result<(), OmaAptError> {
    let oma_apt_args = OmaAptArgsBuilder::default().build().unwrap();
    let apt = OmaApt::new(vec![], oma_apt_args, false)?;

    let pkgs = apt.select_pkg(vec!["vscodium", "go"], false, true)?;
    std::fs::create_dir_all("./test").unwrap();
    let res = apt.download(pkgs, None, Some(Path::new("test")), false)?;

    dbg!(res);

    Ok(())
}
