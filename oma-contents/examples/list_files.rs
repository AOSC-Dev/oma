use oma_contents::searcher::{Mode, pure_search};

fn main() {
    pure_search("/var/lib/apt/lists", Mode::Files, "apt", |(pkg, file)| {
        println!("{pkg}: {file}")
    })
    .unwrap();
}
