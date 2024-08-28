use oma_contents::searcher::{pure_search, Mode};

fn main() {
    pure_search("/var/lib/apt/lists", Mode::Files, "apt", |(pkg, file)| {
        println!("{pkg}: {file}")
    })
    .unwrap();
}
