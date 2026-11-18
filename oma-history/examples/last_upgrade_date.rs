use oma_history::History;

/// Resolve a host path below the Termux prefix when running there.
fn prefix_path(path: &str) -> String {
    std::env::var("PREFIX")
        .map(|prefix| format!("{prefix}{path}"))
        .unwrap_or_else(|_| path.to_string())
}

fn main() {
    let db = prefix_path("/var/lib/oma/history.db");
    let conn = History::new(db.as_str(), false, false).unwrap();
    let n = conn.last_upgrade_timestamp().unwrap();

    println!(
        "Last upgrade system date: {}",
        jiff::Timestamp::from_second(n).unwrap()
    );
}
