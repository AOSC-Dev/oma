use oma_pm::apt::{OmaAptError, OmaApt, select_pkg, AptArgs};

fn main() -> Result<(), OmaAptError> {
    let apt = OmaApt::new()?;
    let pkgs = select_pkg(vec!["fish"], &apt.cache)?;

    apt.install(pkgs, false)?;
    let op = apt.operation_vec()?;
    dbg!(op);

    apt.commit(None, AptArgs::default())?;

    Ok(())
}