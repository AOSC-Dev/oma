use anyhow::{bail, Context, Result};
use rust_apt::util::DiskSpace;
use sysinfo::{DiskExt, System, SystemExt};

#[cfg(target_arch = "powerpc64")]
#[inline]
pub fn get_arch_name() -> Option<&'static str> {
    use nix::libc;
    let mut endian: libc::c_int = -1;
    let result;
    unsafe {
        result = libc::prctl(libc::PR_GET_ENDIAN, &mut endian as *mut libc::c_int);
    }
    if result < 0 {
        return None;
    }
    match endian {
        libc::PR_ENDIAN_LITTLE | libc::PR_ENDIAN_PPC_LITTLE => Some("ppc64el"),
        libc::PR_ENDIAN_BIG => Some("ppc64"),
        _ => None,
    }
}

/// AOSC OS specific architecture mapping table
#[cfg(not(target_arch = "powerpc64"))]
#[inline]
pub fn get_arch_name() -> Option<&'static str> {
    use std::env::consts::ARCH;
    match ARCH {
        "x86_64" => Some("amd64"),
        "x86" => Some("i486"),
        "powerpc" => Some("powerpc"),
        "aarch64" => Some("arm64"),
        "mips64" => Some("loongson3"),
        "riscv64" => Some("riscv64"),
        _ => None,
    }
}

pub fn size_checker(size: &DiskSpace, download_size: u64) -> Result<()> {
    let s = System::new_all();

    let disk = s
        .disks()
        .iter()
        .find(|x| x.mount_point().to_str() == Some("/"))
        .context("Can not get current disk!")?;

    let size = match size {
        DiskSpace::Require(v) => *v as i64,
        DiskSpace::Free(v) => 0 - *v as i64,
    };

    let need = size + download_size as i64;
    let avail = disk.available_space() as i64;

    if need > avail {
        bail!("Your disk space is too small, need size: {need}, available space: {avail}")
    }

    Ok(())
}
