use apt_history::{Operation, parse_from_file};
use std::borrow::Cow;

fn main() {
    let history = parse_from_file("/var/log/apt/history.log").unwrap();
    for (index, entry) in history.iter().enumerate() {
        println!(
            "ID: {} Command line: {}, Date and Time: {}, Action: {}, Changes: {}",
            index,
            entry.command_line.as_deref().unwrap_or(""),
            entry.start_date,
            {
                let action = entry.action();
                if action.len() == 1 {
                    Cow::Borrowed(match action.iter().next().unwrap() {
                        Operation::Install => "Install",
                        Operation::Upgrade => "Upgrade",
                        Operation::Remove => "Remove",
                        Operation::Reinstall => "Reinstall",
                        Operation::Downgrade => "Downgrade",
                        Operation::Purge => "Purge",
                    })
                } else {
                    action
                        .iter()
                        .map(|op| match op {
                            Operation::Install => "I",
                            Operation::Upgrade => "U",
                            Operation::Remove => "R",
                            Operation::Reinstall => "Re",
                            Operation::Downgrade => "D",
                            Operation::Purge => "P",
                        })
                        .collect::<Vec<_>>()
                        .join(",")
                        .into()
                }
            },
            entry.changes()
        );
        println!("  Install: {:?}", entry.install);
        println!("  Upgrade: {:?}", entry.upgrade);
        println!("  Remove: {:?}", entry.remove);
        println!("  Reinstall: {:?}", entry.reinstall);
        println!("  Downgrade: {:?}", entry.downgrade);
        println!("  Purge: {:?}", entry.purge);
    }
}
