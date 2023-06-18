use std::{path::Path, time::Duration};

use indicatif::ProgressBar;
use oma_contents::{find, Event, Request};

fn main() {
    let (tx, rx) = std::sync::mpsc::channel();
    let req = Request {
        kw: "apt",
        is_list: true,
        cnf: false,
        only_bin: false,
        dists_dir: Path::new("/var/lib/apt/lists"),
        arch: "amd64",
        event: Some(tx),
    };

    let pb = ProgressBar::new_spinner();
    pb.enable_steady_tick(Duration::from_millis(100));

    let r = std::thread::spawn(move || find(req));

    while let Ok(log) = rx.recv() {
        match log {
            Event::Info(s) => pb.set_message(format!("INFO {s}")),
            Event::Warn(s) => pb.set_message(format!("WARN {s}")),
            Event::Error(s) => pb.set_message(format!("ERROR {s}")),
            Event::Searching => pb.set_message("Searching ..."),
            Event::SearchingWithResult(index) => {
                pb.set_message(format!("Searching, found {index} results so far ..."))
            }
            Event::Done => pb.set_message("DONE."),
        }
    }

    println!("{:?}", r.join().unwrap());
}
