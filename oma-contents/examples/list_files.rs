use std::{path::Path, time::Duration};

use indicatif::ProgressBar;
use oma_contents::{find, ContentsEvent, QueryMode};
use oma_utils::dpkg::dpkg_arch;

fn main() {
    let pb = ProgressBar::new_spinner();
    pb.enable_steady_tick(Duration::from_millis(100));
    pb.set_message("Searching ...");

    let r = find(
        "apt",
        QueryMode::ListFiles(false),
        Path::new("/var/lib/apt/lists"),
        &dpkg_arch("/").unwrap(),
        move |c| match c {
            ContentsEvent::Progress(c) => {
                pb.set_message(format!("Searching, found {c} results so far ..."))
            }
            ContentsEvent::ContentsMayNotBeAccurate => pb.println("ContentsMayNotBeAccurate"),
            ContentsEvent::Done => pb.finish_and_clear(),
        },
        true,
    );

    dbg!(r.unwrap());
}
