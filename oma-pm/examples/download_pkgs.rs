use std::path::Path;

use oma_pm::apt::{OmaApt, OmaAptError};
use tracing_subscriber::{EnvFilter, fmt, prelude::__tracing_subscriber_SubscriberExt, util::SubscriberInitExt};

fn main() -> Result<(), OmaAptError> {
    tracing_subscriber::registry()
        .with(fmt::layer())
        .with(EnvFilter::from_default_env())
        .init();

    let apt = OmaApt::new(vec![])?;

    let pkgs = apt.select_pkg(vec!["vscodium", "go"])?;
    std::fs::create_dir_all("./test").unwrap();
    let res = apt.download(pkgs, None, Some(Path::new("test")))?;

    dbg!(res);

    Ok(())
}
