use oma_history::History;

fn main() {
    let conn = History::new("/var/lib/oma/history.db", false, false).unwrap();
    let n = conn.last_upgrade_timestamp().unwrap();

    println!(
        "Last upgrade system date: {}",
        jiff::Timestamp::from_second(n).unwrap()
    );
}
