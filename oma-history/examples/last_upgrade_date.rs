use std::borrow::Cow;

use oma_history::History;

/// Resolve a host path below the Termux prefix when running there.
fn prefix_path(path: &'static str) -> Cow<'static, str> {
    std::env::var("PREFIX")
        .map(|prefix| Cow::Owned(format!("{prefix}{path}")))
        .unwrap_or(Cow::Borrowed(path))
}

fn main() {
    let db = prefix_path("/var/lib/oma/history.db");
    let conn = History::new(db.as_ref(), false, false).unwrap();
    let n = conn.last_upgrade_timestamp().unwrap();

    println!(
        "Last upgrade system date: {}",
        jiff::Timestamp::from_second(n).unwrap()
    );
}
