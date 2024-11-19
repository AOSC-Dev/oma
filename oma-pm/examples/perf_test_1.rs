use std::time::Instant;

use oma_pm::apt::{AptConfig, OmaApt, OmaAptArgs};

fn main() {
    let apt = OmaApt::new(vec![], OmaAptArgs::builder().build(), false, AptConfig::new()).unwrap();
    
    let timer = Instant::now();
    let count = apt.count_pending_autoremovable_pkgs();
    println!("autoremovable: {}", count);
    println!("{}ms", timer.elapsed().as_millis());

    let timer = Instant::now();
    let count = apt.count_pending_upgradable_pkgs().unwrap();
    println!("upgradable: {}", count);
    println!("{}ms", timer.elapsed().as_millis());
}
