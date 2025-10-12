pub use os_release::OsRelease;

#[cfg(feature = "dbus")]
pub mod dbus;
#[cfg(feature = "dbus")]
pub use zbus;

#[cfg(feature = "dpkg")]
pub mod dpkg;
#[cfg(feature = "human-bytes")]
pub mod human_bytes;
#[cfg(feature = "url-no-escape")]
pub mod url_no_escape;

#[inline]
pub fn is_termux() -> bool {
    std::env::var("TERMUX__PREFIX").is_ok_and(|v| !v.is_empty())
}
