use oma_pm::apt::{AptArgs, OmaApt, OmaAptError, OmaArgs};

fn main() -> Result<(), OmaAptError> {
    let apt = OmaApt::new(vec![])?;
    let pkgs = apt.select_pkg(vec!["fish"], false)?;

    apt.install(pkgs, false)?;
    // let op = apt.operation_vec()?;

    apt.commit(None, &AptArgs::default(), &OmaArgs::default())?;

    Ok(())
}
