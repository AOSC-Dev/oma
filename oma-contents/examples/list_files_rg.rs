use oma_contents::searcher::{ripgrep_search, Mode};

fn main() {
    ripgrep_search("/var/lib/apt/lists", Mode::Files, "apt", |(pkg, file)| {
        println!("{pkg}: {file}")
    })
    .unwrap();
}
