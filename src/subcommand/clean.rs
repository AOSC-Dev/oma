use crate::fl;
use oma_console::{indicatif::ProgressBar, pb::oma_spinner, success};
use oma_pm::apt::{OmaApt, OmaAptArgsBuilder};

use crate::{error::OutputError, utils::root};

pub fn execute(no_progress: bool) -> Result<i32, OutputError> {
    root()?;

    let oma_apt_args = OmaAptArgsBuilder::default().build()?;
    let apt = OmaApt::new(vec![], oma_apt_args, false)?;
    let download_dir = apt.get_archive_dir();
    let dir = std::fs::read_dir(&download_dir)?;

    let pb = if no_progress {
        let (sty, inv) = oma_spinner(false).unwrap();
        let pb = ProgressBar::new_spinner().with_style(sty);
        pb.enable_steady_tick(inv);
        pb.set_message(fl!("cleaning"));

        Some(pb)
    } else {
        None
    };

    for i in dir.flatten() {
        if i.path().extension().and_then(|x| x.to_str()) == Some("deb") {
            std::fs::remove_file(i.path()).ok();
        }
    }

    let p = download_dir.join("..");
    std::fs::remove_file(p.join("pkgcache.bin")).ok();
    std::fs::remove_file(p.join("srcpkgcache.bin")).ok();

    if let Some(pb) = pb {
        pb.finish_and_clear();
    }

    success!("{}", fl!("clean-successfully"));

    Ok(0)
}
