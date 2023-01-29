use update::update;

mod update;
mod utils;
mod pkgversion;
mod verify;

fn main() {
    env_logger::init();

    update().unwrap();
}
