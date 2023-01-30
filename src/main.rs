use update::update;

mod pkgversion;
mod update;
mod utils;
mod verify;
mod blackbox;

fn main() {
    env_logger::init();
    let client = reqwest::blocking::ClientBuilder::new()
        .user_agent("aoscpt")
        .build()
        .unwrap();

    update(&client).unwrap();
}
