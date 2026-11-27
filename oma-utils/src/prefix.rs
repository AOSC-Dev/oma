//! Runtime locations and prefix-aware host path mapping.
//!
//! Not every Termux installation lives at the default
//! `/data/data/com.termux/files/usr` prefix (forks and custom app data
//! directories exist), so the prefix is resolved from the environment instead
//! of being hardcoded. `$TERMUX__PREFIX` is the modern variable; `$PREFIX` is
//! kept for older Termux releases.

use std::{
    env,
    path::{Path, PathBuf},
};

use crate::is_termux;

/// Prefix used when the environment does not provide one.
pub const DEFAULT_PREFIX: &str = "/data/data/com.termux/files/usr";

/// App data directory used when it cannot be derived from the prefix.
const DEFAULT_DATA_DIR: &str = "/data/data/com.termux";

/// Configuration file used by non-Termux builds.
const SYSTEM_CONFIG_FILE: &str = "/etc/oma.toml";

fn env_path(key: &str) -> Option<PathBuf> {
    env::var_os(key)
        .map(PathBuf::from)
        .filter(|path| path.is_absolute())
}

/// The Termux installation prefix (`$PREFIX`).
pub fn prefix() -> PathBuf {
    env_path("TERMUX__PREFIX")
        .or_else(|| env_path("PREFIX"))
        .unwrap_or_else(|| PathBuf::from(DEFAULT_PREFIX))
}

/// The Termux app data directory, i.e. the parent of `$PREFIX/files`.
pub fn data_dir() -> PathBuf {
    data_dir_from(&prefix()).unwrap_or_else(|| PathBuf::from(DEFAULT_DATA_DIR))
}

fn data_dir_from(prefix: &Path) -> Option<PathBuf> {
    prefix.parent()?.parent().map(Path::to_path_buf)
}

/// oma's configuration file: `$PREFIX/etc/oma.toml` on Termux and
/// `/etc/oma.toml` elsewhere.
pub fn oma_config_file() -> PathBuf {
    if is_termux() {
        prefix().join("etc/oma.toml")
    } else {
        PathBuf::from(SYSTEM_CONFIG_FILE)
    }
}

/// apt's `sources.list` under the Termux prefix.
pub fn apt_sources_list() -> PathBuf {
    prefix().join("etc/apt/sources.list")
}

/// apt's `sources.list.d` under the Termux prefix.
pub fn apt_sources_list_d() -> PathBuf {
    prefix().join("etc/apt/sources.list.d")
}

/// apt's authorization configuration directory.
pub fn apt_auth_conf_d() -> PathBuf {
    host_path(Path::new("/"), "/etc/apt/auth.conf.d")
}

/// apt's archive directory under the Termux app data directory.
pub fn apt_archive_dir() -> PathBuf {
    data_dir().join("cache/apt/archives")
}

/// oma's system plugin directory.
pub fn libexec_dir() -> PathBuf {
    host_path(Path::new("/"), "/usr/libexec")
}

/// oma's local plugin directory.
pub fn local_libexec_dir() -> PathBuf {
    host_path(Path::new("/"), "/usr/local/libexec")
}

/// The oma lock file under the given root.
///
/// Termux uses `$PREFIX/var/run` because `/run` is not available there.
pub fn lock_file(rootfs: &Path) -> PathBuf {
    if is_termux() {
        rootfs.join("var/run/lock/oma.lock")
    } else {
        rootfs.join("run/lock/oma.lock")
    }
}

fn host_path_with(rootfs: &Path, host: &str, prefix: &Path, termux: bool) -> PathBuf {
    let host = Path::new(host);

    if termux {
        let rest = host
            .strip_prefix("/usr")
            .or_else(|_| host.strip_prefix("/"))
            .unwrap_or(host);
        prefix.join(rest)
    } else {
        rootfs.join(host.strip_prefix("/").unwrap_or(host))
    }
}

/// Resolve an absolute path from the host layout for `rootfs`.
///
/// On Termux the whole system lives under `$PREFIX`: `/usr/share/...` maps to
/// `$PREFIX/share/...`, while `/etc/...` and `/var/...` map to
/// `$PREFIX/etc/...` and `$PREFIX/var/...`. On other systems the host path is
/// resolved below `rootfs`.
pub fn host_path(rootfs: &Path, host: &str) -> PathBuf {
    host_path_with(rootfs, host, &prefix(), is_termux())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn data_dir_is_the_prefix_grandparent() {
        assert_eq!(
            data_dir_from(Path::new("/data/data/com.termux/files/usr")),
            Some(PathBuf::from("/data/data/com.termux"))
        );
        assert_eq!(data_dir_from(Path::new("/")), None);
    }

    #[test]
    fn host_paths_are_resolved_below_the_root() {
        assert_eq!(
            host_path_with(
                Path::new("/mnt"),
                "/usr/share/apt",
                Path::new("/prefix"),
                false
            ),
            PathBuf::from("/mnt/usr/share/apt")
        );
        assert_eq!(
            host_path_with(Path::new("/"), "/etc/apt", Path::new("/prefix"), false),
            PathBuf::from("/etc/apt")
        );
    }

    #[test]
    fn host_paths_are_resolved_below_the_prefix() {
        let prefix = Path::new("/data/data/com.termux/files/usr");

        assert_eq!(
            host_path_with(Path::new("/mnt"), "/usr/share/apt", prefix, true),
            prefix.join("share/apt")
        );
        assert_eq!(
            host_path_with(Path::new("/"), "/etc/apt", prefix, true),
            prefix.join("etc/apt")
        );
    }
}
