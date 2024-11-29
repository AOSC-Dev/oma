use std::{path::Path, thread};

use apt_auth_config::AuthConfig;
use flume::unbounded;
use oma_fetch::{reqwest::ClientBuilder, Event};
use oma_pm::{
    apt::{AptConfig, DownloadConfig, OmaApt, OmaAptArgs, OmaAptError},
    matches::PackagesMatcher,
};
use oma_utils::dpkg::dpkg_arch;

fn main() -> Result<(), OmaAptError> {
    let oma_apt_args = OmaAptArgs::builder().build();
    let apt = OmaApt::new(vec![], oma_apt_args, false, AptConfig::new())?;
    let arch = dpkg_arch("/").unwrap();

    let matcher = PackagesMatcher::builder()
        .cache(&apt.cache)
        .filter_candidate(true)
        .filter_downloadable_candidate(false)
        .select_dbg(false)
        .native_arch(&arch)
        .build();

    let pkgs = matcher.match_pkgs_and_versions(["vscodium", "go"])?;

    std::fs::create_dir_all("./test").unwrap();

    let client = ClientBuilder::new().user_agent("oma").build().unwrap();

    let (tx, rx) = unbounded();

    thread::spawn(move || {
        while let Ok(v) = rx.recv() {
            println!("{:#?}", v);
            if let Event::AllDone = v {
                break;
            }
        }
    });

    let res = apt.download(
        &client,
        pkgs.0,
        DownloadConfig {
            network_thread: None,
            download_dir: Some(Path::new("test")),
            auth: &AuthConfig::system("/").unwrap(),
        },
        tx,
        false,
    )?;

    dbg!(res);

    Ok(())
}
