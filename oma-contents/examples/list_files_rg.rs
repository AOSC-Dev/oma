use oma_contents::searcher::{Mode, ripgrep_search};

fn main() {
    ripgrep_search("/var/lib/apt/lists", Mode::Files, "apt", |(pkg, file)| {
        println!("{pkg}: {file}")
    })
    .unwrap();
}
