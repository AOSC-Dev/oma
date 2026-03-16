use oma_history::History;

fn main() {
    let conn = History::new("/var/lib/oma/history.db", false, false).unwrap();
    let n = conn.last_upgrade_timestamp().unwrap();

    println!(
        "Last upgrade system date: {}",
        chrono::DateTime::from_timestamp(n, 0).unwrap()
    );
}
