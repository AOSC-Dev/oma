use std::borrow::Cow;

use oma_contents::searcher::{Mode, ripgrep_search};

/// Resolve a host path below the Termux prefix when running there.
fn prefix_path(path: &'static str) -> Cow<'static, str> {
    std::env::var("PREFIX")
        .map(|prefix| Cow::Owned(format!("{prefix}{path}")))
        .unwrap_or(Cow::Borrowed(path))
}

fn main() {
    let lists = prefix_path("/var/lib/apt/lists");
    ripgrep_search(lists.as_ref(), Mode::Files, "apt", |(pkg, file)| {
        println!("{pkg}: {file}")
    })
    .unwrap();
}
