pub mod apt;
pub mod pkginfo;
pub mod progress;
pub mod query;
pub mod search;
pub use oma_apt::error::AptErrors;
pub use oma_apt::PkgCurrentState;
pub use search::PackageStatus;
mod dbus;

pub fn format_description(desc: &str) -> (&str, Option<&str>) {
    if let Some((short, long)) = desc.split_once('\n') {
        (short, Some(long))
    } else {
        (desc, None)
    }
}

#[cfg(test)]
mod test {
    use std::sync::{LazyLock, Mutex};
    pub(crate) static TEST_LOCK: LazyLock<Mutex<()>> = LazyLock::new(|| Mutex::new(()));
}
