use std::{path::Path, time::Duration};

use indicatif::ProgressBar;
use oma_contents::{find, QueryMode};
use oma_utils::dpkg_arch;

fn main() {
    let pb = ProgressBar::new_spinner();
    pb.enable_steady_tick(Duration::from_millis(100));
    pb.set_message("Searching ...");

    let r = find(
        "apt",
        QueryMode::ListFiles(false),
        Path::new("/var/lib/apt/lists"),
        &dpkg_arch().unwrap(),
        |c| {
            pb.set_message(format!("Searching, found {c} results so far ..."));
        },
    );

    dbg!(r.unwrap());
}
