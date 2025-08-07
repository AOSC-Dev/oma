use oma_history::{connect_db, last_upgrade_timestamp};

fn main() {
    let conn = connect_db("/var/lib/oma/history.db", false).unwrap();
    let n = last_upgrade_timestamp(&conn).unwrap();

    println!(
        "Last upgrade system date: {}",
        chrono::DateTime::from_timestamp(n, 0).unwrap()
    );
}
