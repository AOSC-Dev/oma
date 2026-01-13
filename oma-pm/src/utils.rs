use std::{fs, io, path::Path};

use spdlog::{debug, warn};

#[cfg(feature = "aosc")]
pub fn pkg_is_current_kernel(
    sysroot: &Path,
    image_name: &once_cell::sync::OnceCell<Option<String>>,
    pkg_name: &str,
    current_kernel_ver: &str,
) -> bool {
    fn is_installed_pkg_contains_file_impl(
        pkg_name: &str,
        image_names: &[impl AsRef<str>],
        sysroot: &Path,
    ) -> bool {
        is_installed_pkg_contains_file(pkg_name, image_names, sysroot).unwrap_or_else(|e| {
            warn!("Failed to get package {pkg_name} file list: {e}");
            false
        })
    }

    let image_name = image_name.get_or_init(get_kernel_image_filename);
    debug!("image name = {image_name:?}");

    if let Some(image_name) = image_name {
        is_installed_pkg_contains_file_impl(pkg_name, &[image_name], sysroot)
    } else {
        is_installed_pkg_contains_file_impl(
            pkg_name,
            &[
                format!("vmlinux-{current_kernel_ver}"),
                format!("vmlinuz-{current_kernel_ver}"),
            ],
            sysroot,
        )
    }
}

#[cfg(feature = "aosc")]
fn get_kernel_image_filename() -> Option<String> {
    let cmdline = fs::read_to_string("/proc/cmdline").ok()?;

    for i in cmdline.split_ascii_whitespace() {
        if let Some(image_path) = i.strip_prefix("BOOT_IMAGE=") {
            return Some(
                Path::new(image_path)
                    .file_name()
                    .map(|x| x.to_string_lossy().to_string())
                    .unwrap_or_else(|| image_path.to_string()),
            );
        }
    }

    None
}

pub fn is_installed_pkg_contains_file(
    pkg_name: &str,
    file_names: &[impl AsRef<str>],
    sysroot: &Path,
) -> io::Result<bool> {
    let p = sysroot.join(format!("var/lib/dpkg/info/{pkg_name}.list"));
    let file = fs::read_to_string(&p)?;

    for line in file.lines() {
        if Path::new(line).file_name().is_some_and(|n| {
            file_names
                .iter()
                .any(|name| name.as_ref() == n.to_string_lossy())
        }) {
            return Ok(true);
        }
    }

    Ok(false)
}
