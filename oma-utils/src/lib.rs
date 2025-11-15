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
    std::env::var("TERMUX_VERSION").is_ok_and(|v| !v.is_empty())
}

#[inline]
pub fn concat_url(url: &str, path: &str) -> String {
    format!(
        "{}/{}",
        url.trim_end_matches('/'),
        path.trim_start_matches('/')
    )
}
