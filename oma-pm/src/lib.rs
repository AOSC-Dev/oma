pub mod apt;
pub mod pkginfo;
mod progress;
pub mod query;
pub mod search;
pub use search::PackageStatus;
pub mod unmet;

pub fn format_description(desc: &str) -> (String, Option<String>) {
    if let Some((short, long)) = desc.split_once('\n') {
        (short.to_string(), Some(long.to_string()))
    } else {
        (desc.to_string(), None)
    }
}
