use oma_pm::apt::{AptArgs, OmaApt, OmaAptError};

fn main() -> Result<(), OmaAptError> {
    let apt = OmaApt::new(vec![])?;
    let pkgs = apt.select_pkg(vec!["fish"], false, true)?;

    apt.install(pkgs, false)?;
    // let op = apt.operation_vec()?;

    apt.commit(None, &AptArgs::default(), false)?;

    Ok(())
}
