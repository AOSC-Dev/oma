pub mod apt;
pub mod pkginfo;
mod progress;
pub mod query;
pub mod search;
pub use search::PackageStatus;
mod dbus;
pub mod unmet;

pub fn format_description(desc: &str) -> (&str, Option<&str>) {
    if let Some((short, long)) = desc.split_once('\n') {
        (short, Some(long))
    } else {
        (desc, None)
    }
}
