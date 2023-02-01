use action::AoscptAction;

mod pkgversion;
mod update;
mod utils;
mod verify;
mod blackbox;
mod download;
mod action;

fn main() {
    env_logger::init();

    let app = AoscptAction::new().unwrap();

    app.update().unwrap();
    // let client = reqwest::blocking::ClientBuilder::new()
    //     .user_agent("aoscpt")
    //     .build()
    //     .unwrap();

    // update(&client).unwrap();
}
