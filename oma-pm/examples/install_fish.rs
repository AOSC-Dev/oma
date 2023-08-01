use oma_pm::apt::{AptArgs, OmaApt, OmaAptArgsBuilder, OmaAptError};

fn main() -> Result<(), OmaAptError> {
    let oma_apt_args = OmaAptArgsBuilder::default().build().unwrap();
    let apt = OmaApt::new(vec![], oma_apt_args)?;
    let pkgs = apt.select_pkg(vec!["fish"], false, true)?;

    apt.install(pkgs, false, true, false)?;
    // let op = apt.operation_vec()?;

    apt.commit(None, &AptArgs::default(), false)?;

    Ok(())
}
