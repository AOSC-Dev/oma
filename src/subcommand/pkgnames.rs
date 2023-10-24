use oma_pm::apt::{FilterMode, OmaApt, OmaAptArgsBuilder};

use crate::error::OutputError;

pub fn execute(keyword: Option<&str>, sysroot: String) -> Result<i32, OutputError> {
    let oma_apt_args = OmaAptArgsBuilder::default().sysroot(sysroot).build()?;
    let apt = OmaApt::new(vec![], oma_apt_args, false)?;
    let mut pkgs: Box<dyn Iterator<Item = _>> = Box::new(apt.filter_pkgs(&[FilterMode::Names])?);

    if let Some(keyword) = keyword {
        pkgs = Box::new(pkgs.filter(move |x| x.name().starts_with(keyword)));
    }

    for pkg in pkgs {
        println!("{}", pkg.name());
    }

    Ok(0)
}
