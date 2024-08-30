use crate::fl;
use oma_console::{indicatif::ProgressBar, pb::spinner_style, success};
use oma_pm::apt::{AptConfig, OmaApt, OmaAptArgsBuilder};

use crate::{error::OutputError, utils::root};

pub fn execute(no_progress: bool, sysroot: String) -> Result<i32, OutputError> {
    root()?;

    let apt_config = AptConfig::new();
    let oma_apt_args = OmaAptArgsBuilder::default().sysroot(sysroot).build()?;
    let apt = OmaApt::new(vec![], oma_apt_args, false, apt_config)?;
    let download_dir = apt.get_archive_dir();
    let dir = std::fs::read_dir(&download_dir).map_err(|e| OutputError {
        description: format!("Failed to read dir: {}", download_dir.display()),
        source: Some(Box::new(e)),
    })?;

    let pb = if no_progress {
        let (sty, inv) = spinner_style();
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

    if let Some(pb) = pb {
        pb.finish_and_clear();
    }

    success!("{}", fl!("clean-successfully"));

    Ok(0)
}
