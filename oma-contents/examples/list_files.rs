use oma_contents::searcher::{Mode, pure_search};

/// Resolve a host path below the Termux prefix when running there.
fn prefix_path(path: &str) -> String {
    std::env::var("PREFIX")
        .map(|prefix| format!("{prefix}{path}"))
        .unwrap_or_else(|_| path.to_string())
}

fn main() {
    let lists = prefix_path("/var/lib/apt/lists");
    pure_search(lists.as_str(), Mode::Files, "apt", |(pkg, file)| {
        println!("{pkg}: {file}")
    })
    .unwrap();
}
