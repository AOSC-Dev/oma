use std::path::Path;

use oma_pm::apt::{OmaApt, OmaAptArgsBuilder, OmaAptError};
use tracing_subscriber::{
    fmt, prelude::__tracing_subscriber_SubscriberExt, util::SubscriberInitExt, EnvFilter,
};

fn main() -> Result<(), OmaAptError> {
    tracing_subscriber::registry()
        .with(fmt::layer())
        .with(EnvFilter::from_default_env())
        .init();

    let oma_apt_args = OmaAptArgsBuilder::default().build().unwrap();
    let apt = OmaApt::new(vec![], oma_apt_args)?;

    let pkgs = apt.select_pkg(vec!["vscodium", "go"], false, true)?;
    std::fs::create_dir_all("./test").unwrap();
    let res = apt.download(pkgs, None, Some(Path::new("test")))?;

    dbg!(res);

    Ok(())
}
