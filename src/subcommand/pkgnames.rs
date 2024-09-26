use oma_pm::apt::{AptConfig, FilterMode, OmaApt, OmaAptArgs};

use crate::error::OutputError;

pub fn execute(
    keyword: Option<&str>,
    sysroot: String,
    installed: bool,
) -> Result<i32, OutputError> {
    let oma_apt_args = OmaAptArgs::builder().sysroot(sysroot).build();
    let apt = OmaApt::new(vec![], oma_apt_args, false, AptConfig::new())?;
    let mut modes = vec![FilterMode::Names];

    if installed {
        modes.push(FilterMode::Installed);
    }

    let mut pkgs: Box<dyn Iterator<Item = _>> = Box::new(apt.filter_pkgs(&modes)?);

    if let Some(keyword) = keyword {
        pkgs = Box::new(pkgs.filter(move |x| x.name().starts_with(keyword)));
    }

    for pkg in pkgs {
        println!("{}", pkg.name());
    }

    Ok(0)
}
