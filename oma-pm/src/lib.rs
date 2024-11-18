pub mod apt;
pub mod matches;
pub mod pkginfo;
pub mod progress;
pub mod search;
pub use oma_apt::error::AptErrors;
pub use oma_apt::PkgCurrentState;
pub use search::PackageStatus;
mod dbus;

#[cfg(test)]
mod test {
    use std::sync::{LazyLock, Mutex};
    pub(crate) static TEST_LOCK: LazyLock<Mutex<()>> = LazyLock::new(|| Mutex::new(()));
}
