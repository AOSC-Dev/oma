use std::time::Duration;

use indicatif::ProgressBar;
use oma_contents::searcher::{pure_search, Mode, OutputMode};

fn main() {
    let pb = ProgressBar::new_spinner();
    pb.enable_steady_tick(Duration::from_millis(100));
    pb.set_message("Searching ...");

    let r = pure_search(
        "/var/lib/apt/lists",
        Mode::Files,
        "apt",
        OutputMode::Progress(Box::new(move |count| {
            pb.set_message(format!("Searching, found {} results ...", count))
        })),
    );

    dbg!(r.unwrap());
}
