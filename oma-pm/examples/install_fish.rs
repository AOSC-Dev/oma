use oma_pm::apt::{OmaAptError, OmaApt, select_pkg, AptArgs};
use rust_apt::new_cache;

fn main() -> Result<(), OmaAptError> {
    let apt = OmaApt::new()?;
    let cache = new_cache!()?;
    let pkgs = select_pkg(vec!["fish"], &cache)?;

    apt.install(pkgs, false)?;
    let op = apt.operation_vec()?;
    dbg!(op);

    apt.commit(None, AptArgs::default())?;

    Ok(())
}